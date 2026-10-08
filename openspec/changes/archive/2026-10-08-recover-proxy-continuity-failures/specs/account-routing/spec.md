## ADDED Requirements

### Requirement: Hard continuity owners remain eligible through transient error backoff

When selection has narrowed the candidate pool to one resolved hard continuity owner, the selector MUST permit that owner through replica-local transient error-count backoff if no normally available candidate remains. This exception MUST NOT clear error counters or timestamps, broaden ownership, or bypass persisted quota and rate-limit holds, unexpired hard cooldowns, known credential expiry, security and account scope, caller exclusions, or concurrency admission. Ordinary unbound and soft-affinity selection MUST retain existing backoff behavior. Runtime version fences and lease settlement MUST remain effective for these admissions.

#### Scenario: Required owner serves despite transient backoff
- **GIVEN** an otherwise eligible account owns the request's continuity and is in transient error-count backoff
- **WHEN** required-owner selection runs through either the sticky or unbound path
- **THEN** it selects that owner without selecting a sibling or clearing failure observations
- **AND** its acquired lease is released through normal settlement

#### Scenario: Hard sticky owner serves despite transient backoff
- **GIVEN** a hard sticky mapping names an otherwise eligible owner in transient error-count backoff
- **WHEN** selection resolves the mapping
- **THEN** it selects the owner and preserves the mapping

#### Scenario: Hard unavailability still blocks the owner
- **GIVEN** a required owner has exhausted quota, a future rate-limit deadline, a hard cooldown, unavailable credentials, a policy exclusion, or exhausted local admission capacity
- **WHEN** selection runs while that owner also has transient error-count backoff
- **THEN** selection fails closed without admitting it or moving to a sibling

#### Scenario: Ordinary routing keeps healthy sibling preference
- **GIVEN** a movable request can use a healthy sibling of an account in transient backoff
- **WHEN** ordinary selection runs
- **THEN** the healthy sibling remains preferred
- **AND** a lone ordinary candidate receives no new exemption from transient backoff
