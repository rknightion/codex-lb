## Context
Two bounded queues share consumer_backpressure. The websocket message queue has a count bound; helper event queues have count and charged-byte bounds.

## Goals / Non-Goals
Identify the saturated queue and distinguish short bursts from delayed consumers. Do not tune capacities or alter retry behavior.

## Decisions
Capture an immutable typed snapshot before clearing a rejected queue. Track enqueue timestamps and dequeue counters using REAL_CLOCK. Payload bytes represent the existing helper charging model or websocket UTF-8/binary bytes, not Python heap usage. Active helper streams counts registered requests in the same helper generation, including HTTP and websocket streams. Emit once through the existing no-throw terminal logger.

## Risks / Trade-offs
Per-item timestamps and counters add bounded overhead. No periodic logging or background tasks are introduced. Dequeue gap is absent when never consumed; backlog age remains available. Count/byte observations cannot establish a remote or host-level cause alone.

## Migration Plan
Deploy on the existing local troubleshooting branch using the authorized updater; preserve prior image and SHA.

## Open Questions
Production evidence will determine whether capacities or scheduling need changes.
