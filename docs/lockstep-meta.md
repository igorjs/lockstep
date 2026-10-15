<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-meta

What outlives a single run: the chronicle, control transfer, and archetypes with point-buy.

## Chronicle

`Chronicle` is the cross-run record. It is not part of any simulation's state: a run reads it when
it starts (the host copies what the run needs into the configuration), and the host folds the run's
events into it when the run ends.

- `begin_run()` starts a run and returns its number, counting from 1.
- `retire(name, run, members)` retires a name for good, once; the first record is kept. It returns
  whether the name was newly retired. `is_retired`, `lost` and `retired_names` read it back.
- `add(key, amount)` and `tally(key)` keep named counts, saturating.

## Envelope

`seal(&message)` returns bytes: the magic `LKMT`, the envelope version (1), the message version, a
checksum of the payload, and the payload (the fixed-endian encoding the hash covers). `open` reads
them back and refuses with an `EnvelopeError`: `NotAnEnvelope`, `EnvelopeVersion`,
`MessageVersion { found, expected }`, `Corrupt` or `Decode`. It works for any `Message`. Converting
an older version is not done here (deferral D6).

## Control

`Control` holds who is in control. `assume(who)` sets it. `lose(lost, fall_backs, events)` reports
that `lost` has left: if it was in control, control moves to the first of `fall_backs` that is not
`lost` and `ControlEvent::Transferred` is emitted; with none, control is vacant and
`ControlEvent::Vacant` is emitted. The caller chooses and orders the fall-backs.

## Templates

`Templates::from_json(text)` reads archetypes and a point-buy and checks them.

```json
{
  "budget": 6,
  "attributes": [
    { "name": "stamina", "minimum": 1, "maximum": 8, "cost_per_point": 1 },
    { "name": "judgement", "minimum": 1, "maximum": 8, "cost_per_point": 2 }
  ],
  "archetypes": [
    { "name": "cook", "base": { "stamina": 4, "judgement": 2 } }
  ]
}
```

- `build(archetype, purchases)` returns the archetype's base with the purchased whole points
  added, in attribute order. The cost is points times cost a point, summed, and may equal the
  budget but not pass it; no attribute may pass its maximum. A name bought twice adds up.
- Refused with a reason: repeated names, a minimum above the maximum, a cost of nothing, an
  archetype that omits an attribute, names an unknown one or starts outside the bounds, an unknown
  archetype or attribute in `build`, a purchase past a maximum, and a budget overrun.

See decision 0027.

## Tests

- `tests/behaviour/chronicle.rs`, `control.rs`, `envelope.rs`, `templates.rs`: each rule above and
  each refusal.
- `tests/decisions`: control falling back in the caller's order and never to the lost, a retired
  name staying retired across runs and a save, the budget spent exactly and no further, and the
  envelope carrying the message version (decision 0027).
