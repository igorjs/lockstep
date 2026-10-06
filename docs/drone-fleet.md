<!-- SPDX-License-Identifier: Apache-2.0 -->

# drone-fleet (example)

A consumer scenario that is not a game: six delivery drones fly from one depot, and their batteries
charge, drain and wear. It is the second consumer of `lockstep-attributes` and of `Chance`.

- Battery: three attributes read from `data/attributes.json`. `wear` is a ratchet with marks at 25
  (`worn`), 50 (`tired`) and 75 (`retire`): each mark the wear rises through becomes a floor.
  `capacity` is derived from wear (100 minus wear). `charge` has levels at 20 (`low`) and 5
  (`critical`), and its maximum is held at the capacity by an `Override` modifier.
- Intents: `Launch { drone, minutes }`, `Dock { drone }`, `Service { drone }`.
- A launch needs a docked drone with at least 25 charge and less than 75 wear: a drone past the
  `retire` mark never launches again, since service cannot take it back under the mark. The flight is an effect that drains 2 charge
  a game minute for its length, and every launch risks a fault (a 5 percent `Chance` roll on the
  `faults` stream) that adds 12 wear.
- A flight that runs its course lands and docks. Docking is an effect that charges 4 a game minute
  until the next launch.
- A drone that runs dry in the air is stranded. `Dock` recovers it and adds 8 wear.
- `Service` removes 20 wear from a docked drone, but never below the last mark passed.
- Events: `Launched`, `Fault`, `Landed`, `Docked`, `Stranded`, `Serviced`, `Battery { level }`,
  `Wear { mark }`, and `Refused { reason }` (`NotDocked`, `TooLow`, `Retired`, `NotFlying`,
  `UnknownDrone`).

The fixture gives a random order to a random drone now and then, drawn from a separate stream set so
the script depends only on the seed. Its hash is committed in `fixtures/drone-fleet.hash` and checked
natively and under WebAssembly.
