## Why
Exhausted owners and their stale retry circuits can trap loops in retries. The operator explicitly accepts starting a new upstream conversation with the current request when retained history is unavailable.

## What Changes
- Prefer verified full replay, otherwise allow controlled portable fresh starts on a matched long-unavailable durable owner.
- Exclude the old account, remove old upstream affinity headers and settle only the captured obsolete retry circuit/quarantine.
- Preserve explicit client anchors and account-bound state; return an actionable error when these cannot recover safely.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `responses-api-compat`: controlled fresh-start recovery and fenced circuit settlement.

## Impact
HTTP bridge recovery only. No schema or new setting; extends the existing request-path owner-retirement policy on this local-only troubleshooting branch.
