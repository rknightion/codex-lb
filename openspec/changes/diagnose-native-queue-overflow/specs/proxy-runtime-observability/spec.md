## ADDED Requirements

### Requirement: Native queue overflow evidence
Native websocket terminal diagnostics for consumer backpressure SHALL identify websocket_messages or helper_events and capture pre-clear count, charged bytes, configured limits, rejected/projected bytes and owning-generation active stream count. An absent byte limit SHALL mean no enforced byte bound. Diagnostics SHALL remain content-free and preserve queue acceptance, sizing, cancellation and public errors.

#### Scenario: Websocket message count overflow
- **WHEN** the per-websocket message queue rejects another message
- **THEN** the correlated terminal log identifies websocket_messages and reports the pre-clear count and count limit with no byte limit

#### Scenario: Helper event overflow
- **WHEN** the helper reader cannot enqueue an event for a request
- **THEN** the correlated terminal log identifies helper_events and reports both configured limits and the pre-clear charged-byte count

### Requirement: Native queue consumer timing
Overflow snapshots SHALL report enqueue/dequeue counts, oldest queued age and time since the last consumer dequeue using monotonic time. Never-consumed queues SHALL report an absent dequeue gap. Cleanup SHALL NOT overwrite captured timing.

#### Scenario: Recent consumer activity
- **WHEN** a consumer has dequeued an item before overflow
- **THEN** the snapshot reports the monotonic elapsed dequeue time and backlog age at overflow, independent of later cleanup

### Requirement: Bounded native queue overflow metrics
Queue overflows SHALL emit codex_lb_ Prometheus counters and snapshot distributions labelled only by closed queue and limit enums. Measurements SHALL describe overflow observations rather than live queue gauges. Unknown optional timing SHALL be omitted, never replaced by zero. Identifiers and payload content SHALL NOT become labels. Metric failures SHALL NOT replace transport outcomes.

#### Scenario: Count or byte bound rejects an item
- **WHEN** a queue rejects an item
- **THEN** metrics identify the first rejecting bound and report pre-clear queue count, payload bytes, available consumer timing and owning-generation stream count

### Requirement: Diagnostic dashboard coverage
The m7kni troubleshooting dashboard SHALL cover every emitted codex_lb_ metric family and provide log and trace drilldowns. Its coverage inventory SHALL distinguish missing panels from queries with no current series.

#### Scenario: Queue overflow has no recent samples
- **WHEN** a queue metric has no series in the selected window
- **THEN** the dashboard inventory still records its panel coverage separately from data availability
