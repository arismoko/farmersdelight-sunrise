//! Tomatoes (PORT_PLAN M5-03): `budding_tomatoes` (BuddingTomatoBlock) → `tomatoes` (TomatoBlock)
//! → `tomatoes_on_rope` (HangingTomatoBlock) climbing up to three ropes.
//!
//! The blocks are declared on crop templates (wheat for the budding bush, beetroots — a CropBlock
//! with `age` 0-3 — for the vines), which bring the crop properties, survival on
//! `#minecraft:supports_crops` (answered in `may_place_on`) and bone meal routing. Growth, bone
//! meal, harvest, climbing and rope permanence follow the reference here.
use crate::blocks::cutting_board::state_value;
use mod_api::*;

pub const BUDDING: &str = "farmersdelight:budding_tomatoes";
pub const TOMATOES: &str = "farmersdelight:tomatoes";
pub const HANGING: &str = "farmersdelight:tomatoes_on_rope";
const TOMATO: &str = "farmersdelight:tomato";
const ROTTEN: &str = "farmersdelight:rotten_tomato";
const PICK_SOUND: &str = "farmersdelight:block.tomatoes.pick_tomatoes";
/// BuddingBushBlock.MAX_AGE (AGE runs to 4, the transition state).
const BUDDING_MAX: i32 = 3;
/// TomatoBlock#getMaxAge.
const VINE_MAX: i32 = 3;

pub(crate) fn age(level: &dyn LevelHost, state: BlockState) -> i32 {
    level
        .state_string(state)
        .and_then(|s| state_value(&s, "age").and_then(|v| v.parse().ok()))
        .unwrap_or(0)
}

pub(crate) fn block_name(level: &dyn LevelHost, state: BlockState) -> &str {
    level.state_block(state).unwrap_or("")
}

pub(crate) fn at(pos: BlockPos) -> [i32; 3] {
    [pos.x, pos.y, pos.z]
}

pub(crate) fn up(pos: [i32; 3], n: i32) -> [i32; 3] {
    [pos[0], pos[1] + n, pos[2]]
}

/// A state of `block` with `age`, from its default (`getStateForAge`).
pub(crate) fn with_age(level: &dyn LevelHost, block: &str, age: i32) -> Option<BlockState> {
    let base = level.block_state_id(block)?;
    level.state_with_property(base, "age", &age.to_string())
}

fn is_vine(name: &str) -> bool {
    name == TOMATOES || name == HANGING
}

/// Draws from the level's random and stores the stream back, so draws move the level's own.
pub(crate) fn draw<T>(
    level: &mut dyn LevelRandomHost,
    f: impl FnOnce(&mut LegacyRandomSource) -> T,
) -> Option<T> {
    let mut random = level.random()?;
    let out = f(&mut random);
    level.set_random(random);
    Some(out)
}

/// The two growth-speed rules: `vanilla` is CropBlock.getGrowthSpeed (a block in
/// `#minecraft:grows_crops` counts 1, moist 3), FD's BuddingBushBlock version counts every block
/// below as 1 and any with `moisture` > 0 as 3. Neighbours of the same block in both rows, or on a
/// diagonal, halve it.
pub(crate) fn growth_speed(
    level: &dyn LevelHost,
    state: BlockState,
    pos: [i32; 3],
    vanilla: bool,
) -> f32 {
    let mut speed = 1.0f32;
    for dx in -1..=1 {
        for dz in -1..=1 {
            let below = [pos[0] + dx, pos[1] - 1, pos[2] + dz];
            let mut bonus = 0.0f32;
            if let Some(s) = level.block_state(below) {
                let counts = !vanilla
                    || level.block_in_tag(s.block(), "minecraft:grows_crops") == Some(true);
                if counts {
                    bonus = 1.0;
                    let moist = level
                        .state_string(s)
                        .and_then(|n| {
                            state_value(&n, "moisture").and_then(|m| m.parse::<i32>().ok())
                        })
                        .is_some_and(|m| m > 0);
                    if moist {
                        bonus = 3.0;
                    }
                }
            }
            if dx != 0 || dz != 0 {
                bonus /= 4.0;
            }
            speed += bonus;
        }
    }
    let block = state.block();
    let same = |d: [i32; 3]| {
        level
            .block_state([pos[0] + d[0], pos[1], pos[2] + d[2]])
            .is_some_and(|s| s.block() == block)
    };
    let east_west = same([-1, 0, 0]) || same([1, 0, 0]);
    let north_south = same([0, 0, -1]) || same([0, 0, 1]);
    let diagonal = same([-1, 0, -1]) || same([1, 0, -1]) || same([1, 0, 1]) || same([-1, 0, 1]);
    if (east_west && north_south) || diagonal {
        speed /= 2.0;
    }
    speed
}

pub(crate) fn on_supports_crops(level: &dyn LevelHost, ground: BlockState) -> bool {
    level.block_in_tag(ground.block(), "minecraft:supports_crops") == Some(true)
}

pub(crate) fn bright_enough(level: &dyn LevelHost, pos: [i32; 3], min: i32) -> bool {
    level.raw_brightness(pos, 0) >= min
}

// ---------------------------------------------------------------------------------------------
// budding_tomatoes

pub struct BuddingTomato;

#[crate::engine::part]
impl BlockPart for BuddingTomato {
    /// BuddingBushBlock#randomTick: at raw light ≥ 9, one in `25/speed + 1` grows a stage; at
    /// max age it grows past into `tomatoes` (BuddingTomatoBlock#growPastMaxAge).
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> Flow {
        let pos = at(event.pos);
        if bright_enough(level, pos, 9) {
            let age = age(level, event.state);
            if age <= BUDDING_MAX {
                let speed = growth_speed(level, event.state, pos, false);
                let bound = (25.0 / speed) as i32 + 1;
                if draw(level, |r| r.next_int_bound(bound)) == Some(0) {
                    let next = if age >= BUDDING_MAX {
                        with_age(level, TOMATOES, 0)
                    } else {
                        level.state_with_property(event.state, "age", &(age + 1).to_string())
                    };
                    if let Some(next) = next {
                        level.set_block(
                            pos,
                            next,
                            BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
                        );
                    }
                }
            }
        }
        Flow::Cancel
    }

    /// BuddingTomatoBlock#updateShape: the transition state (age 4) turns into `tomatoes`.
    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        if age(level, event.state) == 4 {
            return with_age(level, TOMATOES, 0);
        }
        None
    }

    /// BuddingBushBlock#mayPlaceOn: `#minecraft:supports_crops`.
    fn may_place_on(&self, level: LevelSide<'_>, event: &mut MayPlaceOnEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(on_supports_crops(level, event.ground))
    }

    fn is_valid_bonemeal_target(
        &self,
        _: LevelSide,
        _: &mut IsValidBonemealTargetEvent,
    ) -> Option<bool> {
        Some(true)
    }

    fn is_bonemeal_success(&self, _: &mut dyn LevelRandomHost, event: &mut IsBonemealSuccessEvent) {
        event.success = true;
    }

    /// BuddingTomatoBlock#performBonemeal: + Mth.nextInt(1, 4) stages (≤ 7); past max age the
    /// rest carries into the vine's age (`ageGrowth - maxAge - 1`).
    fn perform_bonemeal(&self, level: &mut dyn LevelWriteHost, event: &mut PerformBonemealEvent) {
        let pos = at(event.pos);
        let Some(inc) = draw(level, |r| 1 + r.next_int_bound(4)) else {
            return;
        };
        let growth = (age(level, event.state) + inc).min(7);
        let next = if growth <= BUDDING_MAX {
            level.state_with_property(event.state, "age", &growth.to_string())
        } else {
            with_age(level, TOMATOES, growth - BUDDING_MAX - 1)
        };
        if let Some(next) = next {
            level.set_block(
                pos,
                next,
                BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// tomatoes and tomatoes_on_rope

pub struct Tomato {
    pub hanging: bool,
}

impl Tomato {
    fn own(&self) -> &'static str {
        if self.hanging { HANGING } else { TOMATOES }
    }

    /// TomatoBlock#canClimbBlock: rope tagged `#farmersdelight:ropes` (config default) — FD's
    /// rope only when the option is off.
    fn can_climb(level: &dyn LevelHost, state: BlockState) -> bool {
        if crate::config::tomato_vine_climbing_tagged_ropes() {
            level.block_in_tag(state.block(), "farmersdelight:ropes") == Some(true)
        } else {
            block_name(level, state) == crate::blocks::rope::ROPE
        }
    }

    /// TomatoBlock#climbRopeAbove: onto a rope above while fewer than 3 of this block stand
    /// at and below `pos`.
    fn climb_rope_above(&self, level: &mut dyn LevelWriteHost, pos: [i32; 3]) {
        let above = up(pos, 1);
        let Some(state_above) = level.block_state(above) else {
            return;
        };
        if !Self::can_climb(level, state_above) {
            return;
        }
        let own = self.own();
        let mut height = 1;
        while level
            .block_state(up(pos, -height))
            .is_some_and(|s| block_name(level, s) == own)
        {
            height += 1;
        }
        if height < 3
            && let Some(hanging) = with_age(level, HANGING, 0)
        {
            level.set_block(
                above,
                hanging,
                BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
            );
        }
    }

    /// TomatoBlock#isValidBonemealTarget.
    fn valid_bonemeal(level: &dyn LevelHost, pos: [i32; 3], state: BlockState) -> bool {
        if age(level, state) < VINE_MAX {
            return true;
        }
        for h in 1..=2 {
            let Some(next) = level.block_state(up(pos, h)) else {
                return false;
            };
            if Self::can_climb(level, next) {
                return true;
            }
            if block_name(level, next) == HANGING {
                if age(level, next) < VINE_MAX {
                    return true;
                }
            } else {
                return false;
            }
        }
        false
    }

    /// TomatoBlock#performBonemeal (recursing up the vine through `tomatoes_on_rope`).
    fn bonemeal(&self, level: &mut dyn LevelWriteHost, pos: [i32; 3], state: BlockState) {
        if self.convert_ropelogged(level, pos, state) {
            return;
        }
        // CropBlock#getBonemealAgeIncrease (Mth.nextInt(random, 2, 5)) / 2.
        let Some(inc) = draw(level, |r| (2 + r.next_int_bound(4)) / 2) else {
            return;
        };
        let new_age = age(level, state) + inc;
        if new_age <= VINE_MAX {
            if let Some(next) = level.state_with_property(state, "age", &new_age.to_string()) {
                level.set_block(
                    pos,
                    next,
                    BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
                );
            }
            if draw(level, |r| r.next_float()).is_some_and(|f| f < 0.3) {
                self.climb_rope_above(level, pos);
            }
            return;
        }
        let above = up(pos, 1);
        let Some(above_state) = level.block_state(above) else {
            return;
        };
        if Self::can_climb(level, above_state) {
            self.climb_rope_above(level, pos);
        } else if block_name(level, above_state) == HANGING
            && Self::valid_bonemeal(level, above, above_state)
        {
            Tomato { hanging: true }.bonemeal(level, above, above_state);
        }
    }

    /// TomatoBlock's legacy `ropelogged` tomatoes turn into `tomatoes_on_rope` of the same age.
    fn convert_ropelogged(
        &self,
        level: &mut dyn LevelWriteHost,
        pos: [i32; 3],
        state: BlockState,
    ) -> bool {
        if self.hanging
            || !level
                .state_string(state)
                .is_some_and(|s| s.contains("ropelogged=true"))
        {
            return false;
        }
        if let Some(hanging) = with_age(level, HANGING, age(level, state)) {
            level.set_block(
                pos,
                hanging,
                BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
            );
        }
        true
    }
}

#[crate::engine::part]
impl BlockPart for Tomato {
    /// TomatoBlock#randomTick: grows at raw light ≥ 9 (one in `25/speed + 1`, flag 2) and then
    /// tries to climb the rope above, every tick while bright enough.
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> Flow {
        let pos = at(event.pos);
        if self.convert_ropelogged(level, pos, event.state) {
            return Flow::Cancel;
        }
        if bright_enough(level, pos, 9) {
            let age = age(level, event.state);
            if age < VINE_MAX {
                let speed = growth_speed(level, event.state, pos, true);
                let bound = (25.0 / speed) as i32 + 1;
                if draw(level, |r| r.next_int_bound(bound)) == Some(0)
                    && let Some(next) =
                        level.state_with_property(event.state, "age", &(age + 1).to_string())
                {
                    level.set_block(pos, next, BlockUpdateFlags::CLIENTS);
                }
            }
            self.climb_rope_above(level, pos);
        }
        Flow::Cancel
    }

    /// TomatoBlock#canSurvive: above another tomato vine only the light matters; otherwise the
    /// crop's own rule (light and `#minecraft:supports_crops` below).
    fn can_survive(&self, level: LevelSide<'_>, event: &mut CanSurviveEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        let pos = at(event.pos);
        let below = level.block_state(up(pos, -1))?;
        if is_vine(block_name(level, below)) {
            return Some(bright_enough(level, pos, 8));
        }
        if self.hanging {
            // CropBlock#canSurvive: (light ≥ 8 or sky) and mayPlaceOn(below).
            return Some(bright_enough(level, pos, 8) && on_supports_crops(level, below));
        }
        None
    }

    fn may_place_on(&self, level: LevelSide<'_>, event: &mut MayPlaceOnEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(on_supports_crops(level, event.ground))
    }

    /// HangingTomatoBlock: a vine that can no longer survive is destroyed and leaves its rope
    /// (`tick` → `destroyBlock` → `onRemove` → `placeRope`). The template's shape update would
    /// remove it to air without asking `remove_block`, so the rope is put in place here; the
    /// ripe vine's 1-2 tomatoes are not dropped on this path (delta A20).
    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        if !self.hanging || !crate::config::tomato_rope_permanence() {
            return None;
        }
        let pos = at(event.pos);
        let below = level.block_state(up(pos, -1))?;
        let survives = if is_vine(block_name(level, below)) {
            bright_enough(level, pos, 8)
        } else {
            bright_enough(level, pos, 8) && on_supports_crops(level, below)
        };
        if survives {
            return None;
        }
        crate::blocks::rope::rope_state_for_vine(level, pos)
    }

    /// TomatoBlock#useItemOn: bone meal on an unripe vine goes to the bone meal; anything else
    /// reaches the empty-hand use (the harvest works with any item held).
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        let mature = age(&LevelOf(&*host), event.state) == VINE_MAX;
        let bone_meal = event
            .click
            .stack
            .is_some_and(|s| s.item().id(&*host) == "minecraft:bone_meal");
        Some(if !mature && bone_meal {
            InteractionResult::Pass
        } else {
            InteractionResult::TryWithEmptyHand
        })
    }

    /// TomatoBlock#useWithoutItem: a ripe vine drops 1-2 tomatoes (5%: a rotten one too), plays
    /// the pick sound and goes back to age 0 (flag 2).
    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        if age(&LevelOf(&*host), event.state) != VINE_MAX {
            return None;
        }
        let click = event.click;
        let level = host.level(click.dimension.key().id())?;
        let pos = click.pos;
        let mut tomato = item_stack(&*host, TOMATO, 1)?;
        let quantity = level
            .with_random(host, |r| 1 + r.next_int_bound(2))
            .unwrap_or(1);
        let _ = tomato.set_count(quantity as u32);
        let _ = level.pop_resource(host, pos, tomato);
        if level
            .with_random(host, |r| r.next_float())
            .is_some_and(|f| f < 0.05)
            && let Some(rotten) = item_stack(&*host, ROTTEN, 1)
        {
            let _ = level.pop_resource(host, pos, rotten);
        }
        let pitch = 0.8 + level.with_random(host, |r| r.next_float()).unwrap_or(0.5) * 0.4;
        let centre = [
            pos[0] as f64 + 0.5,
            pos[1] as f64 + 0.5,
            pos[2] as f64 + 0.5,
        ];
        level.play_sound(
            host,
            PICK_SOUND,
            centre,
            SoundSource::Blocks,
            1.0,
            pitch,
            None,
        );
        let name = host.block_state_string(event.state)?;
        let reset = host.block_state_id(&crate::items::skillet::set_prop(&name, "age", "0"))?;
        let _ = level.set_block(host, pos, reset, BlockUpdateFlags::CLIENTS);
        Some(InteractionResult::Success)
    }

    fn is_valid_bonemeal_target(
        &self,
        level: LevelSide,
        event: &mut IsValidBonemealTargetEvent,
    ) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(Self::valid_bonemeal(level, at(event.pos), event.state))
    }

    fn is_bonemeal_success(&self, _: &mut dyn LevelRandomHost, event: &mut IsBonemealSuccessEvent) {
        // CropBlock#isBonemealSuccess: always.
        event.success = true;
    }

    fn perform_bonemeal(&self, level: &mut dyn LevelWriteHost, event: &mut PerformBonemealEvent) {
        self.bonemeal(level, at(event.pos), event.state);
    }

    /// HangingTomatoBlock (ReplaceOnRemoval, config `enableTomatoRopePermanence`) and the legacy
    /// ropelogged vine (TomatoBlock#playerDestroy): removing it leaves the default vine rope,
    /// unless a piston moves it.
    fn remove_block(
        &self,
        level: &dyn LevelHost,
        event: &mut RemoveBlockEvent,
    ) -> Option<BlockState> {
        if event.moved_by_piston {
            return None;
        }
        let rope_left = if self.hanging {
            crate::config::tomato_rope_permanence()
        } else {
            level
                .state_string(event.state)
                .is_some_and(|s| s.contains("ropelogged=true"))
        };
        if !rope_left {
            return None;
        }
        crate::blocks::rope::rope_state_for_vine(level, at(event.pos))
    }
}

/// A read-only `LevelHost` view over a `Host` (for helpers shared with the level-host hooks).
pub(crate) struct LevelOf<'a>(pub(crate) &'a dyn Host);

impl LevelHost for LevelOf<'_> {
    fn state_string(&self, state: BlockState) -> Option<String> {
        self.0.block_state_string(state)
    }
    fn block_state_id(&self, name: &str) -> Option<BlockState> {
        self.0.block_state_id(name)
    }
    fn state_fluid_state(&self, state: BlockState) -> Option<FluidState> {
        self.0.state_fluid_state(state)
    }
    fn block_in_tag(&self, block: BlockKey, tag: &str) -> Option<bool> {
        self.0.block_in_tag(block, tag)
    }
}

pub(crate) fn item_stack(host: &dyn Host, id: &str, count: u32) -> Option<ItemStack> {
    let key = host.item_key(&Identifier::parse(id).ok()?)?;
    ItemStack::new(key, count)
}

pub fn register(registrar: &mut dyn Registrar) {
    let budding = registrar.block_key(BUDDING);
    registrar.block_part(budding, BuddingTomato);
    let tomatoes = registrar.block_key(TOMATOES);
    registrar.block_part(tomatoes, Tomato { hanging: false });
    let hanging = registrar.block_key(HANGING);
    registrar.block_part(hanging, Tomato { hanging: true });
}
