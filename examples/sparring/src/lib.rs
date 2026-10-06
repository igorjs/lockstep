// SPDX-License-Identifier: Apache-2.0
//! Sparring: the consumer scenario for `lockstep-combat`.
//!
//! Partners spar in pairs on one floor with a few pillars. Each has a stick (a quick jab at the
//! cell ahead), a sweep (a slower arc that knocks back a cell), a dodge, and a soft ball to throw,
//! and walks or runs on the shared stamina pool. A partner knocked into a pillar or the edge of
//! the floor takes impact damage. A partner whose health reaches zero yields: the point goes to
//! the other, and the health comes back in full.
//!
//! Each step runs, in order: the intents, combat, movement (a partner that is not ready stands
//! still), the thrown balls, then the damage.

use lockstep_combat::{
    direction, step_combat, step_movement, step_projectiles, ActionDefinition, ActionId,
    CombatEvent, DamageKind, DamagePacket, Defence, DodgeDefinition, Fighter, Gait, Hit, HitShape,
    Impact, InterruptMask, Launch, MoveOrder, MovementEvent, MovementRules, MovementWorld, Mover,
    Moveset, MovesetId, Order, Phase, Projectile, Tags,
};
use lockstep_core::math::{Fixed32, Turn};
use lockstep_core::{
    Chance, ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_spatial::{Cell, GridMap, Occupancy, Pathfinder, Square8};
use serde::{Deserialize, Serialize};

/// The runner's default step rate.
pub const STEPS_PER_SECOND: u32 = 30;
pub const STICK: ActionId = ActionId(0);
pub const SWEEP: ActionId = ActionId(1);
pub const HEALTH: i32 = 100;
pub const STAMINA: i32 = 100;
/// Damage from being knocked into a pillar or the edge of the floor.
pub const IMPACT_DAMAGE: i32 = 5;
pub const THROW_STAMINA: i32 = 8;
/// The coach backs off from its partner below this stamina.
pub const RETREAT_BELOW: i32 = 30;

fn seconds(hundredths: i32) -> Fixed32 {
    Fixed32::from_ratio(hundredths, 100)
}

fn packet(amount: i32, stagger: i32, knockback: u8) -> DamagePacket {
    DamagePacket {
        amount: Fixed32::from_int(amount),
        kind: DamageKind::Blunt,
        knockback,
        stagger: Fixed32::from_int(stagger),
        critical: Chance(1_000),
        critical_multiplier: Fixed32::from_ratio(3, 2),
        tags: Tags::NONE,
    }
}

/// The one moveset every partner uses.
pub fn moveset() -> Moveset {
    Moveset {
        actions: vec![
            ActionDefinition {
                windup_seconds: seconds(20),
                active_seconds: seconds(10),
                recovery_seconds: seconds(30),
                shape: HitShape::Adjacent,
                stamina_cost: Fixed32::from_int(8),
                damage: packet(8, 5, 0),
                interruptible_by: InterruptMask::STAGGER,
            },
            ActionDefinition {
                windup_seconds: seconds(45),
                active_seconds: seconds(15),
                recovery_seconds: seconds(50),
                shape: HitShape::Arc {
                    radius: 1,
                    half_angle: 8_192,
                },
                stamina_cost: Fixed32::from_int(20),
                damage: packet(18, 40, 1),
                interruptible_by: InterruptMask::NONE,
            },
        ],
        dodge: DodgeDefinition {
            startup_seconds: seconds(5),
            invulnerable_seconds: seconds(30),
            recovery_seconds: seconds(20),
            distance_cells: 2,
            stamina_cost: Fixed32::from_int(15),
            cooldown_seconds: seconds(80),
            perfect_window_seconds: seconds(12),
            counter_seconds: seconds(50),
        },
    }
}

/// The soft ball: six cells at two a step, waist high.
pub fn ball() -> Launch {
    Launch {
        range_cells: 6,
        cells_per_step: 2,
        damage: Some(packet(4, 0, 0)),
        loudness: 0,
        height: 1,
        descent_every: 0,
    }
}

fn defence() -> Defence {
    Defence {
        armour: Fixed32::from_int(1),
        poise: Fixed32::from_int(10),
        evasion: Chance(500),
        ..Defence::default()
    }
}

/// A floor with pillars, and partners on their starting cells. Partners spar in pairs in the
/// order listed: the first with the second, the third with the fourth.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    pub width: u32,
    pub height: u32,
    pub pillars: Vec<(u32, u32)>,
    pub partners: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    Move {
        who: Handle,
        x: u32,
        y: u32,
        gait: Gait,
    },
    Stop {
        who: Handle,
    },
    Attack {
        who: Handle,
        action: ActionId,
        facing: Turn,
    },
    Dodge {
        who: Handle,
        heading: Turn,
    },
    /// Throws the ball, when the thrower is ready and has the stamina.
    Throw {
        who: Handle,
        facing: Turn,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Combat(CombatEvent),
    Movement(MovementEvent),
    Threw {
        who: Handle,
    },
    /// The thrower was busy, tired or unknown.
    ThrowRefused {
        who: Handle,
    },
    Hurt {
        who: Handle,
        by: Handle,
        amount: Fixed32,
        health: Fixed32,
    },
    /// Knocked into a pillar or the edge of the floor.
    Impact {
        who: Handle,
        health: Fixed32,
    },
    /// Health reached zero: the point goes to `to`, and the health comes back in full.
    Yielded {
        who: Handle,
        to: Handle,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub map: GridMap<Square8>,
    pub occupancy: Occupancy,
    pub partners: StableVector<()>,
    pub fighters: Column<Fighter>,
    pub movers: Column<Mover>,
    pub health: Column<Fixed32>,
    /// Points won: bouts the partner made the other yield.
    pub points: Column<u32>,
    pub balls: StableVector<Projectile>,
}

pub struct Sparring {
    world: World,
    movesets: Vec<Moveset>,
    rules: MovementRules,
    pathfinder: Pathfinder,
}

impl Sparring {
    fn with_world(world: World) -> Self {
        let pathfinder = Pathfinder::new(&world.map);
        Sparring {
            world,
            movesets: vec![moveset()],
            rules: MovementRules::default(),
            pathfinder,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// Every partner, in handle order.
    pub fn partners(&self) -> Vec<Handle> {
        self.world.partners.handles()
    }

    pub fn cell_of(&self, who: Handle) -> Option<Cell> {
        self.world.occupancy.cell_of(who)
    }

    pub fn health_of(&self, who: Handle) -> Option<Fixed32> {
        self.world.health.get(who).copied()
    }

    pub fn fighter(&self, who: Handle) -> Option<&Fighter> {
        self.world.fighters.get(who)
    }

    fn throw(&mut self, who: Handle, facing: Turn, events: &mut Vec<Event>) {
        let cost = Fixed32::from_int(THROW_STAMINA);
        let cell = self.world.occupancy.cell_of(who);
        let fighter = self.world.fighters.get_mut(who);
        let (Some(cell), Some(fighter)) = (cell, fighter) else {
            events.push(Event::ThrowRefused { who });
            return;
        };
        if fighter.phase != Phase::Ready || fighter.stamina < cost {
            events.push(Event::ThrowRefused { who });
            return;
        }
        fighter.stamina -= cost;
        fighter.facing = facing;
        let ball = Projectile::launch(&self.world.map, who, cell, facing, &ball());
        self.world.balls.insert(ball);
        events.push(Event::Threw { who });
    }

    fn hurt(&mut self, hit: &Hit, by: Handle, events: &mut Vec<Event>) {
        let who = hit.target;
        let Some(health) = self.world.health.get_mut(who) else {
            return;
        };
        let amount = hit.result.dealt;
        if amount > Fixed32::ZERO {
            *health = (*health - amount).max(Fixed32::ZERO);
            events.push(Event::Hurt {
                who,
                by,
                amount,
                health: *health,
            });
        }
        if hit
            .knocked
            .is_some_and(|knocked| knocked.impact == Some(Impact::Wall))
        {
            *health = (*health - Fixed32::from_int(IMPACT_DAMAGE)).max(Fixed32::ZERO);
            events.push(Event::Impact {
                who,
                health: *health,
            });
        }
        if *health == Fixed32::ZERO {
            *health = Fixed32::from_int(HEALTH);
            let points = self.world.points.get(by).copied().unwrap_or(0);
            self.world.points.set(by, points + 1);
            events.push(Event::Yielded { who, to: by });
        }
    }
}

impl Simulation for Sparring {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let mut map: GridMap<Square8> = GridMap::new(configuration.width, configuration.height);
        for (x, y) in &configuration.pillars {
            map.set_passable(map.index(*x, *y), false);
        }
        let mut world = World {
            occupancy: Occupancy::new(&map),
            map,
            partners: StableVector::new(),
            fighters: Column::new(),
            movers: Column::new(),
            health: Column::new(),
            points: Column::new(),
            balls: StableVector::new(),
        };
        for (x, y) in &configuration.partners {
            let who = world.partners.insert(());
            let cell = world.map.index(*x, *y);
            world
                .occupancy
                .place(cell, who)
                .expect("partners start on distinct cells");
            world.fighters.set(
                who,
                Fighter::new(MovesetId(0), Fixed32::from_int(STAMINA), defence()),
            );
            world.movers.set(who, Mover::default());
            world.health.set(who, Fixed32::from_int(HEALTH));
        }
        Sparring::with_world(world)
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut events = Vec::new();
        let mut combat_orders = Vec::new();
        let mut move_orders = Vec::new();
        for intent in intents {
            match intent {
                Intent::Move { who, x, y, gait } => {
                    if self.world.map.contains(*x, *y) {
                        move_orders.push((
                            *who,
                            MoveOrder::MoveTo {
                                target: self.world.map.index(*x, *y),
                                gait: *gait,
                            },
                        ));
                    }
                }
                Intent::Stop { who } => move_orders.push((*who, MoveOrder::Stop)),
                Intent::Attack {
                    who,
                    action,
                    facing,
                } => combat_orders.push((
                    *who,
                    Order::Attack {
                        action: *action,
                        facing: *facing,
                    },
                )),
                Intent::Dodge { who, heading } => {
                    combat_orders.push((*who, Order::Dodge { heading: *heading }))
                }
                Intent::Throw { who, facing } => self.throw(*who, *facing, &mut events),
            }
        }

        let mut combat_events = Vec::new();
        step_combat(
            &mut self.world.fighters,
            &self.movesets,
            &self.world.map,
            &mut self.world.occupancy,
            &combat_orders,
            STEPS_PER_SECOND,
            context.randomness,
            &mut combat_events,
        );

        // A partner that is attacking, dodging or staggered stands still.
        // The stop comes after any move order of this step, so it wins.
        for (who, fighter) in self.world.fighters.iter() {
            if fighter.phase != Phase::Ready {
                move_orders.push((who, MoveOrder::Stop));
            }
        }
        let mut movement_events = Vec::new();
        let stone = |_: Cell| Fixed32::from_int(6);
        let mut movement_world = MovementWorld {
            map: &self.world.map,
            occupancy: &mut self.world.occupancy,
            pathfinder: &mut self.pathfinder,
            rules: &self.rules,
            steps_per_second: STEPS_PER_SECOND,
            walking_noise: &stone,
        };
        step_movement(
            &mut self.world.movers,
            &mut self.world.fighters,
            &mut movement_world,
            &move_orders,
            &mut movement_events,
        );

        step_projectiles(
            &mut self.world.balls,
            &mut self.world.fighters,
            &self.movesets,
            &self.world.map,
            &mut self.world.occupancy,
            STEPS_PER_SECOND,
            context.randomness,
            &mut combat_events,
        );
        for event in &combat_events {
            match event {
                CombatEvent::Landed { who, hits, .. } => {
                    for hit in hits {
                        self.hurt(hit, *who, &mut events);
                    }
                }
                CombatEvent::ProjectileHit { by, hit, .. } => self.hurt(hit, *by, &mut events),
                _ => {}
            }
        }
        context
            .events
            .extend(combat_events.into_iter().map(Event::Combat));
        context
            .events
            .extend(movement_events.into_iter().map(Event::Movement));
        context.events.extend(events);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Sparring::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_007;
pub const DEFAULT_STEPS: u64 = 6_000;

/// The duel: two partners on a sixteen by nine floor with two pillars.
pub fn duel() -> Configuration {
    Configuration {
        width: 16,
        height: 9,
        pillars: vec![(7, 2), (8, 6)],
        partners: vec![(4, 4), (11, 4)],
    }
}

/// `pairs` pairs side by side in rows of ten, five cells apart, with a pillar beside every
/// fourth pair.
pub fn crowd(pairs: u32) -> Configuration {
    let rows = pairs.div_ceil(10);
    let mut partners = Vec::new();
    let mut pillars = Vec::new();
    for pair in 0..pairs {
        let (x, y) = (3 + (pair % 10) * 5, 3 + (pair / 10) * 5);
        partners.push((x, y));
        partners.push((x + 1, y));
        if pair % 4 == 3 {
            pillars.push((x + 3, y + 2));
        }
    }
    Configuration {
        width: 54,
        height: 3 + rows * 5,
        pillars,
        partners,
    }
}

pub fn runner(configuration: Configuration, seed: u64) -> Runner<Sparring> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 600,
            starting_day: 0,
        },
    )
}

/// One step of the scripted coach. Each ready partner closes on its pair; next to it, it
/// strikes, sweeps, dodges a wind-up it sees coming, or backs off when out of breath; from range
/// it now and then throws.
/// The coach draws from its own streams, so it depends only on the seed and the state.
pub fn coach(sparring: &Sparring, script: &mut Streams, step: u64) -> Vec<Intent> {
    let world = sparring.world();
    let partners = sparring.partners();
    let mut intents = Vec::new();
    for (index, who) in partners.iter().copied().enumerate() {
        let Some(other) = partners.get(index ^ 1).copied() else {
            continue;
        };
        let (Some(fighter), Some(rival)) = (world.fighters.get(who), world.fighters.get(other))
        else {
            continue;
        };
        let (Some(from), Some(to)) = (sparring.cell_of(who), sparring.cell_of(other)) else {
            continue;
        };
        if fighter.phase != Phase::Ready {
            continue;
        }
        let facing = direction(&world.map, from, to);
        let distance = world.map.steps(from, to);
        let roll = script.range("coach", 0, 100);
        if distance == 1 && fighter.stamina < Fixed32::from_int(RETREAT_BELOW) {
            // Out of breath: back off four cells to get it back.
            let ((x, y), (other_x, other_y)) =
                (world.map.coordinates(from), world.map.coordinates(to));
            let away = |mine: u32, theirs: u32, size: u32| {
                let target = mine as i64 + (mine as i64 - theirs as i64) * 4;
                target.clamp(1, size as i64 - 2) as u32
            };
            intents.push(Intent::Move {
                who,
                x: away(x, other_x, world.map.width()),
                y: away(y, other_y, world.map.height()),
                gait: Gait::Walk,
            });
            continue;
        }
        if distance == 1 {
            let threatened = matches!(rival.phase, Phase::Windup { .. });
            if threatened && roll < 25 {
                intents.push(Intent::Dodge {
                    who,
                    heading: facing.wrapping_add(32_768),
                });
            } else if roll < 6 {
                intents.push(Intent::Attack {
                    who,
                    action: STICK,
                    facing,
                });
            } else if roll < 8 {
                intents.push(Intent::Attack {
                    who,
                    action: SWEEP,
                    facing,
                });
            }
            continue;
        }
        if distance <= 6 && roll < 2 {
            intents.push(Intent::Throw { who, facing });
            continue;
        }
        let idle = world
            .movers
            .get(who)
            .is_some_and(|mover| mover.target.is_none());
        if idle || step % 20 == index as u64 % 20 {
            let (x, y) = world.map.coordinates(to);
            let gait = if distance > 6 { Gait::Run } else { Gait::Walk };
            intents.push(Intent::Move { who, x, y, gait });
        }
    }
    intents
}

/// The scripted session the fixture hash covers.
pub fn run_fixture(configuration: Configuration, seed: u64, steps: u64) -> Runner<Sparring> {
    let mut runner = runner(configuration, seed);
    let mut script = Streams::new(seed ^ 0x7370_6172);
    for step in 0..steps {
        let intents = coach(runner.simulation(), &mut script, step);
        runner.step_once(&intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(duel(), seed, steps).hash()
}
