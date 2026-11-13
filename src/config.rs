//! Config keys and defaults (M9-01 wires the config hooks). Until then every key is its default.

/// `crafting.enableVanillaCropCrates` (J/common/Configuration.java:ENABLE_VANILLA_CROP_CRATES).
pub const ENABLE_VANILLA_CROP_CRATES_DEFAULT: bool = true;

/// Current value of `crafting.enableVanillaCropCrates`.
pub fn vanilla_crop_crates_enabled() -> bool {
    ENABLE_VANILLA_CROP_CRATES_DEFAULT
}
