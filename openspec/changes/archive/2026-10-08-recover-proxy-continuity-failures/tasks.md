# Tasks
- [x] Reproduce and fix hard-owner soft-backoff selection with quota/cap negatives.
- [x] Reproduce and fix rejected injected-anchor retirement with multi-request regression.
- [x] Audit native terminal/lease cleanup and fix any reproduced gap.
- [x] Exercise actual pi against candidate proxy and fake upstream for recovery and no unsafe replay.
- [x] Review, validate, sync specifications and archive only verified work.
- [x] Deploy reviewed local candidate to Camden and verify runtime identity and outcomes.

Combined proof includes deliberate safe stop for close -> anchored replay -> rejected anchor when the recovery checkpoint is ambiguous; automatic recovery of that sequence is not claimed. CodeRabbit completed all15 changed application/test files across initial and extraction delta reviews with0findings. Main specifications validated67/67.
