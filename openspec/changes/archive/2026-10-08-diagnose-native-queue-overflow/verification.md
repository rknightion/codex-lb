# Verification

- Extended existing subprocess helper event-byte overflow test: immutable pre-clear snapshot and actual Prometheus counter delta.
- New subprocess websocket 65-frame burst: public adapter returns error with connection correlation; terminal log shows 64 messages, 640 UTF-8 payload bytes, rejected 10/projected 650 bytes, no byte bound, no previous dequeue, owning-generation stream count 1 and no payload disclosure.
- Existing bounded-queue test now uses deterministic REAL_CLOCK seam to prove 3-second oldest backlog age and 1-second consumer gap with enqueue/dequeue counts.
- Initial diagnostic tests failed before implementation (missing typed snapshot and missing log fields), then passed.
- Timestamp mutation produced 4000ms rather than required 1000ms and failed; restored candidate passed.
- Disabled metric wiring produced absent counters at both real overflow sites; restored wiring passed with the metrics extra installed.
- Final focused regression gate: uv run --extra metrics pytest tests/unit/test_native_egress.py tests/unit/test_proxy_websocket_client.py -q --timeout=30: 145 passed.
- Ruff check/format and git diff --check passed for the diagnostic source and test files.
- Strict OpenSpec change validation passed.

Metrics tests, dashboard delivery, review, runtime deployment and archive closeout are owned by the root and sibling lanes.

## Interpretation limits

Queued bytes are payload accounting, not heap use. Websocket bytes count UTF-8 text or binary bytes; parsed duplicate payload structures are not heap-estimated. Helper bytes retain their existing charging model. Zero dequeue gap means the last dequeue was less than one millisecond ago; absent means never dequeued. Active streams means registered HTTP and websocket requests in the same helper generation, not all sessions on the server. Snapshot histogram samples describe overflow conditions, not continuous queue occupancy. No capacities, retries or scheduling behavior were changed.

## Integrated verification

- Combined metrics/native/adapter gate: 158 passed with metrics extra installed; Ruff and diff checks pass.
- OpenSpec 1.13.1: all 67 main specs pass after synchronization.
- Dashboard codex-lb-runtime-full published in m7kni codexlb2otel, covers 71 families; 90 live sample names mapped without gaps. All 110 target queries returned success, 63 had data before queue deployment. Two visual renders inspected and corrected. Unknown/not-yet-emitted metrics remain no-data.
- Existing absent-Prometheus test fixture now blocks import_module via sys.modules; its assertions and intended behavior are unchanged.

- CodeRabbit completed with zero findings, all five changed code/test files covered.
- Camden updater exited zero deploying b04c0425f3fc7d395b64643e7d0f1fb8ae046b11. Healthy, migration policy OK, no schema drift. Three running source hashes match candidate. Metrics endpoint exposes HELP for all six queue families.
- Initial post-deploy Loki12:35:30–12:36:30UTC query had no terminal observations; Mimir had no queue samples yet. Production event-to-Mimir proof remains pending a natural overflow; subprocess-to-registry path is verified locally.
- No queue sizing, retry behavior or account penalty changes; no upstream publication.
