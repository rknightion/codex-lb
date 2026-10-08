## ADDED Requirements

### Requirement: Content-free native websocket failure diagnostics
The system SHALL preserve structured native websocket failure evidence across the native helper boundary and emit it when an established native websocket operation fails, without changing public errors, routing, retries, settlement or cancellation behavior. Diagnostics SHALL identify the failure phase and bounded error category, carry numeric OS error evidence when available, and provide connection correlation and monotonic timing/counters where measured. Unknown evidence SHALL remain absent or explicitly unknown. Diagnostics MUST NOT contain request/response content, raw exception strings, headers, URLs, credentials, or raw peer close reasons.

#### Scenario: Abrupt upstream disconnect
- **WHEN** a local websocket peer disconnects without completing the websocket close handshake
- **THEN** the real native helper and Python adapter emit a structured diagnostic distinguishing the protocol or I/O failure with connection correlation
- **AND** the existing generic public error remains unchanged

#### Scenario: Native failure and malformed diagnostic privacy
- **WHEN** a native failure contains sensitive text or unknown diagnostic values
- **THEN** logs contain only allowlisted diagnostic categories and validated numeric values
- **AND** raw error messages, headers and bodies are excluded

#### Scenario: Helper and liveness failures
- **WHEN** the native helper exits or a websocket liveness deadline expires
- **THEN** the diagnostic identifies the observed failure phase without guessing a network cause
- **AND** existing error classification and cancellation behavior remain unchanged

#### Scenario: Peer close and correlation
- **WHEN** a peer sends a close frame
- **THEN** diagnostics preserve the numeric close code without the reason text and identify the peer close separately from local cleanup
- **AND** native connection diagnostics can be joined to the bridge failure using a shared connection identifier or demonstrated correlation
- **AND** measured application-message progress is distinguished from unmeasured control-frame activity

#### Scenario: Backward compatible diagnostics
- **WHEN** an older helper omits diagnostics or sends an unrecognized category
- **THEN** diagnostic parsing preserves existing error handling and records missing evidence as unknown
- **AND** helper EOF or exit evidence is distinguished from a later cleanup termination

#### Scenario: Diagnostic sink failure
- **WHEN** a diagnostic logging handler raises an exception
- **THEN** the existing websocket send exception or receive result remains unchanged
- **AND** the diagnostic path does not catch task cancellation
