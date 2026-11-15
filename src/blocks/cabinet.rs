//! Cabinets (J/common/block/CabinetBlock.java, J/common/block/entity/CabinetBlockEntity.java;
//! PORT_PLAN M7-05).
//!
//! A 27-slot container shown in the vanilla 9×3 chest menu, open to automation on every face
//! (InvWrapper over the whole container). Its `ContainerOpenersCounter` is the engine's count of
//! the players with the mod container open (`mod_container_viewers`): the first opener plays
//! `block.cabinet.open` and sets `open`, the last one leaving plays `block.cabinet.close` and
//! clears it (`onOpen` / `onClose`, both written with flag 3).
use crate::blocks::cutting_board::{facing_for_yaw, state_value};
use mod_api::*;

pub const IDS: [&str; 13] = [
    "farmersdelight:oak_cabinet",
    "farmersdelight:spruce_cabinet",
    "farmersdelight:birch_cabinet",
    "farmersdelight:jungle_cabinet",
    "farmersdelight:acacia_cabinet",
    "farmersdelight:dark_oak_cabinet",
    "farmersdelight:mangrove_cabinet",
    "farmersdelight:cherry_cabinet",
    "farmersdelight:bamboo_cabinet",
    "farmersdelight:pale_oak_cabinet",
    "farmersdelight:poplar_cabinet",
    "farmersdelight:crimson_cabinet",
    "farmersdelight:warped_cabinet",
];
const SOUND_OPEN: &str = "farmersdelight:block.cabinet.open";
const SOUND_CLOSE: &str = "farmersdelight:block.cabinet.close";

pub fn is_cabinet(id: &str) -> bool {
    IDS.contains(&id)
}

/// `ContainerOpenersCounter.recheckOpeners`: what a change of the opener count from `previous`
/// to `viewers` does. Some(true) is `onOpen`, Some(false) `onClose`, None neither.
pub fn opener_edge(previous: u32, viewers: u32) -> Option<bool> {
    match (previous > 0, viewers > 0) {
        (false, true) => Some(true),
        (true, false) => Some(false),
        _ => None,
    }
}

/// `CabinetBlockEntity#playSound`: half a block out of the front face, at the block's centre
/// height.
pub fn sound_at(pos: [i32; 3], facing: &str) -> [f64; 3] {
    let (dx, dz) = match facing {
        "north" => (0.0, -1.0),
        "south" => (0.0, 1.0),
        "west" => (-1.0, 0.0),
        _ => (1.0, 0.0),
    };
    [
        pos[0] as f64 + 0.5 + dx / 2.0,
        pos[1] as f64 + 0.5,
        pos[2] as f64 + 0.5 + dz / 2.0,
    ]
}

pub struct Cabinet {
    pub signals: crate::blocks::basket::Signals,
    pub names: crate::blocks::basket::Names,
}

#[mod_api::horizon_part]
impl BlockPart for Cabinet {
    /// `getStateForPlacement`: facing the placer.
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        Some(level.state_with_property(event.state, "facing", facing_for_yaw(event.context.yaw)))
    }

    /// `useWithoutItem`: open the menu (SUCCESS on both sides); the opener count does the rest.
    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        let click = event.click;
        if let Some(player) = host.players().acting_player()
            && let Some(level) = host.level(click.dimension.key().id())
            && let Some(be) = host.block_entity(level, click.pos)
        {
            let _ = player.open_menu(host, be);
        }
        Some(InteractionResult::Success)
    }

    fn has_analog_output(&self, _: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }

    fn analog_output(&self, _: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        event.output = crate::blocks::basket::signal(&self.signals, pos);
    }

    /// Barrels (and so cabinets) are not flammable: the oak-planks template's odds are dropped.
    fn fire_odds(&self, _: &dyn LevelHost, _: &mut FireOddsEvent) -> Option<(i32, i32)> {
        Some((0, 0))
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Cabinet {
    /// Ticks only to keep the comparator signal (FD reads it from the container on demand).
    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        if event.entity.items.len() < crate::blocks::basket::SLOTS {
            event
                .entity
                .items
                .resize(crate::blocks::basket::SLOTS, None);
        }
        crate::blocks::basket::update_signal(
            host,
            level,
            event.pos,
            &event.entity.items,
            &self.signals,
        );
    }

    fn pre_remove_side_effects(
        &self,
        _: &mut dyn LevelRandomHost,
        event: &mut PreRemoveSideEffectsEvent<'_>,
    ) {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        crate::blocks::basket::forget_signal(&self.signals, pos);
        crate::blocks::basket::remember_name(&self.names, pos, &event.entity.nbt);
    }
}

/// `ContainerOpenersCounter#onOpen` / `onClose` of a cabinet: the sound (volume 0.5, pitch 0.9 to
/// 1.0), `open` with flag 3, and the container game event.
pub fn on_viewers(host: &mut dyn ServerHost, event: &mut ModContainerViewersEvent<'_>) {
    let Some(open) = opener_edge(event.previous, event.viewers) else {
        return;
    };
    let Some(level) = host.level(event.dimension) else {
        return;
    };
    let pos = event.pos;
    let Some(state) = level.block_state(&*host, pos) else {
        return;
    };
    let name = host.block_state_string(state).unwrap_or_default();
    if !is_cabinet(name.split('[').next().unwrap_or(&name)) {
        return;
    }
    let facing = state_value(&name, "facing").unwrap_or("north").to_string();
    let pitch = level.with_random(host, |r| r.next_float()).unwrap_or(0.5) * 0.1 + 0.9;
    let sound = if open { SOUND_OPEN } else { SOUND_CLOSE };
    level.play_sound(
        host,
        sound,
        sound_at(pos, &facing),
        SoundSource::Blocks,
        0.5,
        pitch,
    );
    let open_value = if open { "true" } else { "false" };
    if let Some(next) =
        host.block_state_id(&crate::items::skillet::set_prop(&name, "open", open_value))
    {
        let _ = level.set_block(
            host,
            pos,
            next,
            BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
        );
    }
    let centre = [
        pos[0] as f64 + 0.5,
        pos[1] as f64 + 0.5,
        pos[2] as f64 + 0.5,
    ];
    let _ = level.game_event(
        host,
        if open {
            "minecraft:container_open"
        } else {
            "minecraft:container_close"
        },
        centre,
        GameEventContext {
            source_entity: None,
            affected_state: None,
        },
    );
}

/// The client's prediction of a cabinet click (E-28): `useWithoutItem` answers SUCCESS for either
/// hand, so a held block is not placed against the cabinet.
pub fn predict(host: HostSide<'_>, event: &mut UseItemOnBlockEvent<'_>) -> Flow {
    let HostSide::Client(host) = host else {
        return Flow::Continue;
    };
    if event.phase != UsePhase::Block {
        return Flow::Continue;
    }
    let host: &dyn ClientHost = &*host;
    let Some(level) = host.level(host.dimension()) else {
        return Flow::Continue;
    };
    let Some(state) = level.block_state(host, event.click.pos) else {
        return Flow::Continue;
    };
    let name = host.block_state_string(state).unwrap_or_default();
    if !is_cabinet(name.split('[').next().unwrap_or(&name)) {
        return Flow::Continue;
    }
    *event.cancellation_result = InteractionResult::Success;
    Flow::Cancel
}

pub fn register(
    registrar: &mut dyn Registrar,
    signals: &crate::blocks::basket::Signals,
    names: &crate::blocks::basket::Names,
) {
    crate::blocks::basket::name_items(registrar, names, &IDS);
    let ty = registrar.block_entity_type(IDS[0]).blocks(&IDS).build();
    registrar
        .block_entity_part(
            &ty,
            Cabinet {
                signals: signals.clone(),
                names: names.clone(),
            },
        )
        .expect("cabinet block entity type");
}

pub fn container() -> ModContainer {
    crate::blocks::basket::container()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_on_first_viewer_and_closes_on_last() {
        assert_eq!(opener_edge(0, 1), Some(true));
        assert_eq!(opener_edge(1, 2), None);
        assert_eq!(opener_edge(2, 1), None);
        assert_eq!(opener_edge(2, 0), Some(false));
    }

    #[test]
    fn sound_comes_from_the_front_face() {
        assert_eq!(sound_at([10, 64, -3], "north"), [10.5, 64.5, -3.0]);
        assert_eq!(sound_at([10, 64, -3], "east"), [11.0, 64.5, -2.5]);
    }
}
