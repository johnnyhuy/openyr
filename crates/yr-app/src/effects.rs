//! Transient combat art follows actual simulation events, outside saved state.

use crate::session::Session;
use std::collections::BTreeMap;
use yr_render::sprites::SpriteBank;
use yr_sim::{Event, Fix};

const MAX_EFFECTS: usize = 256;

#[derive(Clone)]
struct Entity {
    kind: String,
    x: Fix,
    y: Fix,
    facing: u8,
    owner: u8,
}

pub struct Effect {
    pub name: String,
    pub x: Fix,
    pub y: Fix,
    pub born: u64,
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
}

impl EffectScene {
    pub fn remember(&mut self, session: &Session) {
        self.entities = session
            .sim
            .units()
            .iter()
            .map(|u| (u.id, u.kind, u.x, u.y, u.facing, u.owner))
            .chain(session.sim.buildings().iter().map(|b| {
                let (x, y) = b.centre();
                (b.id, b.kind, x, y, 0, b.owner)
            }))
            .filter_map(|(id, kind, x, y, facing, owner)| {
                Some((
                    id,
                    Entity {
                        kind: session.kinds.get(usize::from(kind))?.clone(),
                        x,
                        y,
                        facing,
                        owner,
                    },
                ))
            })
            .collect();
    }

    pub fn update(&mut self, session: &Session, events: &[Event], bank: &mut SpriteBank) {
        let tick = session.sim.tick();
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
                        },
                        origin,
                    ));
                }
                continue;
            }
            if let Event::Impact {
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
                    self.push(bank, name, x, y, tick);
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
                self.push(bank, name, entity.x, entity.y, tick);
            }
        }
    }

    fn push(&mut self, bank: &mut SpriteBank, name: String, x: Fix, y: Fix, born: u64) {
        if !bank.effect_alive(&name, 0) {
            return;
        }
        if self.active.len() == MAX_EFFECTS {
            self.active.remove(0);
        }
        self.active.push(Effect { name, x, y, born });
    }
}

#[cfg(test)]
mod tests;
