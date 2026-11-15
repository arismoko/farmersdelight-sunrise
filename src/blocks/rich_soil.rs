//! Rich soil and rich soil farmland (PORT_PLAN M5-05): RichSoilBlock, RichSoilFarmlandBlock.
//!
//! A random tick only gets the level's write host, which cannot apply bone meal, so a boost is
//! queued and applied at the end of the same server tick through `level_bone_meal` (the
//! dispenser's path: it sends level event 1505, the 15 growth particles FD's
//! RichSoilBoostParticlesPayload draws). Delta A21: bone meal's success roll applies and the
//! growth lands at the end of the tick.
use super::tomato::{at, block_name, draw, up};
use crate::blocks::cutting_board::state_value;
use crate::config;
use mod_api::*;
use std::sync::{Arc, Mutex};

pub const SOIL: &str = "farmersdelight:rich_soil";
pub const FARMLAND: &str = "farmersdelight:rich_soil_farmland";
const BROWN_COLONY: &str = "farmersdelight:brown_mushroom_colony";
const RED_COLONY: &str = "farmersdelight:red_mushroom_colony";
const PLANTED_FROM_BELOW: &str = "farmersdelight:planted_from_below";
const UNAFFECTED: &str = "farmersdelight:unaffected_by_rich_soil";
/// TallFlowerBlock instances in 26.3 (RichSoilFarmlandBlock skips them).
const TALL_FLOWERS: [&str; 4] = [
    "minecraft:sunflower",
    "minecraft:lilac",
    "minecraft:rose_bush",
    "minecraft:peony",
];

/// One pending boost: the plants to try, in order (the first that grows ends it).
pub struct Boost {
    dimension: String,
    targets: Vec<[i32; 3]>,
}

/// Boosts decided in this tick's random ticks, applied in `drain`.
pub type Boosts = Arc<Mutex<Vec<Boost>>>;

fn in_tag(level: &dyn LevelHost, state: BlockState, tag: &str) -> bool {
    level.block_in_tag(state.block(), tag) == Some(true)
}

/// MathUtils.RAND: FD's own random, apart from the level's (the farmland boost roll).
fn own_float() -> f32 {
    static STATE: Mutex<u64> = Mutex::new(0);
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if *s == 0 {
        *s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
            | 1;
    }
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    (*s >> 40) as f32 / (1u64 << 24) as f32
}

fn queue(boosts: &Boosts, dimension: &LevelDimension, targets: Vec<[i32; 3]>) {
    if targets.is_empty() {
        return;
    }
    boosts
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(Boost {
            dimension: dimension.key().id().to_string(),
            targets,
        });
}

/// Applies the queued boosts (`Hooks::server_tick`, end of tick): RichSoilBlock#boostPlant.
pub fn drain(boosts: &Boosts, host: &mut dyn ServerHost) {
    let pending = std::mem::take(&mut *boosts.lock().unwrap_or_else(|e| e.into_inner()));
    for boost in pending {
        let Some(level) = host.level(&boost.dimension) else {
            continue;
        };
        for pos in boost.targets {
            if level.bone_meal(host, pos) {
                break;
            }
        }
    }
}

pub struct RichSoil {
    pub boosts: Boosts,
}

#[mod_api::horizon_part]
impl BlockPart for RichSoil {
    /// RichSoilBlock#randomTick: a mushroom above becomes its colony; otherwise, with the boost
    /// chance, the plant above (or, for `#planted_from_below`, the one below) gets bone meal.
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> Flow {
        let pos = at(event.pos);
        let above = up(pos, 1);
        let above_state = level.block_state(above);
        let colony = above_state.and_then(|s| match block_name(level, s) {
            "minecraft:brown_mushroom" => Some(BROWN_COLONY),
            "minecraft:red_mushroom" => Some(RED_COLONY),
            _ => None,
        });
        if let Some(colony) = colony {
            if let Some(state) = level.block_state_id(colony) {
                level.set_block(
                    above,
                    state,
                    BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
                );
            }
            return Flow::Cancel;
        }
        let chance = config::rich_soil_boost_chance();
        if chance == 0.0 || draw(level, |r| r.next_float()).is_none_or(|f| f as f64 > chance) {
            return Flow::Cancel;
        }
        let mut targets = Vec::new();
        if let Some(s) = above_state
            && !in_tag(level, s, PLANTED_FROM_BELOW)
            && !in_tag(level, s, UNAFFECTED)
        {
            targets.push(above);
        }
        let below = up(pos, -1);
        if let Some(s) = level.block_state(below)
            && in_tag(level, s, PLANTED_FROM_BELOW)
            && !in_tag(level, s, UNAFFECTED)
        {
            targets.push(below);
        }
        queue(&self.boosts, &event.dimension, targets);
        Flow::Cancel
    }

    /// BlockTransformerHelper.registerTilling(rich_soil, rich_soil_farmland), with vanilla's
    /// HoeItem.onlyIfAirAbove (not from below, air above); a shovel makes no path of it.
    fn tool_modified_state(&self, host: &dyn Host, event: &mut ToolModifiedStateEvent<'_>) -> Flow {
        match event.tool.ability {
            ToolAbility::HoeTill => {
                let above = up(event.tool.pos, 1);
                // Without a user (a dispenser-like caller) the level is unknown: allowed.
                let air_above = event
                    .tool
                    .player
                    .and_then(|id| host.entity(id))
                    .and_then(|e| e.level(host))
                    .and_then(|l| l.block_state(host, above))
                    .is_none_or(|s| {
                        matches!(
                            host.block_state_string(s).as_deref(),
                            Some("minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
                        )
                    });
                if event.tool.face != [0, -1, 0] && air_above {
                    *event.final_state = host.block_state_id(FARMLAND);
                    Flow::Continue
                } else {
                    Flow::Cancel
                }
            }
            ToolAbility::ShovelFlatten => Flow::Cancel,
            _ => Flow::Continue,
        }
    }
}

pub struct RichSoilFarmland {
    pub boosts: Boosts,
    pub trample: Trample,
}

/// Which numeric block states are rich soil farmland, for `farmland_trample`, which is told
/// only the game's state number and has no host to ask (engine gap E-22). `predict` fills it
/// when the server starts; a landing the hook saw is checked against the `fall_on` that follows
/// it, which knows the block, so a wrong prediction corrects itself after one landing.
#[derive(Clone, Default)]
pub struct Trample(Arc<Mutex<TrampleState>>);

#[derive(Default)]
struct TrampleState {
    known: std::collections::HashSet<u32>,
    last: Option<([i32; 3], u32)>,
}

impl Trample {
    fn state(&self) -> std::sync::MutexGuard<'_, TrampleState> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The game numbers states block by block in registry order: the farmland's first state is
    /// the count of every earlier block's states.
    pub fn predict(&self, host: &dyn Host) {
        let Some(farmland) = host.block_state_id(FARMLAND).map(|s| s.block()) else {
            return;
        };
        let mut base = 0u32;
        for key in host.block_keys() {
            let Some(states) = host.block_states().states_of(key) else {
                return;
            };
            if key == farmland {
                let mut state = self.state();
                state.known = (base..base + states.len()).collect();
                state.last = None;
                return;
            }
            base += states.len();
        }
    }

    /// RichSoilFarmlandBlock#fallOn never tramples (Q11).
    pub fn on_trample(&self, event: &FarmlandTrampleEvent) -> Flow {
        let mut state = self.state();
        state.last = Some((event.pos, event.state));
        if state.known.contains(&event.state) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// A landing on rich soil farmland: if the trample hook just saw it under a number not
    /// known as this block, the numbering differs from the prediction; learn the block's states.
    fn on_fall(&self, level: &dyn LevelHost, pos: [i32; 3], landed: BlockState) {
        let mut state = self.state();
        let Some((at, raw)) = state.last.take() else {
            return;
        };
        if at != pos || state.known.contains(&raw) {
            return;
        }
        let first = level
            .block_state_id(FARMLAND)
            .and_then(|s| level.state_with_property(s, "moisture", "0"))
            .map_or(landed.ordinal(), |s| s.ordinal());
        let base = raw.saturating_sub(u32::from(landed.ordinal().saturating_sub(first)));
        eprintln!(
            "farmersdelight: rich soil farmland states are numbered from {base}; trampling is cancelled from now on"
        );
        state.known = (base..base + 8).collect();
    }
}

/// RichSoilFarmlandBlock#isNearWater: any `#minecraft:water` fluid within 4 blocks, here or one up.
fn near_water(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    for dy in 0..=1 {
        for dx in -4..=4 {
            for dz in -4..=4 {
                let at = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
                if level
                    .fluid_state(at)
                    .is_some_and(|f| crate::blocks::rope::is_water(level, f))
                {
                    return true;
                }
            }
        }
    }
    false
}

#[mod_api::horizon_part]
impl BlockPart for RichSoilFarmland {
    /// RichSoilFarmlandBlock#randomTick: dries a stage without water or rain (never back to
    /// soil), soaks to 7 with it, and when soaked boosts the plant above with the boost chance.
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> Flow {
        let pos = at(event.pos);
        let moisture: i32 = level
            .state_string(event.state)
            .and_then(|s| state_value(&s, "moisture").and_then(|v| v.parse().ok()))
            .unwrap_or(0);
        let set = |level: &mut dyn LevelWriteHost, value: i32| {
            if let Some(next) =
                level.state_with_property(event.state, "moisture", &value.to_string())
            {
                level.set_block(pos, next, BlockUpdateFlags::CLIENTS);
            }
        };
        let above = up(pos, 1);
        if !near_water(level, pos) && !level.is_raining_at(above) {
            if moisture > 0 {
                set(level, moisture - 1);
            }
        } else if moisture < 7 {
            set(level, 7);
        } else {
            let chance = config::rich_soil_boost_chance();
            if chance == 0.0 {
                return Flow::Cancel;
            }
            let Some(above_state) = level.block_state(above) else {
                return Flow::Cancel;
            };
            if in_tag(level, above_state, UNAFFECTED)
                || TALL_FLOWERS.contains(&block_name(level, above_state))
            {
                return Flow::Cancel;
            }
            if own_float() as f64 <= chance {
                queue(&self.boosts, &event.dimension, vec![above]);
            }
        }
        Flow::Cancel
    }

    fn fall_on(&self, level: &mut dyn LevelRandomHost, event: &mut FallOnEvent<'_>) {
        self.trample.on_fall(level, at(event.pos), event.state);
    }

    /// RichSoilFarmlandBlock#canSurvive: FarmlandBlock's, or under a melon or pumpkin.
    fn can_survive(&self, level: &dyn LevelHost, event: &mut CanSurviveEvent) -> Option<bool> {
        let above = level.block_state(up(at(event.pos), 1))?;
        matches!(
            block_name(level, above),
            "minecraft:melon" | "minecraft:pumpkin"
        )
        .then_some(true)
    }
}

pub fn register(registrar: &mut dyn Registrar, boosts: &Boosts, trample: &Trample) {
    let soil = registrar.block_key(SOIL);
    registrar.block_part(
        soil,
        RichSoil {
            boosts: boosts.clone(),
        },
    );
    let farmland = registrar.block_key(FARMLAND);
    registrar.block_part(
        farmland,
        RichSoilFarmland {
            boosts: boosts.clone(),
            trample: trample.clone(),
        },
    );
}
