use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use codex_lb_protocol::{NativeEvent, NativeWebSocketDiagnostic, NativeWebSocketRequest};
use codex_lb_responses::stream::interpret_websocket;
use futures_util::{SinkExt, StreamExt};
use rustls::{ClientConfig, RootCertStore};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::Connector;
use tokio_tungstenite::client_async_tls_with_config;
use tokio_tungstenite::proxy::connect_via_proxy;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::error::{ProtocolError, TlsError};
use tokio_tungstenite::tungstenite::handshake::client::Response as WebSocketResponse;
use tokio_tungstenite::tungstenite::http::HeaderName as WebSocketHeaderName;
use tokio_tungstenite::tungstenite::http::HeaderValue as WebSocketHeaderValue;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::proxy::ProxyConfig;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tungstenite::Bytes;
use tungstenite::extensions::ExtensionsConfig;
use tungstenite::extensions::compression::deflate::DeflateConfig;
use url::Url;

use crate::http::error_chain_has_invalid_certificate;
use crate::runtime::{Output, emit};

pub(crate) trait AsyncIo: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T> AsyncIo for T where T: AsyncRead + AsyncWrite + Send + Unpin {}

pub(crate) type NativeWebSocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<Box<dyn AsyncIo>>>;

pub(crate) enum WebSocketCommand {
    Send {
        command_id: String,
        message: Message,
    },
    Close {
        command_id: String,
        code: u16,
        reason: String,
    },
}

#[derive(Debug)]
pub(crate) enum NativeWebSocketFailure {
    Connect(WebSocketError),
    WebSocket(WebSocketError, &'static str),
    Timeout,
    LivenessTimeout,
    Output(std::io::Error),
}

impl std::fmt::Display for NativeWebSocketFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(error) => write!(formatter, "{error}"),
            Self::WebSocket(error, _) => write!(formatter, "{error}"),
            Self::Timeout => formatter.write_str("websocket connect timed out"),
            Self::LivenessTimeout => formatter.write_str("websocket pong timed out"),
            Self::Output(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for NativeWebSocketFailure {}

impl From<WebSocketError> for NativeWebSocketFailure {
    fn from(error: WebSocketError) -> Self {
        Self::WebSocket(error, "receive")
    }
}

impl From<std::io::Error> for NativeWebSocketFailure {
    fn from(error: std::io::Error) -> Self {
        Self::Output(error)
    }
}

pub(crate) async fn execute_websocket(
    request: NativeWebSocketRequest,
    mut commands: mpsc::Receiver<WebSocketCommand>,
    output: &Output,
) -> Result<(), NativeWebSocketFailure> {
    let request_id = request.request_id.clone();
    let connect_timeout = Duration::from_millis(request.connect_timeout_ms);
    let connected = tokio::time::timeout(connect_timeout, connect_native_websocket(&request))
        .await
        .map_err(|_| NativeWebSocketFailure::Timeout)?
        .map_err(NativeWebSocketFailure::Connect)?;
    let (mut websocket, response) = connected;
    let response_headers = websocket_headers(response.headers());
    emit(
        output,
        &NativeEvent::WebsocketOpen {
            request_id: request_id.clone(),
            status: response.status().as_u16(),
            headers: response_headers,
        },
    )
    .await?;

    let ping_interval = request.ping_interval_ms.map(Duration::from_millis);
    let ping_timeout = request.ping_timeout_ms.map(Duration::from_millis);
    let dormant_timer = Duration::from_secs(100 * 365 * 24 * 60 * 60);
    let mut next_ping = Box::pin(tokio::time::sleep(ping_interval.unwrap_or(dormant_timer)));
    let mut pong_deadline = Box::pin(tokio::time::sleep(dormant_timer));
    let mut ping_sequence = 0_u64;
    let mut awaiting_pong = None;

    loop {
        tokio::select! {
            _ = &mut next_ping, if ping_interval.is_some() => {
                let interval = ping_interval.expect("guarded ping interval");
                next_ping.as_mut().reset(tokio::time::Instant::now() + interval);
                if awaiting_pong.is_none() {
                    ping_sequence = ping_sequence.wrapping_add(1);
                    let payload = Bytes::copy_from_slice(&ping_sequence.to_be_bytes());
                    websocket.send(Message::Ping(payload.clone())).await
                        .map_err(|error| NativeWebSocketFailure::WebSocket(error, "ping"))?;
                    if let Some(timeout) = ping_timeout {
                        awaiting_pong = Some(payload);
                        pong_deadline.as_mut().reset(tokio::time::Instant::now() + timeout);
                    }
                }
            }
            _ = &mut pong_deadline, if awaiting_pong.is_some() => {
                return Err(NativeWebSocketFailure::LivenessTimeout);
            }
            command = commands.recv() => {
                let Some(command) = command else {
                    return Ok(());
                };
                match command {
                    WebSocketCommand::Send { command_id, message } => {
                        match websocket.send(message).await {
                            Ok(()) => {
                                emit(
                                    output,
                                    &NativeEvent::WebsocketSent {
                                        request_id: request_id.clone(),
                                        command_id,
                                    },
                                ).await?;
                            }
                            Err(error) => {
                                emit_websocket_error(
                                    output,
                                    &request_id,
                                    Some(command_id),
                                    &NativeWebSocketFailure::WebSocket(error, "send"),
                                ).await?;
                                return Ok(());
                            }
                        }
                    }
                    WebSocketCommand::Close { command_id, code, reason } => {
                        let frame = CloseFrame {
                            code: CloseCode::from(code),
                            reason: reason.clone().into(),
                        };
                        match websocket.send(Message::Close(Some(frame))).await {
                            Ok(()) => {
                                emit(
                                    output,
                                    &NativeEvent::WebsocketSent {
                                        request_id: request_id.clone(),
                                        command_id,
                                    },
                                ).await?;
                                emit(
                                    output,
                                    &NativeEvent::WebsocketClose {
                                        request_id,
                                        code: Some(code),
                                        reason: (!reason.is_empty()).then_some(reason),
                                    },
                                ).await?;
                                return Ok(());
                            }
                            Err(error) => {
                                emit_websocket_error(
                                    output,
                                    &request_id,
                                    Some(command_id),
                                    &NativeWebSocketFailure::WebSocket(error, "close"),
                                ).await?;
                                return Ok(());
                            }
                        }
                    }
                }
            }
            incoming = websocket.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let text = text.to_string();
                        let event = request
                            .interpret_responses
                            .then(|| interpret_websocket(&text))
                            .flatten();
                        if let Some(event) = event {
                            emit(
                                output,
                                &NativeEvent::WebsocketResponsesText {
                                    request_id: request_id.clone(),
                                    text,
                                    event_type: event.event_type,
                                    payload: event.payload,
                                    payload_response_id: event.payload_response_id,
                                    sequence_number: event.sequence_number,
                                },
                            )
                            .await?;
                        } else {
                            emit(
                                output,
                                &NativeEvent::WebsocketText {
                                    request_id: request_id.clone(),
                                    text,
                                },
                            )
                            .await?;
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        emit(
                            output,
                            &NativeEvent::WebsocketBinary {
                                request_id: request_id.clone(),
                                data: base64::engine::general_purpose::STANDARD.encode(data),
                            },
                        ).await?;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if let Err(error) = websocket.send(Message::Pong(payload)).await {
                            return Err(NativeWebSocketFailure::WebSocket(error, "pong"));
                        }
                    }
                    Some(Ok(Message::Pong(payload))) => {
                        if awaiting_pong.as_ref().is_some_and(|expected| expected == &payload) {
                            awaiting_pong = None;
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let (code, reason) = frame
                            .map(|frame| {
                                (
                                    Some(u16::from(frame.code)),
                                    (!frame.reason.is_empty()).then(|| frame.reason.to_string()),
                                )
                            })
                            .unwrap_or((None, None));
                        emit(
                            output,
                            &NativeEvent::WebsocketClose { request_id, code, reason },
                        ).await?;
                        return Ok(());
                    }
                    Some(Ok(Message::Frame(_))) => {}
                    Some(Err(error)) => return Err(error.into()),
                    None => {
                        emit(
                            output,
                            &NativeEvent::WebsocketClose {
                                request_id,
                                code: None,
                                reason: None,
                            },
                        ).await?;
                        return Ok(());
                    }
                }
            }
        }
    }
}

pub(crate) async fn connect_native_websocket(
    native_request: &NativeWebSocketRequest,
) -> Result<(NativeWebSocket, WebSocketResponse), WebSocketError> {
    let mut request = native_request.url.as_str().into_client_request()?;
    for (name, value) in &native_request.headers {
        request.headers_mut().append(
            WebSocketHeaderName::from_bytes(name.as_bytes())?,
            WebSocketHeaderValue::from_str(value)?,
        );
    }

    let host = request
        .uri()
        .host()
        .ok_or(tokio_tungstenite::tungstenite::error::UrlError::NoHostName)?
        .to_owned();
    let port = request
        .uri()
        .port_u16()
        .or_else(|| match request.uri().scheme_str() {
            Some("ws") => Some(80),
            Some("wss") => Some(443),
            _ => None,
        })
        .ok_or(tokio_tungstenite::tungstenite::error::UrlError::UnsupportedUrlScheme)?;

    let tls_config = native_tls_config()?;
    let stream: Box<dyn AsyncIo> = match native_request.proxy_url.as_deref() {
        None => Box::new(TcpStream::connect(host_port(&host, port)).await?),
        Some(proxy_url) => {
            let proxy = ProxyEndpoint::parse(proxy_url)?;
            let stream = TcpStream::connect(proxy.config.authority()).await?;
            let stream: Box<dyn AsyncIo> = if proxy.tls {
                let server_name = rustls::pki_types::ServerName::try_from(
                    proxy.config.host.clone(),
                )
                .map_err(|_| tokio_tungstenite::tungstenite::error::TlsError::InvalidDnsName)?;
                Box::new(
                    TlsConnector::from(tls_config.clone())
                        .connect(server_name, stream)
                        .await
                        .map_err(WebSocketError::Io)?,
                )
            } else {
                Box::new(stream)
            };
            Box::new(connect_via_proxy(stream, &proxy.config, &host, port).await?)
        }
    };

    client_async_tls_with_config(
        request,
        stream,
        Some(native_websocket_config(native_request.max_message_bytes)),
        Some(Connector::Rustls(tls_config)),
    )
    .await
}

pub(crate) fn native_websocket_config(max_message_bytes: usize) -> WebSocketConfig {
    let mut extensions = ExtensionsConfig::default();
    extensions.permessage_deflate = Some(DeflateConfig::default());
    let mut config = WebSocketConfig::default();
    config.max_message_size = Some(max_message_bytes);
    config.extensions = extensions;
    config
}

fn native_tls_config() -> Result<Arc<ClientConfig>, WebSocketError> {
    let certificates = rustls_native_certs::load_native_certs();
    let mut roots = RootCertStore::empty();
    roots.add_parsable_certificates(certificates.certs);
    if roots.is_empty() {
        return Err(WebSocketError::Io(std::io::Error::other(
            "native certificate store is empty",
        )));
    }
    Ok(Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    ))
}

#[derive(Debug)]
struct ProxyEndpoint {
    config: ProxyConfig,
    tls: bool,
}

impl ProxyEndpoint {
    fn parse(value: &str) -> Result<Self, WebSocketError> {
        let mut url = Url::parse(value).map_err(|_| invalid_proxy_config())?;
        let tls = url.scheme() == "https";
        if tls {
            let port = url
                .port_or_known_default()
                .ok_or_else(invalid_proxy_config)?;
            url.set_scheme("http").map_err(|_| invalid_proxy_config())?;
            url.set_port(Some(port))
                .map_err(|_| invalid_proxy_config())?;
        }
        let config = ProxyConfig::parse(url.as_str()).map_err(|_| invalid_proxy_config())?;
        Ok(Self { config, tls })
    }
}

fn invalid_proxy_config() -> WebSocketError {
    WebSocketError::Url(
        tokio_tungstenite::tungstenite::error::UrlError::InvalidProxyConfig(
            "<redacted>".to_owned(),
        ),
    )
}

fn host_port(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn websocket_headers(
    headers: &tokio_tungstenite::tungstenite::http::HeaderMap,
) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect()
}

pub(crate) async fn emit_websocket_setup_error(
    output: &Output,
    request_id: &str,
    command_id: Option<String>,
    message: &str,
) -> Result<(), std::io::Error> {
    emit(
        output,
        &NativeEvent::WebsocketError {
            request_id: request_id.to_owned(),
            command_id,
            message: message.to_owned(),
            failure_phase: "setup".to_owned(),
            retryable_same_contract: false,
            is_tls_verification_failure: false,
            status: None,
            headers: Vec::new(),
            body: None,
            diagnostic: None,
        },
    )
    .await
}

pub(crate) async fn emit_websocket_error(
    output: &Output,
    request_id: &str,
    command_id: Option<String>,
    failure: &NativeWebSocketFailure,
) -> Result<(), std::io::Error> {
    let (message, phase, retryable, tls_verification, status, headers, body) = match failure {
        NativeWebSocketFailure::Timeout => (
            "native websocket connection timed out",
            "connect",
            true,
            false,
            None,
            Vec::new(),
            None,
        ),
        NativeWebSocketFailure::LivenessTimeout => (
            "native websocket pong timed out",
            "liveness_timeout",
            false,
            false,
            None,
            Vec::new(),
            None,
        ),
        NativeWebSocketFailure::Connect(WebSocketError::Http(response)) => (
            "native websocket handshake failed",
            "connect",
            false,
            false,
            Some(response.status().as_u16()),
            websocket_headers(response.headers()),
            response
                .body()
                .as_ref()
                .map(|body| base64::engine::general_purpose::STANDARD.encode(body)),
        ),
        NativeWebSocketFailure::Connect(WebSocketError::Io(_)) => {
            let tls_verification = websocket_tls_verification_failure(failure);
            (
                "native websocket transport failed",
                "connect",
                !tls_verification,
                tls_verification,
                None,
                Vec::new(),
                None,
            )
        }
        NativeWebSocketFailure::Connect(_) => {
            let tls_verification = websocket_tls_verification_failure(failure);
            (
                "native websocket connection failed",
                "connect",
                !tls_verification,
                tls_verification,
                None,
                Vec::new(),
                None,
            )
        }
        NativeWebSocketFailure::WebSocket(WebSocketError::Io(_), _) => (
            "native websocket transport failed",
            "transport",
            false,
            websocket_tls_verification_failure(failure),
            None,
            Vec::new(),
            None,
        ),
        NativeWebSocketFailure::WebSocket(_, _) => (
            "native websocket protocol failed",
            "protocol",
            false,
            websocket_tls_verification_failure(failure),
            None,
            Vec::new(),
            None,
        ),
        NativeWebSocketFailure::Output(_) => (
            "native helper output failed",
            "helper_write",
            false,
            false,
            None,
            Vec::new(),
            None,
        ),
    };
    emit(
        output,
        &NativeEvent::WebsocketError {
            request_id: request_id.to_owned(),
            command_id,
            message: message.to_owned(),
            failure_phase: phase.to_owned(),
            retryable_same_contract: retryable,
            is_tls_verification_failure: tls_verification,
            status,
            headers,
            body,
            diagnostic: Some(websocket_diagnostic(failure)),
        },
    )
    .await
}

fn websocket_tls_verification_failure(failure: &NativeWebSocketFailure) -> bool {
    match failure {
        NativeWebSocketFailure::Connect(WebSocketError::Tls(TlsError::Rustls(error)))
        | NativeWebSocketFailure::WebSocket(WebSocketError::Tls(TlsError::Rustls(error)), _) => {
            matches!(error.as_ref(), rustls::Error::InvalidCertificate(_))
        }
        NativeWebSocketFailure::Connect(error) | NativeWebSocketFailure::WebSocket(error, _) => {
            error_chain_has_invalid_certificate(error)
        }
        _ => false,
    }
}

fn websocket_diagnostic(failure: &NativeWebSocketFailure) -> NativeWebSocketDiagnostic {
    let mut diagnostic = NativeWebSocketDiagnostic {
        category: "unknown".into(),
        operation: "unknown".into(),
        protocol: None,
        io_kind: None,
        os_code: None,
        tls: None,
    };
    let error = match failure {
        NativeWebSocketFailure::Connect(error) => {
            diagnostic.operation = "connect".into();
            error
        }
        NativeWebSocketFailure::WebSocket(error, operation) => {
            diagnostic.operation = (*operation).into();
            error
        }
        NativeWebSocketFailure::Timeout => {
            diagnostic.category = "timeout".into();
            diagnostic.operation = "connect".into();
            return diagnostic;
        }
        NativeWebSocketFailure::LivenessTimeout => {
            diagnostic.category = "liveness_timeout".into();
            diagnostic.operation = "pong_wait".into();
            return diagnostic;
        }
        NativeWebSocketFailure::Output(error) => {
            diagnostic.operation = "helper_write".into();
            io_diagnostic(error, &mut diagnostic);
            return diagnostic;
        }
    };
    diagnostic.category = match error {
        WebSocketError::ConnectionClosed => "connection_closed",
        WebSocketError::AlreadyClosed => "already_closed",
        WebSocketError::Io(error) => {
            io_diagnostic(error, &mut diagnostic);
            "io"
        }
        WebSocketError::Tls(error) => {
            diagnostic.tls = Some(
                match error {
                    TlsError::InvalidDnsName => "invalid_dns_name",
                    TlsError::Rustls(error) => match error.as_ref() {
                        rustls::Error::InvalidCertificate(_) => "invalid_certificate",
                        rustls::Error::AlertReceived(_) => "alert_received",
                        _ => "rustls_other",
                    },
                    _ => "other",
                }
                .into(),
            );
            "tls"
        }
        WebSocketError::Protocol(error) => {
            diagnostic.protocol = Some(protocol_category(error).into());
            "protocol"
        }
        WebSocketError::Capacity(_) => "capacity",
        WebSocketError::WriteBufferFull(_) => "write_buffer_full",
        WebSocketError::Utf8(_) => "utf8",
        WebSocketError::AttackAttempt => "attack_attempt",
        WebSocketError::Url(_) => "url",
        WebSocketError::Http(_) => "http",
        WebSocketError::HttpFormat(_) => "http_format",
    }
    .into();
    if websocket_tls_verification_failure(failure) {
        diagnostic.tls = Some("invalid_certificate".into());
    }
    diagnostic
}

fn io_diagnostic(error: &std::io::Error, diagnostic: &mut NativeWebSocketDiagnostic) {
    use std::io::ErrorKind;
    diagnostic.category = "io".into();
    diagnostic.os_code = error.raw_os_error();
    diagnostic.io_kind = Some(
        match error.kind() {
            ErrorKind::ConnectionReset => "connection_reset",
            ErrorKind::ConnectionAborted => "connection_aborted",
            ErrorKind::ConnectionRefused => "connection_refused",
            ErrorKind::BrokenPipe => "broken_pipe",
            ErrorKind::NotConnected => "not_connected",
            ErrorKind::TimedOut => "timed_out",
            ErrorKind::UnexpectedEof => "unexpected_eof",
            ErrorKind::WouldBlock => "would_block",
            ErrorKind::Interrupted => "interrupted",
            ErrorKind::InvalidData => "invalid_data",
            ErrorKind::PermissionDenied => "permission_denied",
            ErrorKind::Other => "other",
            _ => "unclassified",
        }
        .into(),
    );
    // rustls read failures may be nested in io::Error; never stringify them.
    if let Some(tls) = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<rustls::Error>())
    {
        diagnostic.tls = Some(
            match tls {
                rustls::Error::InvalidCertificate(_) => "invalid_certificate",
                rustls::Error::AlertReceived(_) => "alert_received",
                _ => "rustls_other",
            }
            .into(),
        );
    }
}

fn protocol_category(error: &ProtocolError) -> &'static str {
    match error {
        ProtocolError::WrongHttpMethod => "WrongHttpMethod",
        ProtocolError::WrongHttpVersion => "WrongHttpVersion",
        ProtocolError::MissingConnectionUpgradeHeader => "MissingConnectionUpgradeHeader",
        ProtocolError::MissingUpgradeWebSocketHeader => "MissingUpgradeWebSocketHeader",
        ProtocolError::MissingSecWebSocketVersionHeader => "MissingSecWebSocketVersionHeader",
        ProtocolError::MissingSecWebSocketKey => "MissingSecWebSocketKey",
        ProtocolError::SecWebSocketAcceptKeyMismatch => "SecWebSocketAcceptKeyMismatch",
        ProtocolError::SecWebSocketSubProtocolError(_) => "SecWebSocketSubProtocolError",
        ProtocolError::InvalidExtensionsHeader(_) => "InvalidExtensionsHeader",
        ProtocolError::JunkAfterRequest => "JunkAfterRequest",
        ProtocolError::CustomResponseSuccessful => "CustomResponseSuccessful",
        ProtocolError::InvalidHeader(_) => "InvalidHeader",
        ProtocolError::HandshakeIncomplete => "HandshakeIncomplete",
        ProtocolError::HttparseError(_) => "HttparseError",
        ProtocolError::SendAfterClosing => "SendAfterClosing",
        ProtocolError::ReceivedAfterClosing => "ReceivedAfterClosing",
        ProtocolError::NonZeroReservedBits => "NonZeroReservedBits",
        ProtocolError::UnmaskedFrameFromClient => "UnmaskedFrameFromClient",
        ProtocolError::MaskedFrameFromServer => "MaskedFrameFromServer",
        ProtocolError::FragmentedControlFrame => "FragmentedControlFrame",
        ProtocolError::CompressedControlFrame => "CompressedControlFrame",
        ProtocolError::ControlFrameTooBig => "ControlFrameTooBig",
        ProtocolError::UnknownControlFrameType(_) => "UnknownControlFrameType",
        ProtocolError::UnknownDataFrameType(_) => "UnknownDataFrameType",
        ProtocolError::UnexpectedContinueFrame => "UnexpectedContinueFrame",
        ProtocolError::CompressedContinueFrame => "CompressedContinueFrame",
        ProtocolError::ExpectedFragment(_) => "ExpectedFragment",
        ProtocolError::ResetWithoutClosingHandshake => "ResetWithoutClosingHandshake",
        ProtocolError::InvalidOpcode(_) => "InvalidOpcode",
        ProtocolError::InvalidCloseSequence => "InvalidCloseSequence",
        ProtocolError::CompressionFailure(_) => "CompressionFailure",
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn diagnostic_never_serializes_error_content() {
        let failures = [
            NativeWebSocketFailure::WebSocket(
                WebSocketError::Utf8("private-payload".into()),
                "receive",
            ),
            NativeWebSocketFailure::WebSocket(
                WebSocketError::Io(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    "private-peer",
                )),
                "receive",
            ),
            NativeWebSocketFailure::Connect(WebSocketError::Protocol(
                ProtocolError::InvalidHeader(Box::new(WebSocketHeaderName::from_static(
                    "private-header",
                ))),
            )),
        ];
        for failure in failures {
            let value = serde_json::to_string(&websocket_diagnostic(&failure)).unwrap();
            assert!(!value.contains("private-"));
        }
        let reset = websocket_diagnostic(&NativeWebSocketFailure::WebSocket(
            WebSocketError::Protocol(ProtocolError::ResetWithoutClosingHandshake),
            "receive",
        ));
        assert_eq!(reset.category, "protocol");
        assert_eq!(
            reset.protocol.as_deref(),
            Some("ResetWithoutClosingHandshake")
        );
        let io = std::io::Error::from_raw_os_error(104);
        let expected_kind = io.kind();
        let diagnostic = websocket_diagnostic(&NativeWebSocketFailure::WebSocket(
            WebSocketError::Io(io),
            "receive",
        ));
        assert_eq!(diagnostic.os_code, Some(104));
        if expected_kind == std::io::ErrorKind::ConnectionReset {
            assert_eq!(diagnostic.io_kind.as_deref(), Some("connection_reset"));
        }
    }
}
