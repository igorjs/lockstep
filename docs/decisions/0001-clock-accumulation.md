# 0001 Clock accumulation (decided)

Status: decided, implemented.

The reference code accumulated `minute_of_day` as a 32-bit float on every step. Its test says one
real day of steps at a 120 minute day advances exactly 1,440 game minutes. Measured: after 216,000
steps the clock read 1.3747971 minutes into the next day, identically on native and WebAssembly. A
day took about 215,800 steps. The result was deterministic but not exact.

Decision: game time is an exact integer position inside the day. One step at multiplier 1.0
advances 65,536 units, and a day is the number of steps per day times 65,536 units. The multiplier
is stored in 1/65,536ths (rounded, capped at 1,000,000, and zero for negative or not-a-number values).
A 120 minute day is exactly 216,000 steps, and a day at twenty times is exactly 10,800 steps.

Consequences:
- The public methods are unchanged. `minute_of_day()` is now derived from the integer position and is for display and reading, not for equality.
- `Clock::position_units()` is new. The runner hash uses it instead of the minute-of-day float bits, so two different positions can never hash alike.
- The reference hashed the float bits of the minute. This is a deliberate change of the hash definition.
- Elapsed game minutes per step are still `game_minutes_per_step * multiplier` as a 32-bit float, for systems that drain by rate.

Alternatives rejected: keeping the 32-bit float and relaxing the claim, and 64-bit floats (smaller drift, still not exact at the boundary).

Tests: `tests/decisions/clock_in_integer_ticks.rs`, and the one-day test in `tests/behaviour/clock.rs`, which now asserts an exact return to the start.

Fixture hashes changed with this decision (capsule and ledger), because the clock position is part of the state hash.
