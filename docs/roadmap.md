<!-- SPDX-License-Identifier: Apache-2.0 -->

# Roadmap and gaps

Where Lockstep stands against the reference's sixteen milestones, and every place the work is
short of the reference, differs from it, or defers part of it. Every pull request that changes
either keeps this page current. This repository holds the framework only: host, art and game
items in a milestone are out of scope here.

## Milestones

| Milestone | Status | What exists | Gate |
|---|---|---|---|
| M1 Core | Done | Workspace, CI on three platforms, `Simulation`, `Runner`, `Clock`, store, `Streams`, hashing, `Message`; capsule and ledger | Met: `just ci` green on Linux, macOS, Windows; hashes equal natively and under WebAssembly |
| M2 Host | Out of scope | Lives with the host, not in this repository | Not applicable here |
| M3 Grid and math | Done, two open items | `lockstep-spatial`: topologies, map, occupancy, A*, path batch, flow field, line of sight; `Fixed32`, `Vector2`, angles; benchmark; mars-rovers, crowd | Tie-breaking holds; isotropy holds for squares, not for hexagons (gap G1); baseline committed |
| M4 Attributes | Done | `Chance` and three rolls; `lockstep-attributes`: registry from JSON, modifiers, thresholds, derived curves, effects with four stacking rules; exact game minutes; capsule needs, bleeding and prayer as data; drone-fleet | Met: stacking, crossing, frame-rate independence tests green |
| M5 Timeline, replay, derive | Done, three deferrals | `#[derive(Message)]` and `Indexable`; recorder, replay, bisect; timeline with compaction; ledger journal; headless `record`, `replay`, `bisect`, `stats` | Met: a planted rule change is found at the right checkpoint |
| M6 Combat and movement | In progress | `lockstep-combat`: damage order, hit shapes, knockback, actions, dodges | Pending: projectiles, movement, consumers, duel fixture hash, 200-fighter spike |
| M7 Inventory | Not started | | |
| M8 Agents | Not started | | |
| M9 Knowledge | Not started | | |
| M10 Progression | Not started | | |
| M11 Relations | Not started | | |
| M12 Crafting | Not started | | |
| M13 Regions and calendar | Not started | | |
| M14 Structures and procgen | Not started | | |
| M15 Meta, lineage, templates | Not started | | |
| M16 Hardening | Not started | Saves, migrations, fuzzing and Miri, money are framework items; the full host, sprites and map plugin are out of scope here | |

## Gaps: short of the reference

| | Gap | Where | Status |
|---|---|---|---|
| G1 | Hexagon distance is 15.47 percent off true distance; the reference asks for 3 percent, which no hexagon step count can meet | Decision 0005 | Open: needs the reference's figure revised |
| G2 | 500 paths on a 512 by 512 map take about 611 milliseconds serially (88 with the path batch on ten threads); the reference asks for 50 | Decision 0005, 0006 | Open: needs a faster search (hierarchical paths, or bounded search) or a revised target |
| G3 | `bisect` narrows to the checkpoint, not the step, unless the recording checkpoints every step | Decision 0011 | Accepted: the recording has no hash between checkpoints |
| G4 | Only the ledger can be recorded by `lockstep-headless record` | `crates/lockstep-headless/src/recordings.rs` | Open: add other simulations as they gain consumers |
| G5 | The 200-fighter spike and the duel fixture hash | M6 gate | Pending in M6 |

## Deferrals: built later, when a consumer pulls them

| | Deferral | Waits for | Record |
|---|---|---|---|
| D1 | Schema export from `#[derive(Message)]` | A tool that reads schemas | Decision 0010 |
| D2 | The crash bundle (`recording`, `last-good` save, host log) | A host, which writes it | M5 notes in decision 0011 |
| D3 | `replay --save` and saving mid-recording | Saves and migrations (M16) | Decision 0011 |
| D4 | Conversions between message versions | Saves (M16): until a save ships, no version has shipped | Capsule and ledger notes |

## Departures: where the work differs from the reference on purpose

| | Departure | Record |
|---|---|---|
| P1 | Game time is an exact integer position, not an accumulated float | Decision 0001 |
| P2 | Entities live in a custom `StableVector` that reuses the lowest free slot, not a slot map | Decision 0002 |
| P3 | The core crate is `lockstep-core`; the spatial crate is `lockstep-spatial` (the reference says `lockstep-grid`) | Decision 0003 |
| P4 | A path batch may run on a thread pool, the one place threads are allowed | Decision 0006 |
| P5 | A ratchet attribute raises its minimum to each threshold it rises through | Decision 0008 |
| P6 | Thresholds fire in the order the value passes them, not definition order | Decision 0008 |
| P7 | `EffectContext` bundles the effect call arguments; `tick` takes the game day | Decision 0009 |
| P8 | A recording may keep snapshots, so `bisect` can show what differs | Decision 0011 |
| P9 | `Defence` carries evasion and block; `Reach` stops at the first body and `Line` pierces | Decision 0013 |
| P10 | Combat phases count whole steps; the counter window runs only while the fighter is free | Decision 0014 |
| P11 | The perfect-dodge cases are 3 and 4 steps at 30 a second, the nearest whole steps to the reference's 0.11 and 0.13 seconds | Decision 0014 |
| P12 | Consumers that are not games are preferred; game-named consumers use neutral names | Owner's instruction |
