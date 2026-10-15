<!-- SPDX-License-Identifier: Apache-2.0 -->

# franchise (example)

The consumer scenario for `lockstep-meta`, and not a game. A franchise keeps a crew of shift
staff in lines that share a family name. Four lines found each season (Okafor, Silva, Tanaka and
Haddad); the first founder is the shift manager.

- A hire names a line, an archetype from `data/templates.json` (`cook`, `cashier`, `lead`) and
  points bought above the archetype's base (`Templates::build`). The data allows 6 points, stamina
  at 1 each and judgement at 2 each, and 8 at most in either. A refused purchase is reported with
  its reason.
- When the manager departs, control falls back (`Control::lose`) to the newest member of the same
  line, then to anyone else in handle order. With no one left it is vacant, and the next hire
  takes it.
- When the last member of a line departs, the line is lost: the event `LineLost` counts everyone
  the line ever had, and the name is burned. No later hire may carry it, in this season or any
  later one (`Refused` with `Burned`).
- The chronicle is not in the simulation's state. After each season `record` folds the events
  into it (lost lines retired, departures tallied); it is saved with `seal` and read with `open`;
  the next season's configuration takes its burned names from it.

The fixture hash in `fixtures/franchise.hash` covers three seasons of 2,400 steps each, with the
chronicle sealed and opened between them, and combines the last season's runner hash with the
final chronicle. It is checked by `lockstep-headless verify franchise` and by a WebAssembly test.

The milestone gate is `the_chronicle_survives_a_new_season`: the lines lost in season one are in
the chronicle after it is saved and opened, season two starts with none of them on its crew and
refuses to hire their names, and two further seasons leave the record of season one untouched.
Other tests: control passing to the newest of a line, then to another line, then vacant; a burned
name refused in the same season; point-buy within the budget and the maximum; a departure naming
no one being unknown; the same seed playing the same seasons; and a restored snapshot continuing
exactly.

This example stands in for the reference's meta consumer: death, a fall-back, and a burned name
(departure P24 in the roadmap).
