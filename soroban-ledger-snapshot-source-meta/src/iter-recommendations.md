# Recommendations for Simplifying `iter.rs`

## 1. Separate "Boundary" Logic from the Main State Machine

The `ProcessingPhase` enum conflates two different iteration modes:
- **Boundary mode**: Forward iteration through `Before` groups only (lines 32-35)
- **Normal mode**: Reverse iteration through both groups (lines 37-41)

This dual-purpose design causes complexity in `advance()` (lines 105-195) and `next()` (lines 273-283).

**Recommendation**: Create two separate iterator types:
```rust
enum IteratorMode<'a> {
    Boundary(BoundaryIterator<'a>),  // Forward, Before-only
    Normal(ReverseIterator<'a>),     // Reverse, both groups
}
```

## Summary of Key Structural Changes

| Current | Proposed |
|---------|----------|
| One iterator with boundary/normal modes | Two iterator types composed together |
| Group-based skipping during iteration | Normal flow: After only; Boundary flow: Before only |
| Complex `advance()` state machine | Pre-computed phase sequence or simpler transition table |
| Repeated TransactionMeta version matching | Normalized accessor trait/struct |

These changes would reduce the ~415 lines to roughly 250-300 lines while making the control flow linear and predictable.
