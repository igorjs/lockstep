# capsule (example)

The game shaped consumer scenario. One survivor lives in a sixteen by sixteen room.

- Intents: `MoveTo { entity, cell, run }`, `Stop { entity }`.
- Walking: one cell every six steps, three when running, horizontal first, then vertical.
- Needs: hunger, thirst, and sanity drain per game minute, so a rest at twenty times drains twenty times faster.
- Starvation: when hunger or thirst is empty, health drains. At zero the survivor dies and every column entry is removed.
- Events: `Arrived`, `Hungry`, `Starving`, `Died`, `Rejected`.

The fixture script walks, runs, sends one bad move that is rejected, then rests at twenty times until
the survivor starves. Its hash is committed in `fixtures/capsule.hash`.
