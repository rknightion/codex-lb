## ADDED Requirements

### Requirement: Bounded native websocket burst tolerance
Native websocket message buffering SHALL accept short bursts within a 4096-message and 32 MiB charged-payload bound. The pump SHALL yield cooperatively without awaiting consumer capacity. Overflow SHALL preserve content-free queue evidence and SHALL NOT penalize upstream account health or mark host networking unavailable.

#### Scenario: Active consumer receives a small burst
- **WHEN** hundreds of small websocket messages arrive before the next consumer scheduling turn
- **THEN** the messages are delivered in order without a spurious transport failure

#### Scenario: Stalled consumer exceeds budget
- **WHEN** buffered websocket messages exceed either bounded capacity
- **THEN** the socket fails locally, reports the rejecting bound, and settles without blaming its upstream account

## MODIFIED Requirements

### Requirement: Native queue overflow evidence
Native websocket terminal diagnostics for consumer backpressure SHALL identify websocket_messages or helper_events and capture pre-clear count, charged bytes, configured limits, rejected/projected bytes and owning-generation active stream count. An absent byte limit SHALL mean no enforced byte bound. Diagnostics SHALL remain content-free. Proven local queue overflow SHALL use native_websocket_backpressure transport provenance, remain retryable, and preserve cancellation and settlement.

#### Scenario: Websocket message count overflow
- **WHEN** the per-websocket message queue rejects another message
- **THEN** the correlated terminal log identifies websocket_messages and reports pre-clear count, charged bytes and both configured limits

#### Scenario: Helper event overflow
- **WHEN** the helper reader cannot enqueue an event for a request
- **THEN** the correlated terminal log identifies helper_events and reports both configured limits and the pre-clear charged-byte count
