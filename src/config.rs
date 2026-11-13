//! Config keys and defaults (M9-01 wires the config hooks). Until then every key is its default.

/// `crafting.enableVanillaCropCrates` (J/common/Configuration.java:ENABLE_VANILLA_CROP_CRATES).
pub const ENABLE_VANILLA_CROP_CRATES_DEFAULT: bool = true;

/// Current value of `crafting.enableVanillaCropCrates`.
pub fn vanilla_crop_crates_enabled() -> bool {
    ENABLE_VANILLA_CROP_CRATES_DEFAULT
}

/// `crafting.cuttingBoardFortuneBonus` (J/common/Configuration.java:CUTTING_BOARD_FORTUNE_BONUS).
pub const CUTTING_BOARD_FORTUNE_BONUS_DEFAULT: f64 = 0.1;

pub fn cutting_board_fortune_bonus() -> f64 {
    CUTTING_BOARD_FORTUNE_BONUS_DEFAULT
}
