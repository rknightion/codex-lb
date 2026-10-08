# Proxy Runtime Observability Context

## Purpose and Scope

This capability defines what operators should be able to see in the live server console while debugging proxy traffic.

See `openspec/specs/proxy-runtime-observability/spec.md` for normative requirements.

## Decisions

- **Timestamps are always on:** timestamped console logs are a baseline operator need, not a debug-only feature.
- **Request tracing is opt-in:** outbound request summary and payload tracing remain configurable because payload logs can be noisy or sensitive. Since issue #1340 phase 1 the switch is the single `CODEX_LB_TRACE` comma-separated channel list (`shape`, `shape_raw_cache_key`, `payload`, `service_tier`, `upstream_summary`, `upstream_payload`); empty default = all off. It is an incident-debugging knob for interactive use only.
- **Error logs must be correlated:** request id, endpoint, status, code, and message are the minimum useful fields for debugging 4xx/5xx failures.
- **Prewarm observability is outcome-only:** the Codex HTTP-bridge prewarm canary experiment finished, so its bucket/cohort dimensions were retired (issue #1340 phase 4). The `codex_lb_http_bridge_prewarm_total` counter is labelled by `outcome` only, request logs record `prewarm_status` / `prewarm_latency_ms` (statuses: `not_applicable`, `skipped`, `success`, `timeout`, `error` — `canary_miss` no longer occurs). The legacy `prewarm_canary_bucket` / `prewarm_eligible_reason` request-log columns stayed declared but unwritten through v1.22–v1.24; `retire-prewarm-canary-column-mappings` removed them from the ORM model and allow-listed the retained physical columns in the schema-drift gate, because a previous-release replica still maps them and renders explicit NULLs in its INSERTs while the migration Job runs ahead of the workload roll. The Alembic drop revision ships the release after (see the next-release queue in `openspec/specs/deployment-installation/context.md`).
- **TTFT datasource selection stays in Grafana:** the Helm chart packages the
  TTFT dashboard but does not provision a PostgreSQL datasource or its
  credentials. The visible, single-select `DS_SQL` variable keeps
  installation-specific datasource UIDs out of chart values while routing all
  four SQL panels through one explicit selection.

## Operational Notes

- Use request ids to correlate inbound proxy logs, outbound upstream traces, and client-visible failures.
- Prefer summary tracing in normal debugging sessions; enable payload tracing only when the exact normalized outbound request matters.
- For direct compact `5xx` failures, look for `proxy_compact_failure` alongside `upstream_request_complete`; together they show the compact failure phase, failure detail, exception type, retry metadata, and affinity source.
- After the Grafana sidecar imports the TTFT dashboard, select the ordinary
  PostgreSQL datasource that points to the codex-lb database from the visible
  **PostgreSQL** dropdown. A datasource registered only as a frontend runtime
  plugin is not listed by Grafana's datasource variable.
- Timeout invariant violation logs describe startup `Settings` and imported
  constant validation only. They intentionally avoid request-scoped overrides,
  runtime-derived effective timeout values, payloads, API keys, access tokens,
  raw affinity keys, account emails, and other high-cardinality identifiers.

## Affinity decisions in request logs

Issue #2349 adds three nullable columns to existing request logs: `sticky_key_source`, `sticky_kind`, and `sticky_key_hash`. They work without trace settings. Callers with `conversations:read` permission can also read them as `stickyKeySource`, `stickyKind`, and `stickyKeyHash` in `GET /api/request-logs`. Responses without that permission hide all three fields through the existing sensitive-metadata gate.

The hash is the first 16 lowercase hexadecimal characters of SHA-256 over the resolved selection key encoded as UTF-8. For example, a resolved key of `abc` records `ba7816bf8f01cfea`. Session selection keys can differ from raw session headers, so hashing the header separately does not reproduce that value. The metadata never stores raw keys or prompts, even when raw-key tracing is enabled. A hash supports equality grouping, but it is not protection against guessing low-entropy keys. Existing request-log retention applies.

Direct streaming rows retain their existing attempt granularity. Compact records its final operation, while native WebSocket and HTTP bridge rows describe the settled or failed request state. A row covering several sends records its final policy, not an intermediate decision history. Existing recovery can clear a key; that produces a null hash while retaining the original source classification. Metadata alone does not explain hard-owner precedence, rerouting, or account-health decisions.

Rows before affinity resolution, historical rows, auxiliary control/file/transcription/realtime/warmup rows, and external-model-source rows can have all three fields null. This means observation was unavailable. An explicit no-affinity observation uses source `none`; its kind and hash are null. No historical decision is reconstructed.

For a bounded administrator query:

```sql
SELECT sticky_key_source, sticky_kind, sticky_key_hash, COUNT(*) AS rows
FROM request_logs
WHERE requested_at >= CURRENT_TIMESTAMP - INTERVAL '1 day'
GROUP BY sticky_key_source, sticky_kind, sticky_key_hash;
```

## Authentication migration convergence

The affinity history converges with dashboard roles, users, the final compatibility-credential projection and audit actor columns through a no-op merge. Published revisions remain unchanged. Databases already on the authentication branch retain their current credentials and session generations; the earlier credential projection is not replayed on merge-only reupgrade. Databases on the older affinity history run the existing authentication backfills once. Ledgerless schema bootstrap retains those migrations' existing legacy-credential projection behavior.

The subsequent invite migration converges through a second no-op join. Pending, consumed and revoked invite rows retain their hashes, expiry/consumption/revocation times, creator snapshots and flags. The older affinity history creates an empty invite table through the unchanged upstream migration.


## Local native websocket diagnostics

Established native websocket failures emit `native_websocket_terminal`. Join its
`connection_id` to `http_bridge_event event=reader_failure diagnostic_connection_id`.
The reader trace can be inherited from an earlier turn; the connection join is the
reliable link. For example, `category=protocol protocol=ResetWithoutClosingHandshake`
identifies a peer transport ending without the websocket close handshake. It does not
identify who initiated the underlying TCP interruption or prove a remote service defect.

The Rust helper supplies explicit protocol, I/O, OS and TLS categories through optional
IPC metadata. Python validates bounded categories before logging. Peer reason text,
raw exceptions, URLs, headers and content are excluded. Unknown or older IPC metadata
stays unknown. Helper returncode before cleanup is separate from the later observed
returncode; a proxy cleanup signal is not evidence that helper death caused the failure.

Timing and counts describe successful application sends/receives at the adapter, not
ping/pong activity or network packets. Handshake failures keep their existing diagnostics.
Diagnostic sink exceptions do not replace the existing transport outcome. Cancellation
is not caught by that exception guard.

The Camden-only troubleshooting alert `codex-lb-stream-incomplete` evaluates active
reader failures or rejected-anchor 502s over five minutes every minute, threshold >0,
using the existing apps paging route. It excludes idle reader failures, counts events
rather than unique requests, and does not monitor log-delivery freshness. Continuous
firing follows the existing four-hour repeat policy. This branch is not published upstream.

## Native queue overflow follow-up

The two native queues share a public consumer-backpressure error. Pre-clear snapshots distinguish websocket_messages (64 messages) from helper_events (4096 events and 32MiB charged payload). Queue limits remain unchanged while measuring. Logs retain exact count, configured bounds, projected bytes, enqueue/dequeue counts and monotonic ages; bounded Prometheus histograms summarize overflow observations rather than current occupancy. Never-consumed gaps remain absent. Active streams include registered HTTP and websocket requests in one helper generation, not server-wide sessions. For example, count=64 with a very recent dequeue and young backlog suggests a burst; an old backlog and long dequeue gap suggests a delayed consumer. Neither proves CPU saturation. Payload-byte counts are not Python heap usage.

The troubleshooting Grafana dashboard covers the union of source-defined and live codex_lb_ families, including families without samples, with log and trace drilldowns. Live metric availability is distinct from panel coverage.

## Burst tolerance and local failure attribution

Production snapshots showed 64 queued messages occupying about 12.7 KiB with 5–11 ms backlog age and a consumer dequeue under 1 ms ago. The native websocket pump now shares scheduling turns and uses 4096 messages / 32 MiB charged payload bounds. True overflow retains its diagnostic snapshot and carries native_websocket_backpressure provenance so upstream account health is unaffected. These are per-socket bounds, not a global memory budget. The dashboard tracks overflow snapshots alongside concurrency and event-loop lag.
