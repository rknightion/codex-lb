# Proposal
## Why
Continuity-bound clients can be refused during soft owner backoff or repeatedly encounter a rejected injected anchor. Client retry handling differs between HTTP and streaming envelopes, making bounded and safe recovery necessary.
## What Changes
- Admit a sole hard continuity owner during soft transient backoff while preserving hard eligibility and capacity limits.
- Retire rejected proxy-injected anchors with generation fences and verify recovery on a following portable full-context request.
- Verify transport terminal cleanup and implement only reproduced missing cleanup paths.
## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `account-routing`: narrow hard-owner soft-backoff fallback.
- `sticky-session-operations`: safe retirement of rejected injected anchors.
- `responses-api-compat`: close owned response generators when the downstream ends, releasing stream leases.
## Impact
Balancer selection and HTTP bridge continuation, with route regressions. Local troubleshooting deployment only; no upstream publication or schema change.
