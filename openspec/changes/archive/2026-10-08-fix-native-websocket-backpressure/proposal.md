## Why
Small websocket bursts overflow a 64-message queue despite recent consumer progress. Local overflow is then misclassified as upstream account failure.

## What Changes
Use a 4096-message, 32 MiB payload budget with cooperative pump scheduling. Preserve bounded nonblocking control handling and report local overflow with an account-neutral internal code.

## Capabilities
### Modified Capabilities
- `proxy-runtime-observability`: queue bounds and local failure classification.

## Impact
Native websocket client, adapter and regression coverage; no environment setting or protocol change.
