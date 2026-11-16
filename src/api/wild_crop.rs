//! Wild crops for add-on blocks (BX-05): FD 26.3 `WildCropBlock` behaviour, the same code FD's
//! six wild crops run.
//!
//! The add-on declares the block (tall-grass-like, its shape, loot) and calls [`register`] from
//! its `register` hook:
//!
//! ```ignore
//! ported_farmersdelight::api::wild_crop::register(
//!     registrar, "corn_delight:wild_corn", "minecraft:saturation", 8);
//! ```
//!
//! The Part then places and keeps the block only on `#farmersdelight:grows_wild_crops`, makes it a
//! bone meal target (80% success) that spreads a copy nearby unless ten stand in the 9×3×9 around,
//! and a brown mooshroom fed the item takes the stew effect for `seconds` (FD's `FlowerBlock`
//! constructor values). A block with its own Part can call the pieces directly.

use mod_api::{BlockState, LevelHost, LevelRandomHost, LevelWriteHost, Registrar};

/// Attaches FD's wild-crop behaviour to `block` and records its suspicious stew effect (an effect
/// id) and duration in seconds. Registering the same block again replaces the effect.
pub fn register(
    registrar: &mut dyn Registrar,
    block: &'static str,
    stew_effect: &'static str,
    seconds: i32,
) {
    crate::blocks::wild::register_wild_crop(registrar, block, stew_effect, seconds);
}

/// `WildCropBlock#mayPlaceOn`: `ground` is in `#farmersdelight:grows_wild_crops`.
pub fn may_place_on(level: &dyn LevelHost, ground: BlockState) -> bool {
    crate::blocks::wild::may_place_on(level, ground)
}

/// Whether a wild crop at `pos` survives (its ground is in the tag).
pub fn can_survive(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    crate::blocks::wild::can_survive_at(level, pos)
}

/// `isBonemealSuccess`: true with chance 0.8.
pub fn bonemeal_success(level: &mut dyn LevelRandomHost) -> bool {
    crate::blocks::wild::bonemeal_success(level)
}

/// `performBonemeal`: spread one more `state` near `pos` (the 9×3×9 ≤ 10 rule).
pub fn bonemeal_spread(level: &mut dyn LevelWriteHost, pos: [i32; 3], state: BlockState) {
    crate::blocks::wild::bonemeal_spread(level, pos, state);
}

/// The suspicious stew effect id and seconds of a registered wild crop (FD's or an add-on's).
pub fn suspicious_stew_effect(block: &str) -> Option<(&'static str, i32)> {
    crate::blocks::wild::stew_effect(block)
}
