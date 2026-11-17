//! Rice (PORT_PLAN M5-04): `rice` (RiceBlock) grows in a water source and puts `rice_panicles`
//! (RicePaniclesBlock, a CropBlock) on top once mature.
//!
//! `rice` is declared on the kelp template, which always holds a water source as RiceBlock's
//! `getFluidState` does; its own growth, placement, survival and shape rules are replaced here.
use super::tomato::{
    LevelOf, age, at, block_name, bright_enough, draw, growth_speed, on_supports_crops, up,
    with_age,
};
use mod_api::*;

pub const RICE: &str = "farmersdelight:rice";
pub const PANICLES: &str = "farmersdelight:rice_panicles";
/// RiceBlock#getMaxAge / RicePaniclesBlock#getMaxAge.
const MAX_AGE: i32 = 3;

fn flags() -> BlockUpdateFlags {
    BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS
}

/// RiceBlock#mayPlaceOn: VegetationBlock's ground (`#minecraft:supports_vegetation` in 26.3) or
/// `#minecraft:supports_crops`.
fn rice_ground(level: &dyn LevelHost, ground: BlockState) -> bool {
    level.block_in_tag(ground.block(), "minecraft:supports_vegetation") == Some(true)
        || on_supports_crops(level, ground)
}

fn is_air(level: &dyn LevelHost, state: BlockState) -> bool {
    matches!(
        block_name(level, state),
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    )
}

/// RicePaniclesBlock#canSurvive: (raw light ≥ 8 or sky) and rice below.
fn panicles_survive(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    bright_enough(level, pos, 8)
        && level
            .block_state(up(pos, -1))
            .is_some_and(|s| block_name(level, s) == RICE)
}

/// RiceBlock#getStateForPlacement and the panicle checks share "a full water source here".
fn full_water(level: &dyn LevelHost, fluid: FluidState) -> bool {
    fluid.source && crate::blocks::rope::is_water(level, fluid)
}

pub struct Rice;

#[crate::engine::part]
impl BlockPart for Rice {
    /// RiceBlock#getStateForPlacement: only into a full water source, at age 0.
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        if !full_water(level, event.fluid) {
            return Some(None);
        }
        Some(with_age(level, RICE, 0))
    }

    /// RiceBlock#randomTick: with raw light ≥ 6 above, one in 3 grows a stage; at max age the
    /// panicles appear above if they can stand there and it is empty.
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> Flow {
        let pos = at(event.pos);
        if bright_enough(level, up(pos, 1), 6) {
            let age = age(level, event.state);
            // (int)(25.0F / 10) + 1
            if age <= MAX_AGE && draw(level, |r| r.next_int_bound(3)) == Some(0) {
                if age == MAX_AGE {
                    let above = up(pos, 1);
                    let empty = level.block_state(above).is_some_and(|s| is_air(level, s));
                    if empty
                        && panicles_survive(level, above)
                        && let Some(panicles) = with_age(level, PANICLES, 0)
                    {
                        level.set_block(above, panicles, flags());
                    }
                } else if let Some(next) =
                    level.state_with_property(event.state, "age", &(age + 1).to_string())
                {
                    level.set_block(pos, next, BlockUpdateFlags::CLIENTS);
                }
            }
        }
        Flow::Cancel
    }

    /// RiceBlock#canSurvive: its ground (the water is its own, always a full source).
    fn can_survive(&self, level: &dyn LevelHost, event: &mut CanSurviveEvent) -> Option<bool> {
        let below = level.block_state(up(at(event.pos), -1))?;
        Some(rice_ground(level, below))
    }

    fn may_place_on(&self, level: &dyn LevelHost, event: &mut MayPlaceOnEvent) -> Option<bool> {
        Some(rice_ground(level, event.ground))
    }

    /// RiceBlock#updateShape: VegetationBlock's (air when it cannot survive), else a water tick
    /// and `supporting` from the block above.
    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let pos = at(event.pos);
        let survives = level
            .block_state(up(pos, -1))
            .is_some_and(|s| rice_ground(level, s));
        if !survives {
            return level.block_state_id("minecraft:air");
        }
        level.schedule_fluid_tick(pos, "minecraft:water", 5);
        if event.direction == Direction::Up {
            let supporting = block_name(level, event.neighbour) == PANICLES;
            return level.state_with_property(
                event.state,
                "supporting",
                if supporting { "true" } else { "false" },
            );
        }
        Some(event.state)
    }

    /// RiceBlock#isValidBonemealTarget: not once the panicles above are ripe.
    fn is_valid_bonemeal_target(
        &self,
        level: LevelSide,
        event: &mut IsValidBonemealTargetEvent,
    ) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        let above = level.block_state(up(at(event.pos), 1));
        Some(match above {
            Some(s) if block_name(level, s) == PANICLES => age(level, s) < MAX_AGE,
            _ => true,
        })
    }

    fn is_bonemeal_success(&self, _: &mut dyn LevelRandomHost, event: &mut IsBonemealSuccessEvent) {
        event.success = true;
    }

    /// RiceBlock#performBonemeal: + Mth.nextInt(1, 4) stages (≤ 7); past max age the panicles
    /// above take the meal, or new panicles start with the rest.
    fn perform_bonemeal(&self, level: &mut dyn LevelWriteHost, event: &mut PerformBonemealEvent) {
        let pos = at(event.pos);
        let Some(inc) = draw(level, |r| 1 + r.next_int_bound(4)) else {
            return;
        };
        let growth = (age(level, event.state) + inc).min(7);
        if growth <= MAX_AGE {
            if let Some(next) = level.state_with_property(event.state, "age", &growth.to_string()) {
                level.set_block(pos, next, flags());
            }
            return;
        }
        let above = up(pos, 1);
        let Some(top) = level.block_state(above) else {
            return;
        };
        if block_name(level, top) == PANICLES {
            if age(level, top) < MAX_AGE {
                Panicles::grow(level, above, top);
            }
            return;
        }
        let rest = growth - MAX_AGE - 1;
        if is_air(level, top) && panicles_survive(level, above) {
            if let Some(max) = level.state_with_property(event.state, "age", "3") {
                level.set_block(pos, max, flags());
            }
            if let Some(panicles) = with_age(level, PANICLES, rest) {
                level.set_block(above, panicles, BlockUpdateFlags::CLIENTS);
            }
        }
    }
}

pub struct Panicles;

impl Panicles {
    /// CropBlock#growCrops with RicePaniclesBlock#getBonemealAgeIncrease (nextInt(2, 5) / 3).
    fn grow(level: &mut dyn LevelWriteHost, pos: [i32; 3], state: BlockState) {
        let Some(inc) = draw(level, |r| (2 + r.next_int_bound(4)) / 3) else {
            return;
        };
        let next_age = (age(level, state) + inc).min(MAX_AGE);
        if let Some(next) = level.state_with_property(state, "age", &next_age.to_string()) {
            level.set_block(pos, next, BlockUpdateFlags::CLIENTS);
        }
    }
}

#[crate::engine::part]
impl BlockPart for Panicles {
    /// CropBlock#randomTick (not beetroots' one-in-three): raw light ≥ 9, one in `25/speed + 1`.
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> Flow {
        let pos = at(event.pos);
        if bright_enough(level, pos, 9) {
            let age = age(level, event.state);
            if age < MAX_AGE {
                let speed = growth_speed(level, event.state, pos, true);
                let bound = (25.0 / speed) as i32 + 1;
                if draw(level, |r| r.next_int_bound(bound)) == Some(0)
                    && let Some(next) =
                        level.state_with_property(event.state, "age", &(age + 1).to_string())
                {
                    level.set_block(pos, next, BlockUpdateFlags::CLIENTS);
                }
            }
        }
        Flow::Cancel
    }

    fn can_survive(&self, level: &dyn LevelHost, event: &mut CanSurviveEvent) -> Option<bool> {
        Some(panicles_survive(level, at(event.pos)))
    }

    /// RicePaniclesBlock#mayPlaceOn: rice.
    fn may_place_on(&self, level: &dyn LevelHost, event: &mut MayPlaceOnEvent) -> Option<bool> {
        Some(block_name(level, event.ground) == RICE)
    }

    fn is_valid_bonemeal_target(
        &self,
        level: LevelSide,
        event: &mut IsValidBonemealTargetEvent,
    ) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(age(level, event.state) < MAX_AGE)
    }

    fn is_bonemeal_success(&self, _: &mut dyn LevelRandomHost, event: &mut IsBonemealSuccessEvent) {
        event.success = true;
    }

    fn perform_bonemeal(&self, level: &mut dyn LevelWriteHost, event: &mut PerformBonemealEvent) {
        Self::grow(level, at(event.pos), event.state);
    }
}

/// RiceItem#useOn: a failed placement on top of soil says the seed needs shallow water.
pub struct RiceItem;

#[crate::engine::part]
impl ItemPart for RiceItem {
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        // Server only, as at Horizon; the client predicts vanilla.
        let HostSide::Server(host) = host else {
            return None;
        };
        crate::compat::claimed((|| -> Flow {
            if event.face != [0, 1, 0] {
                return Flow::Continue;
            }
            let Some(player) = host.players().acting_player() else {
                return Flow::Continue;
            };
            let Some(level) = player.level(&*host) else {
                return Flow::Continue;
            };
            let Some(clicked) = level.block_state(&*host, event.pos) else {
                return Flow::Continue;
            };
            let target = up(event.pos, 1);
            let water = level
                .fluid_state(&*host, target)
                .is_some_and(|f| f.source && crate::blocks::rope::is_water(&LevelOf(&*host), f));
            let soil = ["minecraft:substrate_overworld", "minecraft:supports_crops"]
                .iter()
                .any(|t| host.block_in_tag(clicked.block(), t) == Some(true));
            if !water && soil {
                let _ = player.action_bar(
                    host,
                    Text::translate("block.farmersdelight.rice.invalid_placement", vec![]),
                );
            }
            Flow::Continue
        })())
    }
}

pub fn register(registrar: &mut dyn Registrar) {
    let rice = registrar.block_key(RICE);
    registrar.block_part(rice, Rice);
    let panicles = registrar.block_key(PANICLES);
    registrar.block_part(panicles, Panicles);
    let item = registrar.item_key(RICE);
    registrar.item_part(item, RiceItem);
}
