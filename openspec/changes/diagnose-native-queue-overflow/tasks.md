## 1. Implementation
- [x] 1.1 Add typed immutable overflow snapshots at both queues before clearing.
- [x] 1.2 Add bounded queue observations and content-free terminal log fields.

## 2. Verification
- [x] 2.1 Observe new checks fail then pass for both overflow sites and correlated adapter output.
- [x] 2.2 Run focused regression tests, lint and strict OpenSpec validation.
- [ ] 2.3 Synchronize requirements and context and archive verified change.

## 3. Metrics and rollout
- [x] 3.1 Validate bounded Prometheus snapshot emission at both overflow sites.
- [x] 3.2 Create the m7kni dashboard with complete emitted-family coverage and log/trace drilldowns.
- [ ] 3.3 Review, deploy the exact local commit and verify runtime and telemetry.
