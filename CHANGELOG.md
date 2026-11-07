<!-- SPDX-License-Identifier: Apache-2.0 -->

# Changelog

All notable changes to Lockstep, one entry per milestone, then the work that came after the
milestones. The status of each milestone and every open gap is in `docs/roadmap.md`. Each entry
ends with the decision records in `docs/decisions` that explain its choices;
`scripts/check-changelog.sh` fails when an entry names none.

## 0.1.0 (not released)

No tag has been pushed and no crate has been published. `docs/release.md` describes the steps and
what `just release-check` proves.

### M1 Core

The workspace, continuous integration on three platforms, `Simulation`, `Runner`, `Clock`, the
store of entities, named random streams, hashing and `Message`, with the capsule and ledger
examples. Simulated time is an exact integer position, entities live in a stable vector, and the
crate names are fixed.

Decisions 0001, 0002, 0003.

### M2 Host

Out of scope for this repository: the host lives elsewhere. The clock decision is the contract a
host drives.

Decisions 0001.

### M3 Grid and math

`lockstep-spatial` with topologies, map, occupancy, A*, hierarchical search, path batch, flow
field and line of sight, plus `Fixed32`, `Vector2` and angles, a benchmark, and the mars-rovers
and crowd examples. Hexagon distance error and the one thread pool are recorded departures.

Decisions 0004, 0005, 0006, 0047, 0048.

### M4 Attributes

`Chance` and three rolls, and `lockstep-attributes`: registry from JSON, modifiers, thresholds,
derived curves and effects with four stacking rules, with the drone-fleet example.

Decisions 0007, 0008, 0009.

### M5 Timeline, replay, derive

`#[derive(Message)]` and `Indexable`, the recorder, replay and bisect, the timeline with
compaction, the ledger journal, and the headless `record`, `replay`, `bisect` and `stats`
commands.

Decisions 0010, 0011, 0012.

### M6 Combat and movement

`lockstep-combat`: damage order, hit shapes, knockback, actions, dodges, projectiles and
movement, with the sparring example.

Decisions 0013, 0014, 0015, 0016.

### M7 Inventory

`lockstep-inventory`: catalogue from JSON, items as entities, containers, stacking, equipment,
affixes and spoilage by temperature, with the cargo-bay example.

Decisions 0017.

### M8 Agents

`lockstep-agents`: sight, noise carried by the wind, hearing, memory, alert states, the
director's budget, leashes, waves in pulses, steering, utility decisions, delegation and weather,
with the crater-survey and checkout examples.

Decisions 0018, 0019, 0020, 0021, 0022, 0055, 0056.

### M9 Knowledge

`lockstep-knowledge`: facts from fragments of distinct sources, questions and rules that fire
once, with the audit example.

Decisions 0023.

### M10 Progression

`lockstep-progression`: a node graph from JSON with costs, requirements, exclusions, gates,
keystones and refunds, with the training example.

Decisions 0024.

### M11 Relations

`lockstep-relations`: directional standings, bounds, thresholds and exact decay, with the
dispatch example.

Decisions 0025.

### M12 Crafting

`lockstep-crafting`: recipes from JSON with stations, inputs, minutes and weighted outcome
tables, with the bakery example.

Decisions 0026.

### M13 Regions and calendar

`lockstep-regions` with a spreading condition and `lockstep-calendar` with seasons, daylight and
temperature, with the orchard example. Weather stays with the agents crate.

Decisions 0027, 0029, 0030, 0060.

### M14 Structures and procedural generation

`lockstep-structures` and `lockstep-procgen`, with the estate example. Template edges join by
socket, opt in and bounded.

Decisions 0028, 0051.

### M15 Meta, lineage, templates

`lockstep-meta`: the chronicle in a versioned envelope, control transfer with a fall back,
archetypes and point buy from JSON, with the roster example.

Decisions 0031, 0067, 0068.

### M16 Hardening

Saves with a versioned envelope and migrations, resume from a save, `lockstep-tables` with the
rates example, `Money` with largest remainder sharing, the deterministic fuzz harness, Miri as an
advisory job and the day length cap.

Decisions 0035, 0036, 0037, 0038, 0039, 0040, 0041, 0042.

## After the milestones

### Recordable examples and schemas

Every example with one session records, a fixture of many sessions records as a set, messages
export a schema, a changed shape needs a new version, and renames that move hashes are named.

Decisions 0043, 0061, 0062, 0063, 0071.

### Exact jump search

An exact opt-in jump point search and its batch, a bucket open set that did not pay, and a
recorded no-go for an exact precomputed abstraction (gap G2 in the roadmap).

Decisions 0075, 0076, 0077, 0081, 0082.

### Exact bisect step

A hash at every step with coarse snapshots, so `bisect` names the exact step (gap G3).

Decisions 0079.

### Guided fuzzing

A coverage guided fuzz wrapper and an advisory job (gap G5).

Decisions 0080.

### Conversions

A saved chronicle converts forward through a chain (D7), and a recording names its shapes and
converts forward (D8).

Decisions 0084, 0085.
