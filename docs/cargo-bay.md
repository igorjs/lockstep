<!-- SPDX-License-Identifier: Apache-2.0 -->

# cargo-bay (example)

The consumer scenario for `lockstep-inventory`, and not a game. A crew of three works a cargo bay
with a rack (8 slots, 60 weight, 18 degrees), a cold store (6 slots, 3 degrees) and an airlock
locker (4 slots, 40 weight, 30 degrees).

- The catalogue (`data/catalogue.json`) has rations that stack to six and spoil after 360 game
  minutes at 100 percent, water, suits that add 50 to the wearer's oxygen maximum, and drills.
  Spoilage runs at 20 percent below 5 degrees, 100 percent below 25, and 250 percent above.
- The crew attributes (`data/attributes.json`) are oxygen and nourishment.
- Orders: deliver new items into a store (refused deliveries leave nothing behind), move an item
  between stores, eat a ration (25 nourishment fresh, minus 10 spoiled), wear and take off gear,
  a seal failing on a suit (an affix that binds the suit and takes 30 oxygen), a repair that
  removes it, throwing an item out, and a store's power going off or coming back
  (`set_temperature`).
- Each step runs the orders, then spoils everything by the step's exact game minutes, at the
  cabin's 18 degrees for anything loose or worn.

The fixture hash in `fixtures/cargo-bay.hash` covers 60,000 steps of a scripted session with
deliveries, meals fresh and spoiled, faults and repairs, and power cuts. It is checked by
`lockstep-headless verify cargo-bay` and by a WebAssembly test.

The milestone gate is `spoilage_does_not_depend_on_the_step_rate`: rations on the rack and in the
cold store, with the cold store losing power after two game hours, spoil on game minutes 360 and
456 at both 30 and 60 steps a second.

This example stands in for the reference's inventory consumers, the survivor's can of food and
knife and a Mars cargo bay (departure P16 in the roadmap).
