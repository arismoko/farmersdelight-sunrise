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

/// `overrides.stack_size.enableStackableSoupItems` (Configuration.java:ENABLE_STACKABLE_SOUP_ITEMS).
pub fn stackable_soup_items_enabled() -> bool {
    true
}

/// `overrides.stack_size.soupItemList` (Configuration.java:SOUP_ITEM_LIST).
pub fn soup_item_list() -> &'static [&'static str] {
    &[
        "minecraft:mushroom_stew",
        "minecraft:beetroot_soup",
        "minecraft:rabbit_stew",
    ]
}

/// `overrides.enableRabbitStewBuff` (Configuration.java:ENABLE_RABBIT_STEW_BUFF).
pub fn rabbit_stew_buff_enabled() -> bool {
    true
}

/// `overrides.enableVanillaSoupExtraEffects` (Configuration.java:ENABLE_VANILLA_SOUP_EXTRA_EFFECTS).
pub fn vanilla_soup_extra_effects_enabled() -> bool {
    true
}

/// `overrides.enableVanillaStrawBedRename` (Configuration.java:ENABLE_VANILLA_STRAW_BED_RENAME).
pub fn vanilla_straw_bed_rename_enabled() -> bool {
    true
}

/// `settings.enableRopeReeling` (Configuration.java:ENABLE_ROPE_REELING), default true.
pub fn rope_reeling_enabled() -> bool {
    true
}

/// `settings.enableTomatoVineClimbingTaggedRopes` (Configuration.java), default true.
pub fn tomato_vine_climbing_tagged_ropes() -> bool {
    true
}

/// `crafting.enableTomatoRopePermanence` (Configuration.java), default true.
pub fn tomato_rope_permanence() -> bool {
    true
}

/// `farming.richSoilBoostChance` (Configuration.java:RICH_SOIL_BOOST_CHANCE), default 0.2.
pub fn rich_soil_boost_chance() -> f64 {
    0.2
}

/// `settings.enablePumpkinPieSneakToPlace` (Configuration.java:ENABLE_PUMPKIN_PIE_SNEAK_TO_PLACE), default false.
pub fn pumpkin_pie_sneak_to_place() -> bool {
    false
}

/// `common.enableCuttingBoardDispenserBehavior` (Configuration.java:ENABLE_DISPENSER_TOOLS_CUTTING_BOARD).
pub fn dispenser_tools_cutting_board() -> bool {
    true
}
