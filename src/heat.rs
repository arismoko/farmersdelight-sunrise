//! Heat from below (J/common/block/entity/HeatableBlockEntity.java, PORT_PLAN M2-01).
//!
//! A block is heated when the block below is in `#farmersdelight:heat_sources` (and, if that
//! block has a `lit` property, it is `true`), or — unless the heated block requires direct heat —
//! when the block below is in `#farmersdelight:heat_conductors` and the block two below is such a
//! source. Pure over a lookup so both sides (server ticks, client heat icon) share it.
use mod_api::*;

pub const HEAT_SOURCES: &str = "farmersdelight:heat_sources";
pub const HEAT_CONDUCTORS: &str = "farmersdelight:heat_conductors";

/// What the rule needs to know about one block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeatFacts {
    pub source: bool,
    pub conductor: bool,
    /// `Some(value)` if the state has `lit`.
    pub lit: Option<bool>,
}

fn gives_heat(f: HeatFacts) -> bool {
    f.source && f.lit.unwrap_or(true)
}

/// `HeatableBlockEntity#isHeated` over `below` and `below2` (the block two below).
pub fn is_heated(
    below: HeatFacts,
    below2: impl FnOnce() -> HeatFacts,
    requires_direct_heat: bool,
) -> bool {
    if below.source {
        return gives_heat(below);
    }
    if !requires_direct_heat && below.conductor {
        return gives_heat(below2());
    }
    false
}

/// Facts of the block at `pos` read through a level-scoped host (block-entity ticks, Parts).
pub fn facts_in_level(level: &dyn LevelHost, pos: [i32; 3]) -> HeatFacts {
    let Some(state) = level.block_state(pos) else {
        return HeatFacts::default();
    };
    let block = state.block();
    let lit = level
        .state_string(state)
        .and_then(|s| crate::blocks::cutting_board::state_value(&s, "lit").map(|v| v == "true"));
    HeatFacts {
        source: level.block_in_tag(block, HEAT_SOURCES) == Some(true),
        conductor: level.block_in_tag(block, HEAT_CONDUCTORS) == Some(true),
        lit,
    }
}

/// Whether the block at `pos` is heated, read through a level-scoped host.
pub fn is_heated_in_level(
    level: &dyn LevelHost,
    pos: [i32; 3],
    requires_direct_heat: bool,
) -> bool {
    let [x, y, z] = pos;
    is_heated(
        facts_in_level(level, [x, y - 1, z]),
        || facts_in_level(level, [x, y - 2, z]),
        requires_direct_heat,
    )
}

/// Facts of the block at `pos` read through a full host and a level handle (block-entity Parts).
pub fn facts_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> HeatFacts {
    let Some(state) = level.block_state(host, pos) else {
        return HeatFacts::default();
    };
    let block = state.block();
    let lit = host
        .block_state_string(state)
        .and_then(|s| crate::blocks::cutting_board::state_value(&s, "lit").map(|v| v == "true"));
    HeatFacts {
        source: host.block_in_tag(block, HEAT_SOURCES) == Some(true),
        conductor: host.block_in_tag(block, HEAT_CONDUCTORS) == Some(true),
        lit,
    }
}

/// `HeatableBlockEntity#isHeated` (conductors allowed) through a full host.
pub fn is_heated_in_level_ref(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> bool {
    let [x, y, z] = pos;
    is_heated(
        facts_at(host, level, [x, y - 1, z]),
        || facts_at(host, level, [x, y - 2, z]),
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const AIR: HeatFacts = HeatFacts {
        source: false,
        conductor: false,
        lit: None,
    };
    const STONE: HeatFacts = AIR;
    const fn src(lit: Option<bool>) -> HeatFacts {
        HeatFacts {
            source: true,
            conductor: false,
            lit,
        }
    }
    const HOPPER: HeatFacts = HeatFacts {
        source: false,
        conductor: true,
        lit: None,
    };

    #[test]
    fn twelve_cases() {
        // (below, two below, requires direct heat, heated)
        let cases = [
            ("lit campfire", src(Some(true)), AIR, false, true),
            ("unlit campfire", src(Some(false)), AIR, false, false),
            ("magma block", src(None), AIR, false, true),
            ("lava", src(None), AIR, false, true),
            ("fire", src(None), AIR, false, true),
            ("lava cauldron", src(None), AIR, false, true),
            ("lit stove", src(Some(true)), AIR, false, true),
            ("unlit stove", src(Some(false)), AIR, false, false),
            (
                "hopper over lit campfire",
                HOPPER,
                src(Some(true)),
                false,
                true,
            ),
            (
                "hopper over unlit campfire",
                HOPPER,
                src(Some(false)),
                false,
                false,
            ),
            (
                "hopper over campfire, direct heat only",
                HOPPER,
                src(Some(true)),
                true,
                false,
            ),
            (
                "stone over lit campfire",
                STONE,
                src(Some(true)),
                false,
                false,
            ),
        ];
        for (name, below, below2, direct, expect) in cases {
            assert_eq!(is_heated(below, || below2, direct), expect, "{name}");
        }
    }

    #[test]
    fn carried_tags_name_the_sources() {
        use mod_api::serde_json;
        let tag = |name: &str| -> serde_json::Value {
            let path = format!(
                "{}/data/farmersdelight/tags/block/{name}.json",
                env!("CARGO_MANIFEST_DIR")
            );
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
        };
        let sources = tag("heat_sources").to_string();
        for id in [
            "minecraft:magma_block",
            "farmersdelight:stove",
            "minecraft:lava_cauldron",
            "#farmersdelight:tray_heat_sources",
        ] {
            assert!(sources.contains(id), "{id}");
        }
        assert!(
            tag("heat_conductors")
                .to_string()
                .contains("minecraft:hopper")
        );
        let tray = tag("tray_heat_sources").to_string();
        for id in ["minecraft:lava", "#minecraft:campfires", "#minecraft:fire"] {
            assert!(tray.contains(id), "{id}");
        }
    }
}
