//! Transient combat art follows actual simulation events, outside saved state.

use crate::session::Session;
use std::collections::BTreeMap;
use yr_render::sprites::SpriteBank;
use yr_sim::{Event, Fix};

const MAX_EFFECTS: usize = 256;
const MAX_TRAILS: usize = 512;

pub struct Trail {
    pub from: (Fix, Fix),
    pub to: (Fix, Fix),
    pub born: u64,
    pub style: yr_render::LineTrail,
}

impl Trail {
    pub fn color(&self, tick: u64) -> Option<[u8; 3]> {
        self.style.color_at(tick.checked_sub(self.born)?)
    }
}

#[derive(Clone)]
struct Entity {
    kind: String,
    x: Fix,
    y: Fix,
    facing: u8,
    owner: u8,
    foundation: Option<Foundation>,
}

/// Retained structure bounds keep its combat art above its own body while
/// still allowing foreground neighbours to occlude it after destruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Foundation {
    pub cell: (u16, u16),
    pub size: (u16, u16),
}

pub struct Effect {
    pub name: String,
    pub x: Fix,
    pub y: Fix,
    pub born: u64,
    pub foundation: Option<Foundation>,
}

pub struct InfantryDeath {
    pub kind: String,
    pub sequence: u8,
    pub facing: u8,
    pub owner: u8,
    pub x: Fix,
    pub y: Fix,
    pub born: u64,
}

#[derive(Default)]
pub struct EffectScene {
    entities: BTreeMap<u32, Entity>,
    pub active: Vec<Effect>,
    pub deaths: Vec<InfantryDeath>,
    pub trails: Vec<Trail>,
}

impl EffectScene {
    pub fn remember(&mut self, session: &Session) {
        self.entities = session
            .sim
            .units()
            .iter()
            .map(|u| (u.id, u.kind, u.x, u.y, u.facing, u.owner, None))
            .chain(session.sim.buildings().iter().map(|b| {
                let (x, y) = b.centre();
                (
                    b.id,
                    b.kind,
                    x,
                    y,
                    0,
                    b.owner,
                    Some(Foundation {
                        cell: b.cell,
                        size: b.size,
                    }),
                )
            }))
            .filter_map(|(id, kind, x, y, facing, owner, foundation)| {
                Some((
                    id,
                    Entity {
                        kind: session.kinds.get(usize::from(kind))?.clone(),
                        x,
                        y,
                        facing,
                        owner,
                        foundation,
                    },
                ))
            })
            .collect();
    }

    pub fn update(&mut self, session: &Session, events: &[Event], bank: &mut SpriteBank) {
        let tick = session.sim.tick();
        self.trails.retain(|t| t.color(tick).is_some());
        self.active
            .retain(|e| tick >= e.born && bank.effect_alive(&e.name, tick - e.born));
        self.deaths.retain(|d| {
            tick >= d.born && bank.infantry_death_alive(&d.kind, d.sequence, tick - d.born)
        });
        // Current poses win for survivors; previous poses preserve entities
        // which fire and die in the same tick. Refresh the table afterwards.
        let mut entities = self.entities.clone();
        self.remember(session);
        entities.extend(self.entities.iter().map(|(&id, e)| (id, e.clone())));
        let mut fatal_hits = BTreeMap::new();
        for event in events {
            if let Event::ProjectileMoved { from, to, origin } = *event {
                if from != to {
                    if let Some(style) = origin
                        .and_then(|o| o.weapon)
                        .and_then(|source| bank.projectile_line_trail(source))
                    {
                        if self.trails.len() == MAX_TRAILS {
                            self.trails.remove(0);
                        }
                        self.trails.push(Trail {
                            from,
                            to,
                            born: tick,
                            style,
                        });
                    }
                }
                continue;
            }
            if let Event::FatalHit {
                target,
                kind,
                owner,
                facing,
                x,
                y,
                origin,
            } = *event
            {
                if let Some(kind) = session.kinds.get(usize::from(kind)) {
                    fatal_hits.entry(target).or_insert((
                        Entity {
                            kind: kind.clone(),
                            x,
                            y,
                            facing,
                            owner,
                            foundation: None,
                        },
                        origin,
                    ));
                }
                continue;
            }
            if let Event::Impact {
                target,
                x,
                y,
                origin: Some(origin),
                ..
            } = *event
            {
                if let Some(name) = origin
                    .weapon
                    .and_then(|weapon| bank.impact_effect(weapon, origin.damage))
                {
                    self.push(
                        bank,
                        Effect {
                            name,
                            x,
                            y,
                            born: tick,
                            foundation: entities.get(&target).and_then(|e| e.foundation),
                        },
                    );
                }
                continue;
            }
            let (id, firing) = match *event {
                Event::Fired { shooter, .. } => (shooter, true),
                Event::Died { id } => (id, false),
                _ => continue,
            };
            if !firing {
                if let Some((victim, sequence)) =
                    fatal_hits.remove(&id).and_then(|(victim, origin)| {
                        let source = origin?.weapon?;
                        let sequence = bank.infantry_death_sequence(&victim.kind, source)?;
                        Some((victim, sequence))
                    })
                {
                    if self.deaths.len() == MAX_EFFECTS {
                        self.deaths.remove(0);
                    }
                    self.deaths.push(InfantryDeath {
                        kind: victim.kind,
                        sequence,
                        facing: victim.facing,
                        owner: victim.owner,
                        x: victim.x,
                        y: victim.y,
                        born: tick,
                    });
                    continue;
                }
            }
            let Some(entity) = entities.get(&id) else {
                continue;
            };
            let name = if firing {
                bank.muzzle_effect(&entity.kind, entity.facing)
            } else {
                bank.destruction_effect(&entity.kind)
            };
            if let Some(name) = name {
                self.push(
                    bank,
                    Effect {
                        name,
                        x: entity.x,
                        y: entity.y,
                        born: tick,
                        foundation: entity.foundation,
                    },
                );
            }
        }
    }

    fn push(&mut self, bank: &mut SpriteBank, effect: Effect) {
        if !bank.effect_alive(&effect.name, 0) {
            return;
        }
        if self.active.len() == MAX_EFFECTS {
            self.active.remove(0);
        }
        self.active.push(effect);
    }
}

#[cfg(test)]
mod tests;
