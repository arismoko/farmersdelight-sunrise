//! Crop helpers for add-on blocks (CD-06): the pieces of vanilla `CropBlock` the base uses for FD's crops.

use mod_api::*;

/// `CropBlock.getGrowthSpeed(state, level, pos)`: 1, plus 1 (moist farmland 3) for the block below `pos` in
/// `#minecraft:grows_crops`, plus a quarter of that for each of the 8 diagonal/side cells below; halved when
/// the same block stands both east/west and north/south, or on a diagonal.
pub fn growth_speed(level: &dyn LevelHost, state: BlockState, pos: [i32; 3]) -> f32 {
    crate::blocks::tomato::growth_speed(level, state, pos, true)
}

/// `random.nextInt(bound)` from the level's own random stream (the draw advances it). `None` when the level
/// has no random to draw from.
pub fn next_int(level: &mut dyn LevelRandomHost, bound: i32) -> Option<i32> {
    crate::blocks::tomato::draw(level, |r| r.next_int_bound(bound))
}

/// `random.nextFloat()` from the level's own random stream.
pub fn next_float(level: &mut dyn LevelRandomHost) -> Option<f32> {
    crate::blocks::tomato::draw(level, |r| r.next_float())
}
