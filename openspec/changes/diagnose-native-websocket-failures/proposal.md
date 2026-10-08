## Why
Camden loses active upstream websocket responses and clients retry through rejected anchors. Current native adapter diagnostics discard the cause. Local-only instrumentation must distinguish transport/protocol/liveness/helper failures without exposing content.

## What Changes
- Preserve typed, allowlisted native failure categories across Rust IPC and Python.
- Emit one content-free diagnostic for a terminal native websocket failure, with connection correlation, timing and counters.
- Prove the diagnostics using local wire failures and privacy regressions.
- Deploy the exact reviewed local commit with Camden update.sh; alert using existing active-request symptom logs.

## Capabilities
### New Capabilities
### Modified Capabilities
- `proxy-runtime-observability`: native websocket failure diagnostics.

## Impact
Native egress Rust/Python boundary and tests only; no routing, retry, settlement, public error, schema or credential changes. Local branch only, no upstream publication.
