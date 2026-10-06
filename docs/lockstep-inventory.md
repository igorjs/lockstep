<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-inventory

Items as entities, containers, equipment, affixes and spoilage, all in integers. The catalogue
is in place; the inventory that uses it follows in the same milestone.

## The catalogue

`Catalogue::from_json(text, &attribute_registry)` reads every kind, equipment slot and affix, and
the spoilage bands, and checks them. Numbers are whole numbers or decimal strings, never JSON
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
  weight, and bands that do not rise or do not end with an open band.

## Tests

- `tests/behaviour/catalogue.rs`: reading every field, the spoilage bands at their edges, and
  each refusal.
