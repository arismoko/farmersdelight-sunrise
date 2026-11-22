//! Wild crops (PORT_PLAN M6-01): WildCropBlock ×6, SandyShrubBlock, WildRiceBlock.
//!
//! The six wild crops are FlowerBlocks in FD, which is where a brown mooshroom's suspicious stew
//! effect comes from; the game's flower table is fixed to vanilla's, so feeding one is handled in
//! `entity_interact` here (the stew itself is then vanilla's, from the cow's `stew_effects`).
use super::tomato::{at, block_name, draw, item_stack, up};
use crate::blocks::cutting_board::state_value;
use mod_api::*;
use std::sync::{Arc, Mutex};

pub const WILD_RICE: &str = "farmersdelight:wild_rice";
pub const SANDY_SHRUB: &str = "farmersdelight:sandy_shrub";
const GROWS_WILD_CROPS: &str = "farmersdelight:grows_wild_crops";
const SAND: &str = "minecraft:sand";

/// The wild crops with their suspicious stew effect and its seconds (ModBlocks).
pub const WILD_CROPS: [(&str, &str, i32); 6] = [
    ("farmersdelight:wild_cabbages", "minecraft:strength", 6),
    ("farmersdelight:wild_onions", "minecraft:fire_resistance", 6),
    ("farmersdelight:wild_tomatoes", "minecraft:poison", 10),
    ("farmersdelight:wild_carrots", "minecraft:mining_fatigue", 6),
    ("farmersdelight:wild_potatoes", "minecraft:nausea", 8),
    (
        "farmersdelight:wild_beetroots",
        "minecraft:water_breathing",
        8,
    ),
];

/// Items to pop at a block, asked for inside a level write that cannot spawn them
/// (WildRiceBlock#performBonemeal); dropped at the end of the same server tick.
pub type Drops = Arc<Mutex<Vec<(String, [i32; 3], &'static str)>>>;

pub fn drain(drops: &Drops, host: &mut dyn ServerHost) {
    let pending = std::mem::take(&mut *drops.lock().unwrap_or_else(|e| e.into_inner()));
    for (dimension, pos, item) in pending {
        if let Some(level) = host.level(&dimension)
            && let Some(stack) = item_stack(&*host, item, 1)
        {
            let _ = level.pop_resource(host, pos, stack);
        }
    }
}

fn ground_in(level: &dyn LevelHost, pos: [i32; 3], tag: &str) -> bool {
    level
        .block_state(up(pos, -1))
        .is_some_and(|s| level.block_in_tag(s.block(), tag) == Some(true))
}

fn is_air(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    level.block_state(pos).is_some_and(|s| {
        matches!(
            block_name(level, s),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

/// Wild crops registered through `api::wild_crop::register` (FD's six and add-ons'): block id,
/// suspicious stew effect, seconds.
static REGISTERED: std::sync::RwLock<Vec<(&'static str, &'static str, i32)>> =
    std::sync::RwLock::new(Vec::new());

/// `WildCropBlock#mayPlaceOn`: the ground is in `#farmersdelight:grows_wild_crops`.
pub fn may_place_on(level: &dyn LevelHost, ground: BlockState) -> bool {
    level.block_in_tag(ground.block(), GROWS_WILD_CROPS) == Some(true)
}

/// `canSurvive` of a wild crop at `pos`: the block below may hold it.
pub fn can_survive_at(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    ground_in(level, pos, GROWS_WILD_CROPS)
}

/// The suspicious stew effect and its seconds of a registered wild crop (its block id, which is
/// also its item id).
pub fn stew_effect(block: &str) -> Option<(&'static str, i32)> {
    REGISTERED
        .read()
        .ok()?
        .iter()
        .find(|(b, _, _)| *b == block)
        .map(|&(_, e, s)| (e, s))
}

/// Attaches the wild-crop Part to `block` and records its stew effect (`api::wild_crop::register`).
pub fn register_wild_crop(
    registrar: &mut dyn Registrar,
    block: &'static str,
    stew_effect: &'static str,
    seconds: i32,
) {
    let key = registrar.block_key(block);
    registrar.block_part(key, WildCrop);
    if let Ok(mut all) = REGISTERED.write() {
        all.retain(|(b, _, _)| *b != block);
        all.push((block, stew_effect, seconds));
    }
}

/// `WildCropBlock#performBonemeal`: unless ten of the same block stand in the 9×3×9 around
/// `origin`, one more `state` appears at the end of a short random walk over free spots it can
/// stand on.
pub fn bonemeal_spread(level: &mut dyn LevelWriteHost, origin: [i32; 3], state: BlockState) {
    let block = state.block();
    let mut limit = 10;
    for dx in -4..=4 {
        for dy in -1..=1 {
            for dz in -4..=4 {
                let p = [origin[0] + dx, origin[1] + dy, origin[2] + dz];
                if level.block_state(p).is_some_and(|s| s.block() == block) {
                    limit -= 1;
                    if limit <= 0 {
                        return;
                    }
                }
            }
        }
    }
    let free = |level: &dyn LevelHost, p: [i32; 3]| {
        is_air(level, p) && ground_in(level, p, GROWS_WILD_CROPS)
    };
    let step = |level: &mut dyn LevelWriteHost, from: [i32; 3]| {
        draw(level, |r| {
            let x = r.next_int_bound(3) - 1;
            let y = r.next_int_bound(2) - r.next_int_bound(2);
            let z = r.next_int_bound(3) - 1;
            [from[0] + x, from[1] + y, from[2] + z]
        })
    };
    let mut pos = origin;
    let Some(mut random_pos) = step(level, pos) else {
        return;
    };
    for _ in 0..4 {
        if free(level, random_pos) {
            pos = random_pos;
        }
        match step(level, pos) {
            Some(next) => random_pos = next,
            None => return,
        }
    }
    if free(level, random_pos) {
        level.set_block(random_pos, state, BlockUpdateFlags::CLIENTS);
    }
}

/// `WildCropBlock#isBonemealSuccess`: 80%.
pub fn bonemeal_success(level: &mut dyn LevelRandomHost) -> bool {
    draw(level, |r| r.next_float()).is_some_and(|f| f < 0.8)
}

pub struct WildCrop;

#[crate::engine::part]
impl BlockPart for WildCrop {
    /// WildCropBlock#mayPlaceOn: `#farmersdelight:grows_wild_crops`.
    fn may_place_on(&self, level: LevelSide<'_>, event: &mut MayPlaceOnEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(may_place_on(level, event.ground))
    }

    fn can_survive(&self, level: LevelSide<'_>, event: &mut CanSurviveEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(can_survive_at(level, at(event.pos)))
    }

    fn is_valid_bonemeal_target(
        &self,
        _: LevelSide,
        _: &mut IsValidBonemealTargetEvent,
    ) -> Option<bool> {
        Some(true)
    }

    fn is_bonemeal_success(
        &self,
        level: &mut dyn LevelRandomHost,
        event: &mut IsBonemealSuccessEvent,
    ) {
        event.success = bonemeal_success(level);
    }

    /// WildCropBlock#performBonemeal.
    fn perform_bonemeal(&self, level: &mut dyn LevelWriteHost, event: &mut PerformBonemealEvent) {
        bonemeal_spread(level, at(event.pos), event.state);
    }
}

pub struct SandyShrub;

#[crate::engine::part]
impl BlockPart for SandyShrub {
    /// SandyShrubBlock#mayPlaceOn: `#minecraft:sand`.
    fn may_place_on(&self, level: LevelSide<'_>, event: &mut MayPlaceOnEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(level.block_in_tag(event.ground.block(), SAND) == Some(true))
    }

    fn can_survive(&self, level: LevelSide<'_>, event: &mut CanSurviveEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(ground_in(level, at(event.pos), SAND))
    }

    /// SandyShrubBlock#isValidBonemealTarget is false (its performBonemeal is never reached).
    fn is_valid_bonemeal_target(
        &self,
        _: LevelSide,
        _: &mut IsValidBonemealTargetEvent,
    ) -> Option<bool> {
        Some(false)
    }
}

pub struct WildRice {
    pub drops: Drops,
}

fn full_water(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    level
        .fluid_state(pos)
        .is_some_and(|f| f.source && crate::blocks::rope::is_water(level, f))
}

#[crate::engine::part]
impl BlockPart for WildRice {
    /// WildRiceBlock#getStateForPlacement: into a full water block with air above; the lower
    /// half holds the water.
    fn state_for_placement(
        &self,
        level: LevelSide<'_>,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        let p = event.context.pos;
        let pos = [p.x, p.y, p.z];
        let top = level.min_y() + level.height() - 1;
        if pos[1] >= top - 1
            || !(event.fluid.source && crate::blocks::rope::is_water(level, event.fluid))
            || !is_air(level, up(pos, 1))
        {
            return Some(None);
        }
        Some(level.state_with_property(event.state, "waterlogged", "true"))
    }

    fn may_place_on(&self, level: LevelSide<'_>, event: &mut MayPlaceOnEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(level.block_in_tag(event.ground.block(), GROWS_WILD_CROPS) == Some(true))
    }

    /// WildRiceBlock#canSurvive: the lower half on its ground in a full water block, the upper
    /// half on the lower.
    fn can_survive(&self, level: LevelSide<'_>, event: &mut CanSurviveEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        let pos = at(event.pos);
        let lower = level
            .state_string(event.state)
            .is_none_or(|s| state_value(&s, "half") != Some("upper"));
        // FD's lower half reports a water source itself (getFluidState); the engine reads the
        // tall grass template's fluid instead (none), so a placed lower half counts as water here.
        let placed_lower = level.block_state(pos).is_some_and(|s| {
            block_name(level, s) == WILD_RICE
                && level
                    .state_string(s)
                    .is_some_and(|n| state_value(&n, "half") != Some("upper"))
        });
        Some(if lower {
            ground_in(level, pos, GROWS_WILD_CROPS) && (placed_lower || full_water(level, pos))
        } else {
            level.block_state(up(pos, -1)).is_some_and(|s| {
                block_name(level, s) == WILD_RICE
                    && level
                        .state_string(s)
                        .is_some_and(|n| state_value(&n, "half") == Some("lower"))
            })
        })
    }

    fn is_valid_bonemeal_target(
        &self,
        _: LevelSide,
        _: &mut IsValidBonemealTargetEvent,
    ) -> Option<bool> {
        Some(true)
    }

    fn is_bonemeal_success(
        &self,
        level: &mut dyn LevelRandomHost,
        event: &mut IsBonemealSuccessEvent,
    ) {
        event.success = draw(level, |r| r.next_float()).is_some_and(|f| f < 0.3);
    }

    /// The lower half's water stays when it is removed (its getFluidState is a water source);
    /// the engine reads the template's fluid instead (E-13).
    fn remove_block(
        &self,
        level: &dyn LevelHost,
        event: &mut RemoveBlockEvent,
    ) -> Option<BlockState> {
        let name = level.state_string(event.state)?;
        (state_value(&name, "half") != Some("upper"))
            .then(|| level.block_state_id("minecraft:water"))
            .flatten()
    }

    /// WildRiceBlock#performBonemeal: pops one wild rice.
    fn perform_bonemeal(&self, _: &mut dyn LevelWriteHost, event: &mut PerformBonemealEvent) {
        self.drops.lock().unwrap_or_else(|e| e.into_inner()).push((
            event.dimension.key().id().to_string(),
            at(event.pos),
            WILD_RICE,
        ));
    }
}

/// The wild rice table's pools need the other half in place (`location_check` with `offsetY`
/// and a block predicate), which in FD leaves a broken plant one drop, from the half the
/// player broke (DoublePlantBlock#playerWillDestroy rolls it while both halves stand). The
/// engine's `location_check` reads only `biomes` (E-23), so the half that pops when its partner
/// goes would drop too, in creative as well: a half popped without a breaker whose partner is
/// gone drops nothing.
pub fn block_drops(level: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) {
    if event.breaker.is_some() || block_name(level, event.state) != WILD_RICE {
        return;
    }
    let half_of = |s: BlockState| {
        level
            .state_string(s)
            .and_then(|n| state_value(&n, "half").map(str::to_owned))
            .unwrap_or_default()
    };
    let pos = at(event.pos);
    let (partner, half) = if half_of(event.state) == "upper" {
        (up(pos, -1), "lower")
    } else {
        (up(pos, 1), "upper")
    };
    let paired = level
        .block_state(partner)
        .is_some_and(|s| block_name(level, s) == WILD_RICE && half_of(s) == half);
    if !paired {
        event.drops.clear();
    }
}

/// MushroomCow#mobInteract's flower branch for the wild crops (FlowerBlock's
/// SuspiciousEffectHolder): a brown mooshroom takes the crop's effect unless it holds one.
pub fn entity_interact(host: &mut dyn ServerHost, event: &mut EntityInteractEvent<'_>) -> Flow {
    if event.kind != "minecraft:mooshroom" {
        return Flow::Continue;
    }
    let Some(stack) = event.stack else {
        return Flow::Continue;
    };
    let id = stack.item().id(&*host);
    let Some((effect, seconds)) = stew_effect(&id.to_string()) else {
        return Flow::Continue;
    };
    let Some(cow) = host.entity(event.entity as u32) else {
        return Flow::Continue;
    };
    let Some(nbt) = cow.nbt(host) else {
        return Flow::Continue;
    };
    if !matches!(nbt.get(&"Type".into()), Some(Tag::String(t)) if t.to_string_lossy() == "brown") {
        return Flow::Continue;
    }
    let Some(player) = host.players().acting_player() else {
        return Flow::Continue;
    };
    let at = cow.position(&*host).unwrap_or([0.0; 3]);
    let [_, height] = cow.dimensions(&*host).unwrap_or([0.9, 1.4]);
    let level = player.level(&*host);
    let holds_effects = nbt.contains_key(&"stew_effects".into());
    if holds_effects {
        // Two smoke puffs: it already holds effects.
        if let (Some(level), Some(smoke)) = (level, SMOKE.get()) {
            let _ = level.send_particles(
                host,
                &ParticleOptions::simple(*smoke),
                None,
                [at[0], at[1] + height * 0.5, at[2]],
                [0.25, 0.25, 0.25],
                0.0,
                2,
            );
        }
    } else {
        let mut entry = Compound::new();
        entry.insert("id".into(), Tag::String(effect.into()));
        entry.insert("duration".into(), Tag::Int(seconds * 20));
        let mut merge = Compound::new();
        merge.insert("stew_effects".into(), Tag::List(vec![Tag::Compound(entry)]));
        if !cow.merge_nbt(host, merge) {
            return Flow::Continue;
        }
        if !player.is_creative(&*host).unwrap_or(false)
            && let Some(holder) = host.living(player.entity())
        {
            let _ = holder.set_hand_item(host, event.hand, stack.clone().shrink(1));
        }
        if let Some(level) = level {
            level.play_sound(
                host,
                "minecraft:entity.mooshroom.eat",
                at,
                SoundSource::Neutral,
                2.0,
                1.0,
                None,
            );
        }
    }
    Flow::Cancel
}

static SMOKE: std::sync::OnceLock<ParticleKey> = std::sync::OnceLock::new();

pub fn register(registrar: &mut dyn Registrar, drops: &Drops) {
    let _ = SMOKE.set(registrar.particle_key("minecraft:smoke"));
    for (crop, effect, seconds) in WILD_CROPS {
        register_wild_crop(registrar, crop, effect, seconds);
    }
    let shrub = registrar.block_key(SANDY_SHRUB);
    registrar.block_part(shrub, SandyShrub);
    let rice = registrar.block_key(WILD_RICE);
    registrar.block_part(
        rice,
        WildRice {
            drops: drops.clone(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BX-05: an add-on wild crop recorded through the registry gives its stew effect; FD's own
    /// table is unchanged and replacing an entry keeps one.
    #[test]
    fn stew_registry() {
        if let Ok(mut all) = REGISTERED.write() {
            for (crop, effect, seconds) in WILD_CROPS {
                all.retain(|(b, _, _)| *b != crop);
                all.push((crop, effect, seconds));
            }
            all.push(("testaddon:wild_corn", "minecraft:speed", 1));
            all.retain(|(b, _, _)| *b != "testaddon:wild_corn");
            all.push(("testaddon:wild_corn", "minecraft:saturation", 8));
        }
        assert_eq!(
            stew_effect("testaddon:wild_corn"),
            Some(("minecraft:saturation", 8))
        );
        assert_eq!(
            stew_effect("farmersdelight:wild_tomatoes"),
            Some(("minecraft:poison", 10))
        );
        assert_eq!(stew_effect("minecraft:dandelion"), None);
    }
}
