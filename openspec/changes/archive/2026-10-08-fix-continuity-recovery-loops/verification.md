# Accepted revision

The operator corrected the initial stop-on-missing-history proposal and explicitly authorized a fresh upstream conversation so loops can recover from goals on disk. The candidate now prefers full replay, then restarts portable unanchored current input after guarded durable owner retirement. Explicit anchors and account-bound payloads remain protected.

# Evidence

- Red route reproduction with a real persisted obsolete circuit returned 404 after owner retirement instead of serving the fresh conversation. The reset needed both captured episode fencing and authoritative poison/quarantine removal.
- Initial plain retirement and old400 behavior were superseded by the accepted revision; do not use those earlier test results as proof of the new policy.
- Existing full replay/next-turn route cases remain green (3 checks), and prior alias/sibling tests remain unchanged.
- Final accepted candidate: six route checks passed; 17 focused replay/owner/sibling unit checks passed; Ruff and strict change validation passed.
- The new six-case route check covers paused owner, long quota reset, short reset, unknown reset, explicit anchor protection and a concurrently advanced circuit that must remain intact.
- HTTP400 invalid_request_error remains only for unrecoverable protected input and missing ownership. Exact envelope is not retryable according to installed personal pi1.0.4 classifier.

# Boundaries

Fresh start deliberately loses unavailable upstream history. It does not read conversation archives. No broad sibling alias overwrite or scheduler change. Stale/newer circuit generations remain protected; a new failure may still legitimately refuse recovery.

Root extended the fresh-start route case with a third request using the same headers; both successful fallback variants remained off the retired account. Six-case gate passed after extension. Independent review included this test delta.

Final review: CodeRabbit completed on35files with2minor documentation-spacing findings, both corrected. Independent application review passed. Final architecture/timing/Ruff and67main-spec checks passed.
