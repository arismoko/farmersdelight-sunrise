//! Safety net (J/common/block/SafetyNetBlock.java; PORT_PLAN M7-08).
//!
//! `bounceRestitution(0.6)` and `fallOn`: an entity that does not suppress bounce lands with no
//! fall damage (`causeFallDamage(distance, 0)`); one that does (a sneaking player) takes vanilla's.
//! The `fall_on` host cannot read the entity, so the server keeps, each tick, the set of players
//! holding Shift (`Player.isSuppressingBounce` is `isShiftKeyDown`). Waterlogging as the basket.
use crate::blocks::cutting_board::state_value;
use mod_api::*;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub const ID: &str = "farmersdelight:safety_net";
const BOUNCE: f32 = 0.6;

/// The network ids of the players holding Shift, as of this server tick.
#[derive(Clone, Default)]
pub struct Sneaking(Arc<Mutex<HashSet<u32>>>);

impl Sneaking {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashSet<u32>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Re-read every player's Shift key (`server_tick`).
    pub fn refresh(&self, host: &dyn ServerHost) {
        let players = host.players().server_players();
        let mut set = self.lock();
        set.clear();
        for p in players {
            if p.entity().is_shift_key_down(host) == Some(true) {
                set.insert(p.entity().id());
            }
        }
    }

    fn suppresses_bounce(&self, entity: i32) -> bool {
        u32::try_from(entity).is_ok_and(|id| self.lock().contains(&id))
    }
}

pub struct SafetyNet {
    pub sneaking: Sneaking,
}

#[crate::engine::part]
impl BlockPart for SafetyNet {
    /// `getStateForPlacement`: waterlogged in water.
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let water = crate::blocks::rope::is_water(level, event.fluid);
        Some(level.state_with_property(
            event.state,
            "waterlogged",
            if water { "true" } else { "false" },
        ))
    }

    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let name = level.state_string(event.state).unwrap_or_default();
        if state_value(&name, "waterlogged") == Some("true") {
            level.schedule_fluid_tick(
                [event.pos.x, event.pos.y, event.pos.z],
                "minecraft:water",
                5,
            );
        }
        None
    }

    /// Water stays where a waterlogged net is removed (E-13, as the cutting board).
    fn remove_block(
        &self,
        level: &dyn LevelHost,
        event: &mut RemoveBlockEvent,
    ) -> Option<BlockState> {
        let name = level.state_string(event.state)?;
        (state_value(&name, "waterlogged") == Some("true"))
            .then(|| level.block_state_id("minecraft:water"))
            .flatten()
    }

    /// `fallOn`: no fall damage unless the entity suppresses bounce.
    fn fall_on(&self, _: &mut dyn LevelRandomHost, event: &mut FallOnEvent<'_>) {
        if !self.sneaking.suppresses_bounce(event.entity) {
            event.damage_multiplier = 0.0;
        }
    }

    /// `bounceRestitution(0.6)`; the engine already skips a sneaking player.
    fn bounce_restitution(&self, _: LevelSide, event: &mut BounceRestitutionEvent<'_>) {
        event.factor = event.factor.max(BOUNCE);
    }

    fn fire_odds(&self, _: &dyn LevelHost, _: &mut FireOddsEvent) -> Option<(i32, i32)> {
        Some((0, 0))
    }
}

pub fn register(registrar: &mut dyn Registrar, sneaking: &Sneaking) {
    let key = registrar.block_key(ID);
    registrar.block_part(
        key,
        SafetyNet {
            sneaking: sneaking.clone(),
        },
    );
}
