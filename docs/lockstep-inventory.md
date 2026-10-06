<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-inventory

Items as entities, containers, equipment, affixes and spoilage, all in integers.

## The catalogue

`Catalogue::from_json(text, &attribute_registry)` reads every kind, equipment slot and affix, and
the spoilage bands, and checks them. `kind_count` says how many kinds there are, so a simulation can check a
`KindId` that arrives in an intent before using it. Numbers are whole numbers or decimal strings, never JSON
fractions, as in the attribute registry.

```json
{
  "slots": ["suit", "hand"],
  "spoilage": [
    { "below": 5, "percent": 25 },
    { "below": 30, "percent": 100 },
    { "percent": 300 }
  ],
  "affixes": [
    { "name": "faulty_seal", "binds": true,
      "modifiers": [ { "attribute": "oxygen", "modifier": { "add": -20 } } ] }
  ],
  "kinds": [
    { "name": "ration", "tags": ["food"], "weight": "0.5", "stack": 6,
      "spoils_after_minutes": 1440 },
    { "name": "suit", "weight": 12, "slot": "suit", "affix_slots": 2,
      "modifiers": [ { "attribute": "oxygen", "modifier": { "add": 50 } } ] }
  ]
}
```

- A kind has a name, tags, the weight of one unit, a stack size (default one), whole game minutes
  to spoil at 100 percent (absent: never spoils), the slot it is worn in, a number of affix slots,
  and modifiers for the wearer. A modifier is `add`, `multiply` or `override`, as in
  `lockstep-attributes`.
- An affix has a name, modifiers for the wearer, and whether it binds the item to the wearer.
- Spoilage bands: the first band whose `below` is above the temperature sets the percent; the last
  band has no `below`. Temperatures are whole degrees.
- Refused with a reason: repeated names, an unknown slot or attribute, a stack of zero, a negative
  weight, modifiers on a kind with no slot (they would never apply), and bands that do not rise
  or do not end with an open band.

## The inventory

`Inventory` keeps items in their own `StableVector`, and each item's `Place`: loose, in a
container, or worn in a slot. Containers and wearers are keyed by the simulation's own handles,
so a crate, a locker or a person holds items. A `Container` has a number of slots, an optional
weight limit and a temperature, which `set_temperature` changes (a cold store losing power).

- `create` makes a loose item of some units. `put` puts a loose item into a container; `take`
  makes it loose again; `move_between` moves it from one container to another, all or nothing.
- A put merges the item into the first stack of the same kind and affixes, spoiled or not alike,
  that has room for every unit; the merged item is gone, and the stack spoils as the staler of the
  two. Spoiled and fresh units never share a stack. Otherwise the item
  takes a slot of its own. A put never splits an item: `split` first (decision 0017).
- `split` takes units off into a new loose item with the same affixes and spoilage. `consume`
  uses units up, and the item is gone at none. `destroy` removes an item.
- `find_by_tag` lists the items in a container whose kind has a tag, in container order.
- A refused operation returns a `Refusal` and changes nothing.

## Equipment and affixes

`add_wearer` gives an entity every slot in the catalogue. `equip` puts an item on in its kind's
slot, from loose or from a container, and adds its kind's and affixes' modifiers to the wearer's
`Attributes` (a wearer without attributes still wears). `unequip` removes them and leaves the item
loose; while an affix binds the item it is refused. `add_affix` fills a free affix slot and
`remove_affix` removes one copy; on a worn item their modifiers come and go at once, so removing a
binding affix is a repair that frees the item. A worn item cannot be put, moved, split, consumed
or destroyed.

## Spoilage

`spoil(minutes, outside, catalogue, events)` adds raw 16.16 game minutes times the percent for the
item's temperature: its container's, or `outside` when it is loose or worn. The item spoils when
the total reaches its minutes at 100 percent (`Spoiled`, once); a spoiled item stays and stops
counting. The totals are exact integers, so the same game minutes spoil the same at 30 or 60
steps a second (decision 0017). `freshness` reads from 1 down to 0, rounded down.

## Tests

- `tests/behaviour`: the catalogue and its refusals, containers, stacking, moves, splits, tags,
  saving and loading, equipment and affixes, and spoilage by temperature.
- `tests/decisions`: spoilage ignores the step rate, a stack spoils as its stalest unit, a put
  never splits an item, and a binding affix holds until repaired (decision 0017).
