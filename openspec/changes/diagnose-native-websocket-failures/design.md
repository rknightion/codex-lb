## Context
Observed native receive errors become generic stream_incomplete; subsequent anchor 502s are distinct. The native helper already knows failure phase but loses typed transport detail before operational logging.

## Goals / Non-Goals
Expose evidence for failure attribution and correlate it to existing bridge logs. Preserve all transport/retry behavior. Do not log raw exceptions, headers, bodies, URLs, close reasons, prompts or credentials. Do not claim a local transport category proves a remote root cause.

## Decisions
Use additive optional IPC diagnostic fields and bounded categories; absent evidence remains absent. Capture monotonic connection lifetime, last receive/send gaps and counts where available. Include a generated connection identifier and native request correlation, and safe helper lifecycle evidence if available. Emit at the native adapter boundary before its existing generic public error conversion. Keep stderr discarded.

## Risks / Trade-offs
Protocol variants can contain user data, so serialize only explicit variant labels and numeric OS codes, never Display/Debug strings. Background-reader trace context can outlive a turn, so connection correlation must not rely on trace IDs alone. Logging must not raise or change failure handling.

## Migration Plan
No migration. Review and test locally, import a git bundle into Camden, preserve any updater-rotated backups, run update.sh --sha with the exact local SHA, verify running file identity and observe naturally occurring diagnostics. Roll back with the prior SHA through the same updater if needed.

## Open Questions
The next natural failure determines the actual category; diagnostics alone cannot establish remote intent.

## Reviewed refinements
Include numeric peer closes even when receive returns normally. Join the generated native connection identifier to the bridge reader failure. Do not equate helper EOF with a proven exit or cleanup termination with original cause. Last application receive and successful send timings must be named precisely; control-frame progress is unknown unless measured. Missing/unknown optional IPC fields preserve compatibility. The alert matches pending>0 reader failures and rejected-anchor 502s, excludes idle closes, and uses the existing apps paging route. Verify alert evaluation and delivery evidence separately.

Established websocket operation failures are the scope of this change. Handshake failures remain on existing connect diagnostics. Diagnostic sink exceptions are isolated from the public websocket result; cancellation is not swallowed.
