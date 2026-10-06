<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-attributes

Every stat is the same machine: a base maximum, modifiers, a clamped current value, thresholds that
fire once per crossing, and derived values computed by curves. Health, sanity, hunger, a rover's
battery, a credit limit. Integer arithmetic only (`Fixed32`), so every platform agrees.

## Registry

A `Registry` lists every attribute an entity can have, in a stable order; an `AttributeId` is an
index into it. Build one with `Registry::new` or read one from JSON with `Registry::from_json`. Both
check it and refuse, with a `RegistryError`: duplicate or empty names, a minimum above the maximum, a
starting value or threshold outside them, a derived input that does not exist or is the attribute
itself, derived attributes that read each other in a loop, the wrong number of inputs for a curve, and knots or steps that are empty or not strictly
increasing.

```json
{
  "attributes": [
    { "name": "health", "minimum": 0, "maximum": 100, "starting": 100,
      "on_maximum_change": "scale_current",
      "thresholds": [ { "at": 20, "name": "wounded" } ] },
    { "name": "luck", "minimum": 0, "maximum": 20, "starting": 3, "on_maximum_change": "scale_current" },
    { "name": "critical_chance", "minimum": 0, "maximum": 100, "starting": 0,
      "derived": { "inputs": ["luck"], "curve": { "linear": { "per_point": "1", "offset": "5" } } } }
  ]
}
```

Numbers are whole JSON numbers (`100`) or decimal strings (`"1.5"`), parsed exactly by
`Fixed32::parse_decimal`. A JSON fraction such as `1.5` is refused, because a float would not read
the same everywhere. Unknown fields are refused too. `on_maximum_change` defaults to `clamp`,
`thresholds` to none, and `derived` to none (a primary attribute).

## Attributes and modifiers

`Attributes::from_registry` gives one entity every attribute at its starting value. Keep it in a
`Column<Attributes>`.

- `apply(who, id, delta, registry, events)` changes the current value by `delta`, clamped between the
  minimum and the maximum.
- `add_modifier` returns a `ModifierHandle`; `remove_modifier` takes it back, so an item removes
  exactly the modifier it added.
- The maximum is `(base + every Add) × every Multiply`, then the last `Override` added wins, and it
  never falls below the minimum. Base 100 with +20 and ×1.5 is 180 in either order.

What a new maximum does to the current value is the attribute's `MaximumPolicy`:

| Policy | Example | 50 of 100, then ×1.5 |
|---|---|---|
| `clamp` | hunger | 50 of 150 |
| `scale_current` | health, Luck | 75 of 150 |
| `ratchet` | corruption | 50 of 150, and each threshold the value rises through becomes the minimum |

## Events

`AttributeEvent` names the entity (`who`) and the attribute:

- `Crossed { threshold, upward }`: once each time the current value passes a threshold, in the order
  it passes them. Falling below 60, 40 and 20 in one change gives three events, highest first.
  Staying on one side gives none.
- `Emptied`: the current value reached the minimum.
- `Filled`: the current value reached the maximum, including when a lowered maximum meets it.

## Derived attributes and curves

A derived attribute is data: inputs and a `Curve`. Every derived attribute is recomputed in registry
order after any change, clamped to its own minimum and maximum, and reports its own crossings.
`apply` on a derived attribute changes nothing.

| Curve | Value |
|---|---|
| `linear { per_point, offset }` | `input × per_point + offset`; one input |
| `piecewise { knots }` | straight lines between knots, flat past the ends; one input |
| `threshold { steps }` | the value of the last step at or below the input, the first step's below them; one input |
| `product`, `sum` | every input multiplied, or added |
| `difference` | the first input minus the others |

A derived attribute that reads one declared after it sees that one's value from before the change
and catches up on the next change (`tests/spikes`). Declare inputs first.

## Tests

- `tests/behaviour`: registry reading and refusals, changes and events, modifier handles, curves, and
  a saved set of attributes continuing exactly like the original.
- `tests/decisions`: Add then Multiply then Override, scale versus clamp, thresholds once per crossing,
  derived values following their inputs, and the ratchet. See decision 0008.
- `tests/spikes`: the one-change lag of a derived input declared later.
- Every test also runs under WebAssembly in Node.

Effects (timed modifiers and drains) and their stacking arrive in the next part of milestone M4.
