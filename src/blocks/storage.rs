//! Crop storage (PORT_PLAN M7-03): crates, rice bale, rice bag, straw bale.
//!
//! Templates bring almost everything (decl rows). Code: crates and the rice bag burn as nothing (FD's
//! `ofFullCopy` does not copy fire odds), the rice bale faces the clicked face (RiceBaleBlock), and both
//! bales make a campfire above them a signal fire (CampfireBaleMixin, #campfire_signal_smoke).
use crate::blocks::rope::NotFlammable;
use mod_api::*;

pub const CRATES: [&str; 6] = [
    "farmersdelight:carrot_crate",
    "farmersdelight:potato_crate",
    "farmersdelight:beetroot_crate",
    "farmersdelight:cabbage_crate",
    "farmersdelight:tomato_crate",
    "farmersdelight:onion_crate",
];
pub const RICE_BAG: &str = "farmersdelight:rice_bag";
pub const RICE_BALE: &str = "farmersdelight:rice_bale";
const SIGNAL_SMOKE: &str = "farmersdelight:campfire_signal_smoke";

/// `Direction.getName()` of the clicked face.
pub fn face_name(face: Direction) -> &'static str {
    match face {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

pub struct RiceBale;

#[crate::engine::part]
impl BlockPart for RiceBale {
    /// `getStateForPlacement`: `FACING` = the clicked face.
    fn state_for_placement(
        &self,
        level: LevelSide<'_>,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        let facing = face_name(event.context.face);
        Some(level.state_with_property(event.state, "facing", facing))
    }
}

/// `CampfireBaleMixin#isFDSmokeSource`.
pub struct SignalSmoke;

#[crate::engine::part]
impl BlockPart for SignalSmoke {
    fn smoke_source(&self, _: LevelSide<'_>, _: &mut SmokeSourceEvent) -> Option<bool> {
        Some(true)
    }
}

pub fn register(registrar: &mut dyn Registrar) {
    for block in CRATES.into_iter().chain([RICE_BAG]) {
        let key = registrar.block_key(block);
        registrar.block_part(key, NotFlammable);
    }
    let bale = registrar.block_key(RICE_BALE);
    registrar.block_part(bale, RiceBale);
    registrar.block_part_tag(SIGNAL_SMOKE, SignalSmoke);
}
