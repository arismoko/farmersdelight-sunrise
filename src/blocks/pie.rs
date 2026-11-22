//! Pies (J/common/block/PieBlock.java; PORT_PLAN M7-02) and the vanilla pumpkin pie placing
//! `farmersdelight:pumpkin_pie` (J/common/mixin/PlacePumpkinPieMixin.java).
//!
//! A pie has four slices (`bites` 0-3). An empty-hand use (or one with any non-knife item, through
//! TRY_WITH_EMPTY_HAND, main hand only) eats a slice: the slice's food and consume effects (Speed)
//! apply as if eaten, the pie loses a bite and is removed after the fourth. A knife in either hand
//! cuts a slice off as an item instead. Comparator output is 4 − bites.
use crate::blocks::cutting_board::{facing_for_yaw, state_value};
use crate::items::skillet::set_prop;
use mod_api::*;

pub const APPLE_PIE: &str = "farmersdelight:apple_pie";
pub const SWEET_BERRY_CHEESECAKE: &str = "farmersdelight:sweet_berry_cheesecake";
pub const CHOCOLATE_PIE: &str = "farmersdelight:chocolate_pie";
pub const PUMPKIN_PIE: &str = "farmersdelight:pumpkin_pie";
pub const PIES: [&str; 4] = [
    APPLE_PIE,
    SWEET_BERRY_CHEESECAKE,
    CHOCOLATE_PIE,
    PUMPKIN_PIE,
];
/// The three pies with an item of their own (PlaceableItem, "Placeable" tooltip).
pub const PIE_ITEMS: [&str; 3] = [APPLE_PIE, SWEET_BERRY_CHEESECAKE, CHOCOLATE_PIE];
const VANILLA_PUMPKIN_PIE: &str = "minecraft:pumpkin_pie";

/// `getMaxBites()`.
pub const MAX_BITES: u32 = 4;
const KNIFE_TAG: &str = "c:tools/knife";
/// FoodValues.PIE_SLICE: nutrition 3, saturation modifier 0.3 (all four slices).
const SLICE_NUTRITION: i32 = 3;
const SLICE_SATURATION: f32 = 0.3;

// ---------------------------------------------------------------- pure rules

pub fn slice_of(block: &str) -> Option<&'static str> {
    Some(match block {
        APPLE_PIE => "farmersdelight:apple_pie_slice",
        SWEET_BERRY_CHEESECAKE => "farmersdelight:sweet_berry_cheesecake_slice",
        CHOCOLATE_PIE => "farmersdelight:chocolate_pie_slice",
        PUMPKIN_PIE => "farmersdelight:pumpkin_pie_slice",
        _ => return None,
    })
}

/// After a slice is taken: the next `bites`, or None when the pie is gone.
pub fn after_bite(bites: u32) -> Option<u32> {
    (bites < MAX_BITES - 1).then_some(bites + 1)
}

/// `getAnalogOutputSignal`.
pub fn comparator(bites: u32) -> i32 {
    MAX_BITES as i32 - bites as i32
}

/// `getStateForPlacement`: `context.getHorizontalDirection()`, the way the placer looks.
pub fn facing_looking(yaw: f32) -> &'static str {
    match facing_for_yaw(yaw) {
        "north" => "south",
        "south" => "north",
        "east" => "west",
        _ => "east",
    }
}

/// What a click predicts on the client (`PieBlock#useItemOn`/`useWithoutItem` there): a knife cuts
/// (SUCCESS); the main hand eats when the player can eat (SUCCESS), else CONSUME with an empty hand,
/// else PASS on to the held item. The off hand without a knife passes.
pub fn predict(knife: bool, main_hand: bool, can_eat: bool, empty_hand: bool) -> InteractionResult {
    if knife {
        InteractionResult::Success
    } else if !main_hand {
        InteractionResult::Pass
    } else if can_eat {
        InteractionResult::Success
    } else if empty_hand {
        InteractionResult::Consume
    } else {
        InteractionResult::Pass
    }
}

fn bites_of(name: Option<&str>) -> u32 {
    name.and_then(|n| state_value(n, "bites"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

fn block_id(name: &str) -> &str {
    name.split('[').next().unwrap_or(name)
}

fn is_knife(host: &dyn Host, stack: Option<&ItemStack>) -> bool {
    stack.is_some_and(|s| crate::blocks::stove::in_item_tag(host, s, KNIFE_TAG))
}

fn stack_of(host: &dyn Host, id: &str) -> Option<ItemStack> {
    ItemStack::new(host.item_key(&Identifier::parse(id).ok()?)?, 1)
}

/// `canSupportRigidBlock(below)` through a level handle, approximated as the block Part's (A28).
fn floor_supports(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> bool {
    let Some(state) = level.block_state(host, [pos[0], pos[1] - 1, pos[2]]) else {
        return false;
    };
    let name = host.block_state_string(state);
    crate::blocks::feast::rigid_floor(name.as_deref(), |t| {
        host.block_in_tag(state.block(), t) == Some(true)
    })
}

// ---------------------------------------------------------------- the block Part

pub struct Pie {
    pub block: &'static str,
    pub block_particle: ParticleKey,
}

#[crate::engine::part]
impl BlockPart for Pie {
    fn state_for_placement(
        &self,
        level: LevelSide<'_>,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        let facing = facing_looking(event.context.yaw);
        Some(level.state_with_property(event.state, "facing", facing))
    }

    fn can_survive(&self, level: LevelSide<'_>, event: &mut CanSurviveEvent) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(crate::blocks::feast::supported(
            level,
            [event.pos.x, event.pos.y, event.pos.z],
        ))
    }

    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        if event.direction == Direction::Down && !crate::blocks::feast::supported(&*level, pos) {
            return level.block_state_id("minecraft:air");
        }
        None
    }

    /// `useItemOn`: a knife (either hand) cuts a slice; anything else tries the empty hand.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        if !is_knife(&*host, event.click.stack) {
            return Some(InteractionResult::TryWithEmptyHand);
        }
        Some(self.cut_slice(host, event.click, event.state))
    }

    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        Some(self.consume_bite(host, event.click, event.state))
    }

    fn has_analog_output(&self, _: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }

    fn analog_output(&self, level: &mut dyn LevelRandomHost, event: &mut AnalogOutputEvent) {
        let level: &dyn LevelHost = level;
        event.output = comparator(bites_of(level.state_string(event.state).as_deref()));
    }

    fn is_pathfindable(&self, _: &dyn Host, event: &mut IsPathfindableEvent) {
        event.pathfindable = false;
    }
}

impl Pie {
    /// Takes a bite off (or removes the pie, without drops) and plays the bite's effects.
    fn take_bite(&self, host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], bites: u32) {
        match after_bite(bites) {
            Some(next) => {
                host.set_block_property_with(pos, "bites", &next.to_string(), 3);
            }
            None => {
                let _ = level.remove_block(host, pos, false);
            }
        }
    }

    fn particles(
        &self,
        host: &mut dyn ServerHost,
        level: LevelRef,
        pos: [i32; 3],
        state: BlockState,
    ) {
        let _ = level.send_particles(
            host,
            &ParticleOptions::simple(self.block_particle),
            Some(state),
            [
                pos[0] as f64 + 0.5,
                pos[1] as f64 + 0.3,
                pos[2] as f64 + 0.5,
            ],
            [0.1, 0.1, 0.1],
            0.001,
            3,
        );
    }

    /// `consumeBite`: the slice's food (FoodProperties#onConsume: eat sound, food, burp) and
    /// consume effects as if eaten, then the bite, the eat sound at the pie and its particles.
    fn consume_bite(
        &self,
        host: &mut dyn ServerHost,
        click: &BlockClick<'_>,
        state: BlockState,
    ) -> InteractionResult {
        let Some(player) = host.players().acting_player() else {
            return InteractionResult::Pass;
        };
        if player.can_eat(&*host, false) != Some(true) {
            return InteractionResult::Pass;
        }
        let Some(level) = host.level(click.dimension.key().id()) else {
            return InteractionResult::Pass;
        };
        let pos = click.pos;
        let bites = bites_of(host.block_state_string(state).as_deref());
        let feet = player.entity().position(&*host).unwrap_or([
            pos[0] as f64 + 0.5,
            pos[1] as f64,
            pos[2] as f64 + 0.5,
        ]);
        let mut random = level.random(&*host);
        let mut next = || random.as_mut().map_or(0.5, |r| r.next_float());
        // Consumable.sound (entity.generic.eat), NEUTRAL, pitch random.triangle(1.0, 0.4).
        let eat_pitch = 1.0 + 0.4 * (next() - next());
        let burp_pitch = next() * 0.1 + 0.9;
        if let Some(r) = random {
            level.set_random(host, r);
        }
        level.play_sound(
            host,
            "minecraft:entity.generic.eat",
            feet,
            SoundSource::Neutral,
            1.0,
            eat_pitch,
            None,
        );
        let _ = player.feed(host, SLICE_NUTRITION, SLICE_SATURATION);
        level.play_sound(
            host,
            "minecraft:entity.player.burp",
            feet,
            SoundSource::Players,
            0.5,
            burp_pitch,
            None,
        );
        if let Some(slice) = slice_of(self.block).and_then(|id| stack_of(&*host, id))
            && let Some(living) = host.living(player.entity())
        {
            let _ = living.apply_consume_effect(host, &slice, 0);
        }
        self.take_bite(host, level, pos, bites);
        level.play_sound(
            host,
            "minecraft:entity.generic.eat",
            [
                pos[0] as f64 + 0.5,
                pos[1] as f64 + 0.5,
                pos[2] as f64 + 0.5,
            ],
            SoundSource::Players,
            0.8,
            0.8,
            None,
        );
        self.particles(host, level, pos, state);
        InteractionResult::Success
    }

    /// `cutSlice`: the slice flies off toward the player, no food; the knife's use is counted.
    fn cut_slice(
        &self,
        host: &mut dyn ServerHost,
        click: &BlockClick<'_>,
        state: BlockState,
    ) -> InteractionResult {
        let Some(player) = host.players().acting_player() else {
            return InteractionResult::Pass;
        };
        let Some(level) = host.level(click.dimension.key().id()) else {
            return InteractionResult::Pass;
        };
        let pos = click.pos;
        let bites = bites_of(host.block_state_string(state).as_deref());
        self.take_bite(host, level, pos, bites);
        // player.getDirection().getOpposite(): toward the player.
        let toward: [f64; 2] = match player.entity().direction(&*host).unwrap_or("north") {
            "north" => [0.0, 1.0],
            "south" => [0.0, -1.0],
            "west" => [1.0, 0.0],
            _ => [-1.0, 0.0],
        };
        if let Some(slice) = slice_of(self.block).and_then(|id| stack_of(&*host, id)) {
            let _ = level.spawn_item_with_motion(
                host,
                pos[0] as f64 + 0.5,
                pos[1] as f64 + 0.3,
                pos[2] as f64 + 0.5,
                slice,
                [toward[0] * 0.15, 0.05, toward[1] * 0.15],
                0,
            );
        }
        level.play_sound(
            host,
            "minecraft:block.wool.break",
            [
                pos[0] as f64 + 0.5,
                pos[1] as f64 + 0.5,
                pos[2] as f64 + 0.5,
            ],
            SoundSource::Players,
            0.8,
            0.8,
            None,
        );
        self.particles(host, level, pos, state);
        if let Some(knife) = click.stack {
            let id = knife.item().id(&*host).to_string();
            player.award_stat(host, StatType::Used, &id, 1);
        }
        InteractionResult::Success
    }
}

// ---------------------------------------------------------------- vanilla pumpkin pie

/// `PlacePumpkinPieMixin`: after the pumpkin pie's own `useOn` (PASS), place
/// `farmersdelight:pumpkin_pie` as its hidden block item would (sneaking only when
/// `enablePumpkinPieSneakToPlace`, default off).
pub struct VanillaPumpkinPie;

/// `BlockPlaceContext`: the clicked block when it can be replaced, else the one on the clicked
/// face; None when that cell cannot take the pie (not replaceable, or no support below).
fn pie_target(
    host: &dyn Host,
    level: LevelRef,
    clicked: [i32; 3],
    face: [i32; 3],
) -> Option<[i32; 3]> {
    let replaceable = |pos: [i32; 3]| {
        level.block_state(host, pos).is_some_and(|s| {
            let name = host.block_state_string(s).unwrap_or_default();
            matches!(
                block_id(&name),
                "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
            ) || host.block_in_tag(s.block(), "minecraft:replaceable") == Some(true)
        })
    };
    let pos = if replaceable(clicked) {
        clicked
    } else {
        [
            clicked[0] + face[0],
            clicked[1] + face[1],
            clicked[2] + face[2],
        ]
    };
    (replaceable(pos) && floor_supports(host, level, pos)).then_some(pos)
}

fn sneak_ok(host: &dyn Host, player: PlayerRef) -> bool {
    !crate::config::pumpkin_pie_sneak_to_place()
        || player.entity().is_shift_key_down(host) == Some(true)
}

#[crate::engine::part]
impl ItemPart for VanillaPumpkinPie {
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        // Server only, as at Horizon; the client predicts vanilla.
        let HostSide::Server(host) = host else {
            return None;
        };
        crate::compat::claimed((|| -> Flow {
            let Some(player) = host.players().acting_player() else {
                return Flow::Continue;
            };
            if !sneak_ok(&*host, player) || player.may_build(&*host) == Some(false) {
                return Flow::Continue;
            }
            let Some(level) = player.level(&*host) else {
                return Flow::Continue;
            };
            let Some(pos) = pie_target(&*host, level, event.pos, event.face) else {
                return Flow::Continue;
            };
            let facing = player.entity().direction(&*host).unwrap_or("north");
            let Some(state) = host
                .block_state_id(PUMPKIN_PIE)
                .and_then(|s| host.block_state_string(s))
                .and_then(|n| host.block_state_id(&set_prop(&n, "facing", facing)))
            else {
                return Flow::Continue;
            };
            if level.set_block(
                host,
                pos,
                state,
                BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
            ) != Ok(true)
            {
                return Flow::Continue;
            }
            // BlockItem#place: the wool place sound at (1 + 1) / 2, pitch 0.8 × 1.0.
            level.play_sound(
                host,
                "minecraft:block.wool.place",
                [
                    pos[0] as f64 + 0.5,
                    pos[1] as f64 + 0.5,
                    pos[2] as f64 + 0.5,
                ],
                SoundSource::Blocks,
                1.0,
                0.8,
                None,
            );
            if player.is_creative(&*host) != Some(true)
                && let Some(living) = host.living(player.entity())
            {
                let _ = living.set_hand_item(host, event.hand, event.stack.clone().shrink(1));
            }
            Flow::Cancel
        })())
    }
}

// ---------------------------------------------------------------- client prediction

/// The client's answers for pies (E-28): the template (stonecutter) would claim every main-hand
/// click, and the vanilla pumpkin pie's placement (a server Part) would be predicted as a pass,
/// starting to eat it. Server-side nothing is answered here.
pub fn predict_use(host: HostSide<'_>, event: &mut UseItemOnBlockEvent<'_>) -> Flow {
    let HostSide::Client(host) = host else {
        return Flow::Continue;
    };
    let host: &dyn ClientHost = &*host;
    let Some(level) = host.level(host.dimension()) else {
        return Flow::Continue;
    };
    let click = event.click;
    match event.phase {
        UsePhase::Block => {
            let Some(state) = level.block_state(host, click.pos) else {
                return Flow::Continue;
            };
            let name = host.block_state_string(state).unwrap_or_default();
            if !PIES.contains(&block_id(&name)) {
                return Flow::Continue;
            }
            let can_eat = host
                .local_player()
                .and_then(|p| p.can_eat(host, false))
                .unwrap_or(false);
            *event.cancellation_result = predict(
                is_knife(host, click.stack),
                click.hand == Hand::Main,
                can_eat,
                click.stack.is_none(),
            );
            Flow::Cancel
        }
        UsePhase::ItemAfterBlock => {
            let Some(stack) = click.stack else {
                return Flow::Continue;
            };
            if stack.item().id(host) != VANILLA_PUMPKIN_PIE {
                return Flow::Continue;
            }
            let Some(player) = host.local_player() else {
                return Flow::Continue;
            };
            let face = click.hit.face;
            if !sneak_ok(host, player)
                || player.may_build(host) == Some(false)
                || pie_target(host, level, click.pos, face).is_none()
            {
                return Flow::Continue;
            }
            *event.cancellation_result = InteractionResult::Success;
            Flow::Cancel
        }
        _ => Flow::Continue,
    }
}

pub fn register(registrar: &mut dyn Registrar) {
    let block_particle = registrar.particle_key("minecraft:block");
    for block in PIES {
        let key = registrar.block_key(block);
        registrar.block_part(
            key,
            Pie {
                block,
                block_particle,
            },
        );
    }
    for item in PIE_ITEMS {
        let key = registrar.item_key(item);
        registrar.item_part(key, crate::blocks::feast::Placeable);
    }
    let vanilla = registrar.item_key(VANILLA_PUMPKIN_PIE);
    registrar.item_part(vanilla, VanillaPumpkinPie);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_bites_then_gone_and_comparator_counts_down() {
        assert_eq!(after_bite(0), Some(1));
        assert_eq!(after_bite(2), Some(3));
        assert_eq!(after_bite(3), None);
        assert_eq!((0..4).map(comparator).collect::<Vec<_>>(), [4, 3, 2, 1]);
    }

    #[test]
    fn faces_the_way_the_placer_looks() {
        // Minecraft yaw 0 looks south, 90 west.
        assert_eq!(facing_looking(0.0), "south");
        assert_eq!(facing_looking(90.0), "west");
        assert_eq!(facing_looking(180.0), "north");
        assert_eq!(facing_looking(-90.0), "east");
    }

    #[test]
    fn client_prediction_matches_pie_block() {
        use InteractionResult::*;
        assert_eq!(predict(true, false, false, false), Success); // off-hand knife cuts
        assert_eq!(predict(false, false, true, false), Pass); // off hand never eats
        assert_eq!(predict(false, true, true, false), Success);
        assert_eq!(predict(false, true, false, true), Consume);
        assert_eq!(predict(false, true, false, false), Pass);
    }
}
