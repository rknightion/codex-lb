## Why
Production websocket failures report consumer_backpressure without identifying which queue overflowed. Operators cannot distinguish per-socket bursts from helper-wide contention.

## What Changes
- Add content-free snapshots at both overflow sites and include them in correlated terminal diagnostics.
- Record queue identity, occupancy, limits, rejected/projected payload bytes, producer/consumer counts, oldest backlog age, dequeue gap and active helper streams.
- Expose bounded codex_lb_ overflow snapshot metrics and a comprehensive m7kni diagnostic dashboard.
- Preserve queue sizing, acceptance, cancellation and public errors.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- proxy-runtime-observability: distinguish queue overflow mechanisms.

## Impact
Native Python queues and websocket adapter terminal logs; no Rust IPC or public API change.
