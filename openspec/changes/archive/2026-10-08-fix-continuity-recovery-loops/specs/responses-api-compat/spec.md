## ADDED Requirements

### Requirement: Long-unavailable HTTP continuity can restart portable input
When an HTTP bridge request fails before dispatch because its continuity owner is unavailable, the service MUST prefer verified full-history account-neutral replay. If that proof fails, an originally unanchored request with a durable owner that cannot recover within the request budget MAY restart its current portable input as a new upstream conversation. The service MUST verify account-neutral payload shape, exclude the former account, strip stale upstream affinity headers, and atomically retire only the matched durable owner before dispatch. It MUST NOT drop an explicit client previous-response anchor or transfer account-bound file state without the existing full-replay proof. Such unrecoverable requests MUST return HTTP 400 `continuity_recovery_required` with type `invalid_request_error`. Missing required ownership MUST likewise fail explicitly. Short or unknown reset times and transient capacity misses MUST retain their retryable contract.

Any retry-circuit reset after retirement MUST match the episode epoch, count and admission generation captured before retirement. Poison quarantine removal MUST match its captured generation. A newer episode or sibling alias MUST NOT be cleared. Verified recovery aliases MUST retain their existing task-specific precedence.

#### Scenario: Portable incremental turn restarts after quota exhaustion
- **GIVEN** the client supplies fresh portable input without an explicit previous-response anchor
- **AND** its durable owner has exhausted quota beyond the request deadline
- **WHEN** safe complete-history replay is unavailable
- **THEN** the proxy retires the matched owner and sends the current input without its injected anchor to a different account
- **AND** a matching obsolete retry circuit does not block that new conversation

#### Scenario: Explicit anchor requires full replay proof
- **GIVEN** a client supplies an explicit previous-response anchor with incomplete history
- **WHEN** its owner cannot recover within the request budget
- **THEN** the proxy returns 400 `continuity_recovery_required` without discarding that anchor

#### Scenario: Temporary owner failure remains retryable
- **GIVEN** the owner has a short or unknown recovery horizon
- **WHEN** it is temporarily unavailable
- **THEN** the proxy keeps its owner and returns the existing retryable error

#### Scenario: Concurrent replacement circuit is preserved
- **GIVEN** a new circuit episode or admission claim advances after the retirement snapshot
- **WHEN** retirement settles the old circuit
- **THEN** the new episode or claim remains unchanged
