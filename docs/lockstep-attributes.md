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

## Curves

A derived attribute's value is a `Curve` of its inputs. `Curve::evaluate` saturates at the edges of
the 16.16 range instead of wrapping.

| Curve | Value |
|---|---|
| `linear { per_point, offset }` | `input × per_point + offset`; one input |
| `piecewise { knots }` | straight lines between knots, flat past the ends; one input |
| `threshold { steps }` | the value of the last step at or below the input, the first step's below them; one input |
| `product`, `sum` | every input multiplied, or added |
| `difference` | the first input minus the others |

## Tests

- `tests/behaviour`: registry reading and refusals, and every curve.
- Every test also runs under WebAssembly in Node.

Attributes, modifiers, thresholds and effects arrive in the next parts of milestone M4.
