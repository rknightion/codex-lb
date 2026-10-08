## ADDED Requirements

### Requirement: Exhausted usage reconciles blocked account reset
A successful usage refresh for an already rate-limited or quota-exceeded account MUST reconcile its reset to the latest future exhausted standard quota window when no usable credits are reported. The update MUST compare the observed status markers before writing. Exhausted long windows MUST use quota-exceeded recovery semantics; short-window-only exhaustion MUST retain rate-limited semantics. A later actual rate-limit deadline MUST NOT be shortened by quota evidence. Additional model quotas MUST NOT extend the global account deadline. Transient error writes MUST NOT shorten a still-valid blocked deadline or overwrite a concurrently changed account status.

#### Scenario: Weekly exhaustion outlasts a burst cooldown
- **GIVEN** a rate-limited account with a short cooldown
- **WHEN** fresh standard usage reports an exhausted weekly window resetting next week and no usable credits
- **THEN** its persisted reset reflects the weekly deadline
- **AND** its status becomes quota-exceeded and blocked marker remains unchanged

#### Scenario: Credits retain existing recovery semantics
- **GIVEN** fresh usage reports usable credits
- **WHEN** standard usage windows are exhausted
- **THEN** the refresh does not extend the account reset from those windows

### Requirement: Weekly primary slot can recover from fresh usage
Recovery MUST recognize an explicitly weekly duration in the upstream primary slot as a long quota window. After the existing cooldown, fresh available quota MUST permit quota-exceeded recovery without waiting for the old exhausted deadline.

#### Scenario: Weekly quota becomes available early
- **GIVEN** a quota-exceeded account past its minimum cooldown
- **WHEN** fresh usage reports available weekly quota in the primary slot
- **THEN** the account becomes active and its exhausted reset marker clears
