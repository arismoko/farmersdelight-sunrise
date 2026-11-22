//! Tatami, tatami mats and the canvas rug (J/common/block/TatamiBlock.java, TatamiMatBlock.java,
//! TatamiHalfMatBlock.java, CanvasRugBlock.java; PORT_PLAN M7-07).
//!
//! FD's `setPlacedBy` (pairing a tatami with the one clicked, writing a full mat's head) runs in
//! `entity_place`, which the engine fires for the placing entity just before the placed block is
//! written: the partner cell is written first, and the placed block's own neighbour updates then
//! reach it as FD's `updateNeighbourShapes` does.
//! The engine fires the hook from a placement preview and then asks `state_for_placement`
//! again, so both placement rules accept the partner the hook wrote.
use crate::blocks::cutting_board::{facing_for_yaw, state_value};
use crate::blocks::rope::NotFlammable;
use crate::blocks::storage::face_name;
use mod_api::*;

pub const TATAMI: &str = "farmersdelight:tatami";
pub const FULL_MAT: &str = "farmersdelight:full_tatami_mat";
pub const HALF_MAT: &str = "farmersdelight:half_tatami_mat";
pub const RUG: &str = "farmersdelight:canvas_rug";

// ---------------------------------------------------------------- pure rules

pub fn opposite(direction: &str) -> &'static str {
    crate::blocks::basket::opposite(direction)
}

pub fn offset(pos: [i32; 3], direction: &str) -> [i32; 3] {
    let d = match direction {
        "down" => [0, -1, 0],
        "up" => [0, 1, 0],
        "north" => [0, 0, -1],
        "south" => [0, 0, 1],
        "west" => [-1, 0, 0],
        _ => [1, 0, 0],
    };
    [pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]]
}

/// `context.getHorizontalDirection()`: the horizontal direction the placer looks.
pub fn looking(yaw: f32) -> &'static str {
    opposite(facing_for_yaw(yaw))
}

/// `TatamiMatBlock.getDirectionToOther`.
pub fn to_other(part: &str, facing: &str) -> &'static str {
    if part == "foot" {
        opposite(opposite(facing)) // the facing itself, as a 'static str
    } else {
        opposite(facing)
    }
}

/// `TatamiMatBlock#updateShape` toward the other half: the half stays only while it survives and
/// the other cell holds the mat's other part.
pub fn mat_keeps_pair(
    survives: bool,
    other_is_mat: bool,
    other_part: Option<&str>,
    part: &str,
) -> bool {
    survives && other_is_mat && other_part.is_some_and(|p| p != part)
}

// ---------------------------------------------------------------- helpers

fn name_of(level: &dyn LevelHost, state: BlockState) -> String {
    level.state_string(state).unwrap_or_default()
}

fn block_of(name: &str) -> &str {
    name.split('[').next().unwrap_or(name)
}

fn is_air_name(name: &str) -> bool {
    matches!(
        block_of(name),
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    )
}

/// `!level.isEmptyBlock(pos.below())`.
fn floor_below(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    level
        .block_state([pos[0], pos[1] - 1, pos[2]])
        .is_none_or(|s| !is_air_name(&name_of(level, s)))
}

/// `BlockState.canBeReplaced(context)` for a cell the mat's head would take: air or a block in
/// `minecraft:replaceable`.
fn replaceable(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    level.block_state(pos).is_some_and(|s| {
        is_air_name(&name_of(level, s))
            || level.block_in_tag(s.block(), "minecraft:replaceable") == Some(true)
    })
}

fn air(level: &dyn LevelTicksHost) -> Option<BlockState> {
    level.block_state_id("minecraft:air")
}

fn at(pos: BlockPos) -> [i32; 3] {
    [pos.x, pos.y, pos.z]
}

// ---------------------------------------------------------------- tatami

pub struct Tatami;

#[crate::engine::part]
impl BlockPart for Tatami {
    /// `getStateForPlacement`: facing into the clicked block; paired when it is an unpaired
    /// tatami and the placer is not sneaking.
    fn state_for_placement(
        &self,
        level: LevelSide<'_>,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        let context = event.context;
        let face = face_name(context.face);
        let target = offset(at(context.pos), opposite(face));
        let target_name = level.block_state(target).map(|s| name_of(level, s));
        // The engine asks this twice: for the `entity_place` preview, then for the placement
        // itself, after `on_placed` already paired the clicked tatami toward this cell.
        let pairing = !context.sneaking
            && target_name.as_deref().is_some_and(|n| {
                block_of(n) == TATAMI
                    && (state_value(n, "paired") == Some("false")
                        || state_value(n, "facing") == Some(face))
            });
        let state = level.state_with_property(event.state, "facing", opposite(face))?;
        Some(level.state_with_property(state, "paired", if pairing { "true" } else { "false" }))
    }

    /// `updateShape`: the partner in front gone, the tatami is unpaired.
    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let name = level.state_string(event.state)?;
        let facing = state_value(&name, "facing")?;
        if face_name(event.direction) == facing
            && state_value(&name, "paired") == Some("true")
            && block_of(&level.state_string(event.neighbour).unwrap_or_default()) != TATAMI
        {
            return level.state_with_property(event.state, "paired", "false");
        }
        None
    }
}

// ---------------------------------------------------------------- mats

pub struct FullMat;

#[crate::engine::part]
impl BlockPart for FullMat {
    /// `getStateForPlacement`: facing the look direction, refused unless the cell ahead can take
    /// the head (replaceable, with a block under it).
    fn state_for_placement(
        &self,
        level: LevelSide<'_>,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        let facing = looking(event.context.yaw);
        let pair = offset(at(event.context.pos), facing);
        // Asked again after `on_placed` wrote the head (see `Tatami`): the head this placement
        // just wrote counts as free.
        let own_head = level.block_state(pair).is_some_and(|s| {
            let n = name_of(level, s);
            block_of(&n) == FULL_MAT
                && state_value(&n, "part") == Some("head")
                && state_value(&n, "facing") == Some(facing)
        });
        if own_head || (replaceable(level, pair) && floor_below(level, pair)) {
            Some(level.state_with_property(event.state, "facing", facing))
        } else {
            Some(None)
        }
    }

    fn can_survive(&self, level: LevelSide<'_>, event: &mut CanSurviveEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(floor_below(level, at(event.pos)))
    }

    /// `updateShape`: toward the other half the pair must hold, otherwise the floor; either
    /// failing turns the half to air (vanilla's `updateOrDestroy` then drops its loot: the
    /// head's).
    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let name = level.state_string(event.state)?;
        let part = state_value(&name, "part").unwrap_or("foot").to_string();
        let facing = state_value(&name, "facing").unwrap_or("north").to_string();
        let survives = floor_below(&*level, at(event.pos));
        if face_name(event.direction) == to_other(&part, &facing) {
            let other = level.state_string(event.neighbour).unwrap_or_default();
            let keep = mat_keeps_pair(
                survives,
                block_of(&other) == FULL_MAT,
                state_value(&other, "part"),
                &part,
            );
            return if keep { None } else { air(level) };
        }
        if survives { None } else { air(level) }
    }

    /// `playerWillDestroy`: a creative player breaking the foot takes the head away with it,
    /// without drops (FD: `setBlock(air, 35)` + `levelEvent(2001)`).
    fn on_destroyed_by_player(
        &self,
        host: &mut dyn ServerHost,
        event: &mut OnDestroyedByPlayerEvent<'_>,
    ) -> Option<bool> {
        let creative = host
            .players()
            .acting_player()
            .and_then(|p| p.is_creative(&*host))
            .unwrap_or(false);
        let name = host.block_state_string(event.state)?;
        if !creative || state_value(&name, "part") != Some("foot") {
            return None;
        }
        let facing = state_value(&name, "facing").unwrap_or("north");
        let head = offset(event.pos, to_other("foot", facing));
        let level = host.level(host.dimension())?;
        let head_name = level
            .block_state(&*host, head)
            .and_then(|s| host.block_state_string(s))
            .unwrap_or_default();
        if block_of(&head_name) == FULL_MAT && state_value(&head_name, "part") == Some("head") {
            let _ = level.destroy_block(host, head, false, None, false);
        }
        None
    }
}

/// `setPlacedBy` of the tatami and the full mat, from `entity_place` (before the placed block is
/// written).
pub fn on_placed(host: &mut dyn ServerHost, event: &mut EntityPlaceEvent) {
    if event.placer.is_none() {
        return;
    }
    let Some(name) = host.block_state_string(event.state) else {
        return;
    };
    let block = block_of(&name);
    if block != TATAMI && block != FULL_MAT {
        return;
    }
    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    let pos = at(event.pos);
    let facing = state_value(&name, "facing").unwrap_or("north").to_string();
    let target = offset(pos, &facing);
    let partner = if block == TATAMI {
        // Paired at placement exactly when the clicked tatami was unpaired and the placer was not
        // sneaking: the same test setPlacedBy repeats.
        if state_value(&name, "paired") != Some("true") {
            return;
        }
        let set = crate::items::skillet::set_prop(&name, "facing", opposite(&facing));
        crate::items::skillet::set_prop(&set, "paired", "true")
    } else {
        crate::items::skillet::set_prop(&name, "part", "head")
    };
    if let Some(state) = host.block_state_id(&partner) {
        let _ = level.set_block(
            host,
            target,
            state,
            BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
        );
    }
}

pub struct HalfMat;

#[crate::engine::part]
impl BlockPart for HalfMat {
    /// `getStateForPlacement`: facing the placer.
    fn state_for_placement(
        &self,
        level: LevelSide<'_>,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(level.state_with_property(event.state, "facing", facing_for_yaw(event.context.yaw)))
    }

    fn can_survive(&self, level: LevelSide<'_>, event: &mut CanSurviveEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(floor_below(level, at(event.pos)))
    }

    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        if floor_below(&*level, at(event.pos)) {
            None
        } else {
            air(level)
        }
    }
}

pub fn register(registrar: &mut dyn Registrar) {
    for id in [TATAMI, FULL_MAT, HALF_MAT, RUG] {
        let key = registrar.block_key(id);
        registrar.block_part(key, NotFlammable);
    }
    let key = registrar.block_key(TATAMI);
    registrar.block_part(key, Tatami);
    let key = registrar.block_key(FULL_MAT);
    registrar.block_part(key, FullMat);
    let key = registrar.block_key(HALF_MAT);
    registrar.block_part(key, HalfMat);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mat_halves_point_at_each_other() {
        assert_eq!(to_other("foot", "east"), "east");
        assert_eq!(to_other("head", "east"), "west");
        assert_eq!(looking(0.0), "south");
        assert_eq!(looking(90.0), "west");
    }

    #[test]
    fn mat_half_needs_its_partner_and_floor() {
        assert!(mat_keeps_pair(true, true, Some("head"), "foot"));
        assert!(!mat_keeps_pair(true, true, Some("foot"), "foot"));
        assert!(!mat_keeps_pair(true, false, None, "foot"));
        assert!(!mat_keeps_pair(false, true, Some("head"), "foot"));
    }
}
