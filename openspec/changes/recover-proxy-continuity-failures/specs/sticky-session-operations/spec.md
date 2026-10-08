## ADDED Requirements

### Requirement: Submit-time abandoned proxy anchors retire without upstream denial provenance
When submission refuses a proxy-injected anchor because an abandonment tombstone or poisoned-anchor quarantine was adopted after planning, the bridge MUST retire only that exact anchor from its current local carrier and its matching durable owner epoch. The refusal MUST preserve the existing public error and MUST NOT publish upstream-denial provenance for a request never dispatched. Retirement MUST preserve newer anchors, newer owners, explicit client anchors, and retry-circuit and quarantine generations. A following portable full-history request MUST be able to establish replacement continuity without reusing or trimming against the retired anchor. Delta-only requests MUST remain fail-closed while the abandonment tombstone applies.

#### Scenario: Tombstone adopted between planning and submission
- **WHEN** an originally unanchored full-history request receives a proxy-injected anchor and submission adopts an abandonment tombstone
- **THEN** the bridge returns its existing 404 without dispatching that anchor
- **AND** retires the exact matching local and durable anchor without marking it upstream-denied
- **AND** the next full-history request can complete unanchored with all supplied input

#### Scenario: Concurrent successor wins
- **WHEN** an owner epoch or latest anchor advances before retirement completes
- **THEN** retirement does not remove the successor's durable or local continuity

#### Scenario: Client anchor and incomplete context remain protected
- **WHEN** the client supplies an explicit anchor or only a delta while abandonment applies
- **THEN** the bridge does not silently replay the delta as a fresh conversation
