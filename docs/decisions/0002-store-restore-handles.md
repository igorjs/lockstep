<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0002 Store reuse after restore (decided)

Status: decided, implemented.

The reference wrapped `slotmap::SlotMap`. The slot reused next comes from a free list whose order
was not part of the saved form. Measured by `tests/spikes/restore_then_continue.rs`: restoring at a
random point of a random history, then inserting, handed out a different handle than continuing
the original (seed 0, operation 69). A save followed by more play would not match a straight run.

Decision: `StableVector` is a small purpose-built store.
- An insert always reuses the lowest vacant slot. Which slot an entity gets depends only on which slots are occupied, never on the order earlier entities were removed.
- Only the slots are saved. The set of vacant slots is derived when a store is loaded, so a restored store continues exactly as the original, and a corrupt save cannot hold a vacant set that disagrees with its slots.
- A handle is the slot index in the low 32 bits and the generation in the high 32 bits. Generations start at one and skip zero when they wrap, so the all-zero handle never refers to anything.
- A column entry remembers the generation of the handle it was set for, and drops trailing empty entries, so two columns with the same entries serialize and hash the same whatever built them.
- A load that finds a zero generation is refused.

Consequences: the `slotmap` dependency is removed from the core crate and from the workspace
dependency list. The public methods are unchanged, and the all-zero handle is a usable "no entity".
The handle layout differs from the reference, which used slotmap's internal layout.

Alternatives rejected: keeping `slotmap` (cannot fix the reuse order), and forbidding inserts after
a restore (replay from a save needs them).

Tests: `tests/decisions/store_reuses_the_lowest_free_slot.rs`, `tests/spikes/restore_then_continue.rs`
(now passing, including the random-history check), and the store tests in `tests/behaviour/store.rs`.
