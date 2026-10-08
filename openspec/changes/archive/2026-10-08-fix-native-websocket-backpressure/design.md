## Design
Match the existing helper event queue budget for websocket messages: 4096 messages and 32 MiB charged payload. Charge interpreted JSON twice for retained text and decoded payload; counts bound object overhead. Yield every 32 pump events without waiting for the application consumer, so acknowledgements cannot deadlock behind a full message queue. Reject projected bytes beyond budget, including a single oversized message. Preserve pre-clear diagnostics and distinguish count/byte rejection. The dedicated native_websocket_backpressure code is account-neutral, without declaring host networking unavailable.

## Limits
Payload accounting is not a heap-size measurement. Aggregate memory still scales with active sockets; admission settings independently limit concurrency. No payload is logged.
