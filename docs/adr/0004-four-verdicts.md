# ADR-0004: Four-valued verdict contract

**Decision:** PASS (invariant observed), FAIL (invariant violated with all preconditions), INCONCLUSIVE (insufficient required evidence), ERROR (experiment/evidence infrastructure defective). Exit codes are 0/2/3/4 respectively.

**Rejected:** Binary success/failure when fault injection, retry sequence or independent observation was missing. This avoids misclassifying an interrupted experiment as a real target invariant violation.
