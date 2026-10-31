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
}

pub struct Effect {
    pub name: String,
    pub x: Fix,
    pub y: Fix,
    pub born: u64,
}

#[derive(Default)]
pub struct EffectScene {
    entities: BTreeMap<u32, Entity>,
    pub active: Vec<Effect>,
}

impl EffectScene {
    pub fn remember(&mut self, session: &Session) {
        self.entities = session
            .sim
            .units()
            .iter()
            .map(|u| (u.id, u.kind, u.x, u.y, u.facing))
            .chain(session.sim.buildings().iter().map(|b| {
                let (x, y) = b.centre();
                (b.id, b.kind, x, y, 0)
            }))
            .filter_map(|(id, kind, x, y, facing)| {
                Some((
                    id,
                    Entity {
                        kind: session.kinds.get(usize::from(kind))?.clone(),
                        x,
                        y,
                        facing,
                    },
                ))
            })
            .collect();
    }

    pub fn update(&mut self, session: &Session, events: &[Event], bank: &mut SpriteBank) {
        let tick = session.sim.tick();
        self.active
            .retain(|e| tick >= e.born && bank.effect_alive(&e.name, tick - e.born));
        // Current poses win for survivors; previous poses preserve entities
        // which fire and die in the same tick. Refresh the table afterwards.
        let mut entities = self.entities.clone();
        self.remember(session);
        entities.extend(self.entities.iter().map(|(&id, e)| (id, e.clone())));
        for event in events {
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
