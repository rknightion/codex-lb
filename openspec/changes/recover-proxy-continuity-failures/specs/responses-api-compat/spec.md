## ADDED Requirements

### Requirement: Responses stream ownership cleanup
The Responses API MUST close each owned upstream generator when normalization aborts, the consumer closes the response, or downstream consumption is cancelled. Closure MUST cascade through response-error and reasoning-summary wrappers so account stream leases and API-key reservations are finalized without waiting for garbage collection. Cleanup MUST preserve the existing terminal error classification and MUST NOT replay a dispatched request.

#### Scenario: Native synthetic transport failure releases capacity
- **WHEN** a native Responses request holds the only account stream slot and normalization aborts on a marked synthetic transport failure
- **THEN** the upstream stream and account lease MUST close before request teardown finishes
- **AND** a subsequent eligible request MUST be admitted without restarting the service

#### Scenario: Downstream cancellation closes wrapped generators
- **WHEN** downstream consumption ends while a response-error or reasoning-summary wrapper is suspended at a yield
- **THEN** closure MUST reach the owned source generator even in a cancelled task scope
- **AND** cleanup MUST preserve the original failure or cancellation outcome
