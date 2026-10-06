<!-- SPDX-License-Identifier: Apache-2.0 -->

# Roadmap and gaps

Where Lockstep stands against the reference's sixteen milestones, and every place the work is
short of the reference, differs from it, or defers part of it. Every pull request that changes
either keeps this page current. This repository holds the framework only: host, art and game
items in a milestone are out of scope here.

## Milestones

| Milestone | Status | What exists | Gate |
|---|---|---|---|
| M1 Core | Done | Workspace, continuous integration on three platforms, `Simulation`, `Runner`, `Clock`, store, `Streams`, hashing, `Message`; capsule and ledger | Met: `just ci` green on Linux, macOS, Windows; hashes equal natively and under WebAssembly |
| M2 Host | Out of scope | Lives with the host, not in this repository | Not applicable here |
| M3 Grid and math | Done, two open items | `lockstep-spatial`: topologies, map, occupancy, A*, path batch, flow field, line of sight; `Fixed32`, `Vector2`, angles; benchmark; mars-rovers, crowd | Tie-breaking holds; isotropy holds for squares, not for hexagons (gap G1); baseline committed |
| M4 Attributes | Done | `Chance` and three rolls; `lockstep-attributes`: registry from JSON, modifiers, thresholds, derived curves, effects with four stacking rules; exact game minutes; capsule needs, bleeding and prayer as data; drone-fleet | Met: stacking, crossing, frame-rate independence tests green |
| M5 Timeline, replay, derive | Done, three deferrals (D1 to D3) | `#[derive(Message)]` and `Indexable`; recorder, replay, bisect; timeline with compaction; ledger journal; headless `record`, `replay`, `bisect`, `stats` | Met: a planted rule change is found at the right checkpoint |
| M6 Combat and movement | Done | `lockstep-combat`: damage order, hit shapes, knockback, actions, dodges, projectiles, movement; sparring; rovers ram | Met: the duel fixture hash is equal natively and under WebAssembly; 200 partners spar for 10,000 steps without sharing a cell or going below zero health |
| M7 Inventory | Done | `lockstep-inventory`: catalogue from JSON, items as entities, containers, stacking, equipment with modifiers, binding affixes, spoilage by temperature; cargo-bay | Met: the same game minutes spoil on the same minute at 30 and 60 steps a second, natively and under WebAssembly |
| M8 Agents | In progress | `lockstep-agents`: sight cones with line of sight, staggered checks, noise carried by the wind, hearing thresholds, psychic noise; memory that fades, alert states, the director's budget, leashes; steering by flow field with the shared sidestep | Pending: utility and delegation, weather, the rover consumer, the benchmark |
| M9 Knowledge | Not started | | |
| M10 Progression | Not started | | |
| M11 Relations | Not started | | |
| M12 Crafting | Not started | | |
| M13 Regions and calendar | Not started | | |
| M14 Structures and procedural generation | Not started | | |
| M15 Meta, lineage, templates | Not started | | |
| M16 Hardening | Not started | Saves, migrations, fuzzing and Miri, and money are framework items | |

## Gaps: short of the reference

| | Gap | Where | Status |
|---|---|---|---|
| G1 | Hexagon distance is 15.47 percent off true distance; the reference asks for 3 percent, which no hexagon step count can meet | Decision 0005 | Open: needs the reference's figure revised |
| G2 | 500 paths on a 512 by 512 map take about 611 milliseconds serially (88 with the path batch on ten threads); the reference asks for 50 | Decisions 0005, 0006 | Open: needs hierarchical search, bidirectional search or a cheaper step, or a revised target |
| G3 | `bisect` narrows to the checkpoint, not the step, unless the recording checkpoints every step | Decision 0011 | Accepted: the recording has no hash between checkpoints |
| G4 | Only the ledger can be recorded by `lockstep-headless record` | `crates/lockstep-headless/src/recordings.rs` | Open: add other simulations as they gain consumers |

## Deferrals: built later, when a consumer pulls them

| | Deferral | Milestone | Waits for | Record |
|---|---|---|---|---|
| D1 | Schema export from `#[derive(Message)]` | M5 | A tool that reads schemas | Decision 0010 |
| D2 | The crash bundle (a recording, the last good save and the host's log) | M5 | A host, which writes it | Decision 0011 |
| D3 | `replay --save` and saving mid-recording | M5 | Saves and migrations (M16) | `docs/lockstep-headless.md`, `docs/lockstep-core.md` |
| D4 | Conversions between message versions | M16 | Saves: until a save ships, no version has shipped | Decision 0010 |
| D5 | The director's waves, spacing a horde's arrival into pulses | M8 | A consumer with a horde that arrives over time | `docs/lockstep-agents.md` |

## Departures: where the work differs from the reference on purpose

| | Departure | Record |
|---|---|---|
| P1 | Game time is an exact integer position, not an accumulated float, and the runner hash covers that position instead of the float minute | Decision 0001 |
| P2 | Entities live in a custom `StableVector` that reuses the lowest free slot, not a slot map | Decision 0002 |
| P3 | The core crate is `lockstep-core`; the spatial crate is `lockstep-spatial` (the reference says `lockstep-grid`) | Decisions 0003, 0005 |
| P13 | `abs` and negation wrap instead of panicking, `Vector2::length` works in 64 bits, and `from_ratio` rounds ties away from zero | Decision 0004 |
| P4 | A path batch may run on a thread pool, the one place threads are allowed | Decision 0006 |
| P5 | A ratchet attribute raises its minimum to each threshold it rises through | Decision 0008 |
| P6 | Thresholds fire in the order the value passes them, not definition order | Decision 0008 |
| P7 | `EffectContext` bundles the effect call arguments; `tick` takes the game day | Decision 0009 |
| P8 | A recording may keep snapshots, so `bisect` can show what differs | Decision 0011 |
| P9 | `Defence` carries evasion and block; `Reach` stops at the first body and `Line` pierces | Decision 0013 |
| P10 | Combat phases count whole steps; the counter window runs only while the fighter is free | Decision 0014 |
| P14 | Projectiles fly at an absolute altitude instead of checking `can_step`, and a dodged projectile flies on | Decision 0015 |
| P11 | The perfect window is 0.12 seconds; its cases are 3 steps (0.100 seconds, perfect) and 4 (0.133, not), the whole steps either side of the reference's 0.11 and 0.13 second cases at 30 a second | Decision 0014 |
| P15 | The reference's survivor-against-an-opponent consumer is the `sparring` example, two neutral partners, so the survivor example stays free of combat | `docs/sparring.md` |
| P16 | The reference's inventory consumers (a can of food and a knife for the survivor, and a Mars cargo bay) are one non-game `cargo-bay` example | `docs/cargo-bay.md` |
| P17 | "A gale masks walking" is read as a 14 metre a second gale cutting a 6 metre footstep to 1.8 metres across the wind and nothing against it; no single threshold silences footsteps from 4 to 9 metres | Decision 0018 |
