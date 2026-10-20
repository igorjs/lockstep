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
| M3 Grid and math | Done, one open item | `lockstep-spatial`: topologies, map, occupancy, A*, hierarchical search, path batch, flow field, line of sight; `Fixed32`, `Vector2`, angles; benchmark; mars-rovers, crowd | Tie-breaking holds; isotropy holds for squares, not for hexagons (departure P48, pinned by a test); 500 paths in 23 milliseconds through the hierarchy, not by the exact search (gap G2); baseline committed |
| M4 Attributes | Done | `Chance` and three rolls; `lockstep-attributes`: registry from JSON, modifiers, thresholds, derived curves, effects with four stacking rules; exact simulated minutes; capsule needs, bleeding and prayer as data; drone-fleet | Met: stacking, crossing, frame-rate independence tests green |
| M5 Timeline, replay, derive | Done, three deferrals (D1 to D3) | `#[derive(Message)]` and `Indexable`; recorder, replay, bisect; timeline with compaction; ledger journal; headless `record`, `replay`, `bisect`, `stats` | Met: a planted rule change is found at the right checkpoint |
| M6 Combat and movement | Done | `lockstep-combat`: damage order, hit shapes, knockback, actions, dodges, projectiles, movement; sparring; rovers ram | Met: the duel fixture hash is equal natively and under WebAssembly; 200 partners spar for 10,000 steps without sharing a cell or going below zero health |
| M7 Inventory | Done | `lockstep-inventory`: catalogue from JSON, items as entities, containers, stacking, equipment with modifiers, binding affixes, spoilage by temperature; cargo-bay | Met: the same simulated minutes spoil on the same minute at 30 and 60 steps a second, natively and under WebAssembly |
| M8 Agents | Done | `lockstep-agents`: sight cones with line of sight, staggered checks, noise carried by the wind, hearing thresholds, psychic noise; memory that fades, alert states, the director's budget, leashes; waves that space a large arrival into pulses by simulated minutes within that budget (decisions 0055, 0056; checkout); steering by flow field with the shared sidestep; utility decisions and delegation; weather with drift, gusts and fronts; crater-survey; benchmark baseline for 1,000 agents | Met: the budget is never exceeded and leashed rovers never leave their crater over the crater-survey fixture; the wind ranges are exact; a day of weather from seed 42 hashes the same under WebAssembly; the checkout queue never passes the budget over its session, and its hash is equal natively and under WebAssembly |
| M9 Knowledge | Done | `lockstep-knowledge`: facts from fragments of distinct sources, questions, rules over data predicates that fire once, event counts kept in state; audit | Met: the first question opens on its second fragment; nothing fires twice across a save; the predicate hash is equal natively and under WebAssembly |
| M10 Progression | Done | `lockstep-progression`: node graph from JSON with costs, requirements, exclusions both ways, attribute gates, keystones, refunds and modifiers; points by named triggers; training | Met: across the training session no technician holds both sides of an exclusion, every gated node was taken with its gate met, and the keystone never comes back |
| M11 Relations | Done | `lockstep-relations`: directional standings toward entities and groups, bounds, thresholds crossed in passing order, exact decay toward rest; dispatch | Met: across the dispatch session every refusal naming trust comes while trust is below the threshold, and a driver who has lost trust refuses saying so |
| M12 Crafting | Done | `lockstep-crafting`: recipes from JSON with stations, inputs, minutes and weighted outcome tables with failure branches; atomic consumption; outcomes from a named stream; bakery | Met: across the bakery session the pantry changes only by deliveries and whole recipes; 2,000 bakes land within 3 percent of the outcome table |
| M13 Regions and calendar | Done | `lockstep-regions`: region graph from JSON with anchors, receptiveness, hops and nearest, a condition that spreads by chance each interval helped downwind and scaled by a pace (decisions 0029, 0030); `lockstep-calendar`: seasons from JSON, daylight and temperature by day and minute, the clock's daylight, season events (decision 0027); orchard | Met: the mean spread time is within 3 percent of the inverse of the chance; winter has the shortest days and a midwinter blight does not spread; the calendar, spread and orchard hashes are equal natively and under WebAssembly |
| M14 Structures and procedural generation | Done | `lockstep-structures`: durable multi-cell structures that wall a map, wear down, mend and collapse; `lockstep-procgen`: chunk templates and weighted pools from JSON, reserved chunks, a layout rolled with a seed per chunk, applied to a map; estate | Met: a barrier across a chunk border and its collapse each report both chunks dirty; chunk seeds never collide across 200 world seeds times 128 by 128 places. Deferral D6 is closed by decision 0051: `roll_matched` joins template edges by socket, opt-in, bounded, with `NoLayout` and `SearchLimit` refusals, checked on the estate's joined templates |
| M15 Meta, lineage, templates | Done | `lockstep-meta`: the chronicle (the cross-run record) saved in a versioned envelope, control transfer with a caller-ordered fall-back, archetypes and point-buy from JSON (decision 0031); roster | Met: the chronicle survives a new project. A name retired in the first project of `roster` is refused in the second after the chronicle was sealed and opened, the lead falls back to the next hired and never to the one who left, and the roster hash is equal natively and under WebAssembly (`8969859445141e9b`) |
| M16 Hardening | Done | Done: saves with a versioned envelope, migrations between snapshot versions, `Recorder::save` and `replay_from`, headless `replay --until --save` and `resume`, and the `tariffs` consumer with its committed old save (decisions 0035, 0036); `lockstep-tables`: data tables from JSON checked whole and swapped by an intent, with the `rates` consumer (decisions 0037, 0038). `Money` and `Currency` with largest remainder sharing, used by the ledger's shared bills (decision 0040); `lockstep-fuzz`, a deterministic fuzz harness over the decoders, nine JSON loaders and `Fixed32` and `Money` arithmetic with a committed corpus, `just fuzz` for long runs (decision 0039); `just miri` and an advisory Miri job (decision 0041); the day-length cap and save checks the fuzzer found (decision 0042) | Met: every framework part has landed; the long fuzz run is manual and Miri is advisory (gap G5) | Open until every framework part lands |

## Gaps: short of the reference

| | Gap | Where | Status |
|---|---|---|---|
| G2 | The exact search takes about 611 milliseconds for 500 paths on a 512 by 512 map with 20 percent walls (88 with the path batch on ten threads); the reference asks for 50. Measured: 23 milliseconds through the hierarchical search (decision 0047), with paths 6.4 percent dearer in total and a 230 millisecond build per map; exact searches measured and rejected: tighter state, no division, depth tie break, landmarks (1.4 times fewer expansions) | Decisions 0005, 0006, 0047 | Open for exact paths (about 12 times over); the target is met only by the approximate search (departure P47) |
| G3 | `bisect` narrows to the checkpoint, not the step, unless the recording checkpoints every step | Decision 0011 | Accepted: the recording has no hash between checkpoints |
| G4 | `lockstep-headless record` cannot record `roster`: its second project starts from a chronicle folded from the first project's events, which a replay does not rebuild (`mars-rovers` and `orchard` record since decisions 0060 and 0061) | Decision 0043, `crates/lockstep-headless/src/recordings.rs` | Accepted: every other example records |
| G5 | The fuzz harness is not coverage guided, the long run is manual (`just fuzz`), and Miri is advisory | Decisions 0039, 0041 | Accepted: revisit if a coverage-guided fuzzer finds what it misses, or when unsafe code appears |

## Deferrals: built later, when a consumer pulls them

| | Deferral | Milestone | Waits for | Record |
|---|---|---|---|---|
| D1 | Schema export from `#[derive(Message)]` | M5 | Closed in M16: `Message::SCHEMA`, `lockstep-headless schema`, saved schemas for `bakery` and `tariffs`; a change inside a nested type that is not a derived message is not seen | Decisions 0062, 0063 |
| D2 | The crash bundle (a recording, the last good save and the host's log) | M5 | A host, which writes it | Decision 0011 |
| D3 | `replay --save` and saving mid-recording | M5 | Closed in M16: `replay --until --save`, `resume`, `Recorder::save` | Decision 0036, `docs/lockstep-headless.md` |
| D4 | Conversions between message versions | M16 | Closed in M16 for snapshots (decision 0035); see D8 for the rest | Decisions 0010, 0035 |
| D7 | Conversions between versions of a saved chronicle: the envelope refuses another message version and does not convert it | M15 | Saves and migrations (M16), as D4 | Decision 0031, `docs/lockstep-meta.md` |
| D8 | Conversions for intents, events and configurations, so recordings made by an older shape still replay | M16 | A consumer that keeps old recordings | Decision 0035 |

## Departures: where the work differs from the reference on purpose

| | Departure | Record |
|---|---|---|
| P1 | Simulated time is an exact integer position, not an accumulated float, and the runner hash covers that position instead of the float minute | Decision 0001 |
| P2 | Entities live in a custom `StableVector` that reuses the lowest free slot, not a slot map | Decision 0002 |
| P3 | The core crate is `lockstep-core`; the spatial crate is `lockstep-spatial` (the reference says `lockstep-grid`) | Decisions 0003, 0005 |
| P13 | `abs` and negation wrap instead of panicking, `Vector2::length` works in 64 bits, and `from_ratio` rounds ties away from zero | Decision 0004 |
| P47 | Long paths on large maps may use a hierarchical search (`Hierarchy`, `HierarchicalPathfinder`) whose paths are not always the cheapest: 6.4 percent dearer in total on the benchmark map, 13.2 percent on hilly muddy maps, in return for 23 instead of 611 milliseconds. `Pathfinder` stays exact and the default, so no existing path or hash changed | Decision 0047, gap G2 |
| P48 | Hexagon distance is 15.47 percent off true distance, not the reference's 3 percent: no hexagon step count can meet it, and the best constant factor still leaves 7.2 percent. Kept as the cube distance | Decision 0048 |
| P4 | A path batch may run on a thread pool, the one place threads are allowed | Decision 0006 |
| P5 | A ratchet attribute raises its minimum to each threshold it rises through | Decision 0008 |
| P6 | Thresholds fire in the order the value passes them, not definition order | Decision 0008 |
| P7 | `EffectContext` bundles the effect call arguments; `tick` takes the simulated day | Decision 0009 |
| P8 | A recording may keep snapshots, so `bisect` can show what differs | Decision 0011 |
| P9 | `Defence` carries evasion and block; `Reach` stops at the first body and `Line` pierces | Decision 0013 |
| P10 | Combat phases count whole steps; the counter window runs only while the fighter is free | Decision 0014 |
| P14 | Projectiles fly at an absolute altitude instead of checking `can_step`, and a dodged projectile flies on | Decision 0015 |
| P11 | The perfect window is 0.12 seconds; its cases are 3 steps (0.100 seconds, perfect) and 4 (0.133, not), the whole steps either side of the reference's 0.11 and 0.13 second cases at 30 a second | Decision 0014 |
| P15 | The reference's single-actor-against-an-opponent consumer is the `sparring` example, two neutral partners, so the `capsule` example stays free of combat | `docs/sparring.md` |
| P16 | The reference's inventory consumers (a ration and a tool for one actor, and a Mars cargo bay) are one non-game `cargo-bay` example | `docs/cargo-bay.md` |
| P17 | "A gale masks walking" is read as a 14 metre a second gale cutting a 6 metre footstep to 1.8 metres across the wind and nothing against it; no single threshold silences footsteps from 4 to 9 metres | Decision 0018 |
| P18 | The weather (drift, gusts, fronts) lives in `lockstep-agents` beside the wind, not in the regions crate; regions take the wind as a plain `Breeze`. Kept when regions arrived in M13 | Decisions 0022, 0029 |
| P19 | The reference's agent consumers (one actor's perception of an opponent, and a rover that hears a landing unless leashed) are one non-game `crater-survey` example | `docs/crater-survey.md` |
| P20 | The reference's knowledge consumer (a question ledger for one actor) is a non-game `audit` example; and `happened` counts events the simulation feeds as they happen, not the timeline, which sits outside saved state | `docs/audit.md`, decision 0023 |
| P21 | The reference's progression consumer (a six-node web with one keystone) is a non-game `training` example | `docs/training.md` |
| P22 | The reference's relations consumer (an assistant refusing a task below a trust threshold) is a non-game `dispatch` example, using the delegation from `lockstep-agents` | `docs/dispatch.md` |
| P23 | The reference's crafting consumer (a reforge with a gamble) is a non-game `bakery` example | `docs/bakery.md` |
| P24 | The reference's barrier and generated-layout consumers are one non-game `estate` example: a flood barrier and a garden estate rolled from chunk templates | `docs/estate.md`, decision 0028 |
| P25 | The reference's regions consumer (a zone that spreads while winter shortens the days) is a non-game `orchard` example: a blight over orchard blocks whose pace follows the calendar's temperature | `docs/orchard.md` |
| P26 | The reference's lineage consumer (control falling back to the next in line, with a name that stays retired across runs) is a non-game `roster` example: a crew across successive projects | `docs/roster.md`, decision 0031 |
| P27 | The reference's saves consumer (an old save loaded after a rules change) is a non-game `tariffs` example: a duty schedule whose committed version 1 save migrates to version 2 | `docs/tariffs.md`, decision 0035 |
| P28 | Prose, comments and test names use neutral engine terms (actor, simulated minute); the capsule example's own names (accessor, field, script word) were renamed on purpose; these keep their names because renaming them would move a fixture hash or break the public interface: the `"loot"` and `"valley"` random stream names, the `game_minutes` identifiers, the `Wound`, `Bandage` and `Pray` intents of `capsule`, and its actor name | This page |
