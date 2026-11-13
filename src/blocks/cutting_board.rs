//! Cutting board (J/common/block/CuttingBoardBlock.java, J/common/block/entity/CuttingBoardBlockEntity.java).
//!
//! Block: `facing` (opposite the placer), `waterlogged`, survives on a rigid or center-supported
//! floor and pops off otherwise (`updateShape` on DOWN). Block entity: one slot (`Items`, engine
//! owned) and `IsItemCarved` in its NBT. Interaction uses the **main hand only** (Q8).
use crate::client::render::{Pose, draw_board, model_count};
use crate::recipes::{RecipeTables, cutting};
use mod_api::*;
use std::collections::BTreeMap;
use std::sync::Arc;

pub const ID: &str = "farmersdelight:cutting_board";
pub const CARVED: &str = "IsItemCarved";
pub const INVENTORY: &str = "Inventory";
const FLAT_TAG: &str = "farmersdelight:flat_on_cutting_board";
/// `ItemStackHandler`'s default slot limit.
pub const SLOT_LIMIT: u32 = 64;

const SOUND_PLACE: &str = "farmersdelight:block.cutting_board.place_item";
const SOUND_REMOVE: &str = "farmersdelight:block.cutting_board.remove_item";
const SOUND_KNIFE: &str = "farmersdelight:block.cutting_board.knife_cut";
const KNIFE_TAG: &str = "c:tools/knife";
const SHEAR_TAG: &str = "c:tools/shear";
const USE_CUTTING_BOARD: &str = "farmersdelight:use_cutting_board";

// ---------------------------------------------------------------- pure rules

/// What a main-hand click does (`CuttingBoardBlock#useItemOn`).
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// Empty hand, empty board: nothing happens (CONSUME).
    Nothing,
    /// Empty hand: take the whole stored stack.
    Take,
    /// Put this many of the held stack on the board.
    Insert(u32),
    /// The held item cannot go on the board: try it as a cutting tool.
    UseTool,
}

/// `held`: count of the main-hand stack and its max stack size (None = empty hand).
/// `stored`: count on the board and whether the held stack could merge with it.
pub fn decide(held: Option<(u32, u32)>, stored: Option<(u32, bool)>, carved: bool) -> Action {
    let Some((count, max)) = held else {
        return if stored.is_some() {
            Action::Take
        } else {
            Action::Nothing
        };
    };
    match insert_count(count, max, stored, carved) {
        0 => Action::UseTool,
        n => Action::Insert(n),
    }
}

/// `ItemStackHandler.insertItem(0, stack, simulate)`: how many of the held stack fit.
pub fn insert_count(count: u32, max_stack: u32, stored: Option<(u32, bool)>, carved: bool) -> u32 {
    if carved || count == 0 {
        return 0;
    }
    let limit = SLOT_LIMIT.min(max_stack);
    match stored {
        None => count.min(limit),
        Some((_, false)) => 0,
        Some((have, true)) => count.min(limit.saturating_sub(have)),
    }
}

/// Horizontal facing for a placer looking `yaw` degrees (Minecraft yaw: 0 = south, 90 = west):
/// `context.getHorizontalDirection().getOpposite()`.
pub fn facing_for_yaw(yaw: f32) -> &'static str {
    let quarter = ((yaw as f64 / 90.0 + 0.5).floor() as i64).rem_euclid(4);
    // Direction.from2DDataValue: 0 south, 1 west, 2 north, 3 east; then the opposite.
    match quarter {
        0 => "north",
        1 => "east",
        2 => "south",
        _ => "west",
    }
}

/// `Direction.getCounterClockWise()` as a step: north→west, west→south, south→east, east→north.
pub fn counter_clockwise_step(facing: &str) -> [f64; 2] {
    match facing {
        "north" => [-1.0, 0.0],
        "west" => [0.0, 1.0],
        "south" => [1.0, 0.0],
        _ => [0.0, -1.0],
    }
}

pub fn is_carved(nbt: &Compound) -> bool {
    matches!(nbt.get(&CARVED.into()), Some(Tag::Byte(b)) if *b != 0)
}

// ---------------------------------------------------------------- the Part

pub struct CuttingBoard {
    pub recipes: Arc<RecipeTables>,
    pub item_particle: ParticleKey,
}

/// The value of `key` in a state string such as `ns:block[facing=north,waterlogged=true]`.
pub fn state_value<'a>(state: &'a str, key: &str) -> Option<&'a str> {
    let props = state.split_once('[')?.1.strip_suffix(']')?;
    props.split(',').find_map(|kv| {
        kv.split_once('=')
            .filter(|(k, _)| *k == key)
            .map(|(_, v)| v)
    })
}

/// The `farmersdelight:use_cutting_board` trigger has no conditions.
struct Always;
impl CriterionTest for Always {
    fn test(&self, _: &dyn CriterionContext) -> Result<bool, CriterionTestError> {
        Ok(true)
    }
}

#[mod_api::horizon_part]
impl BlockPart for CuttingBoard {
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let facing = facing_for_yaw(event.context.yaw);
        // `fluid.getType() == Fluids.WATER`: water's source fluid, read from the water block.
        let water = level
            .block_state_id("minecraft:water")
            .and_then(|w| level.state_fluid_state(w))
            .is_some_and(|w| w.fluid == event.fluid.fluid);
        let state = level.state_with_property(event.state, "facing", facing)?;
        let state = level.state_with_property(
            state,
            "waterlogged",
            if water { "true" } else { "false" },
        )?;
        Some(Some(state))
    }

    fn can_survive(&self, level: &dyn LevelHost, event: &mut CanSurviveEvent) -> Option<bool> {
        Some(supported(level, [event.pos.x, event.pos.y, event.pos.z]))
    }

    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        let name = level.state_string(event.state).unwrap_or_default();
        if state_value(&name, "waterlogged") == Some("true") {
            level.schedule_fluid_tick(pos, "minecraft:water", 5);
        }
        if event.direction == Direction::Down && !supported(&*level, pos) {
            return level.block_state_id("minecraft:air");
        }
        None
    }

    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        let click = event.click;
        if click.hand != Hand::Main {
            return Some(InteractionResult::Pass);
        }
        Some(
            self.interact(host, click)
                .unwrap_or(InteractionResult::Consume),
        )
    }
}

/// `Block.canSupportRigidBlock(below) || Block.canSupportCenter(below, UP)`, approximated: the
/// level-scoped host these hooks get has no face-sturdiness or shape query at the build pin, so
/// any block below that is not air and not `#minecraft:replaceable` supports the board
/// (accepted delta A9, PROGRESS M1-02).
fn supported(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    let floor = [pos[0], pos[1] - 1, pos[2]];
    let Some(state) = level.block_state(floor) else {
        return true;
    };
    let Some(name) = level.state_block(state) else {
        return true;
    };
    if matches!(
        name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    ) {
        return false;
    }
    level.block_in_tag(state.block(), "minecraft:replaceable") != Some(true)
}

impl CuttingBoard {
    fn interact(
        &self,
        host: &mut dyn ServerHost,
        click: &BlockClick<'_>,
    ) -> Option<InteractionResult> {
        let player = host.players().acting_player()?;
        let level = host.level(click.dimension.key().id())?;
        let pos = click.pos;
        let be = host.block_entity(level, pos)?;
        let stored = be.item_slot(host, 0).ok()?;
        let carved = be.data(host).is_some_and(|d| is_carved(&d));
        let held = click.stack.cloned();
        let creative = player.is_creative(host).unwrap_or(false);
        let center = [
            pos[0] as f64 + 0.5,
            pos[1] as f64 + 0.5,
            pos[2] as f64 + 0.5,
        ];

        let held_info = held
            .as_ref()
            .map(|s| (s.count(), host.max_stack_size(s.item()).unwrap_or(64)));
        let stored_info = stored.as_ref().map(|s| {
            (
                s.count(),
                held.as_ref().is_some_and(|h| h.is_stackable_with(s)),
            )
        });
        match decide(held_info, stored_info, carved) {
            Action::Nothing => Some(InteractionResult::Consume),
            Action::Take => {
                let taken = stored?;
                host.transaction(|host, tx| {
                    be.set_item_slot(host, tx, 0, None).map_err(|_| ())?;
                    if !creative {
                        crate::inventory::give_or_drop(host, tx, player, taken.clone());
                    }
                    Ok::<_, ()>(())
                })
                .ok()?;
                clear_carved(host, be, carved);
                mirror_inventory(host, be, None);
                level.play_sound(host, SOUND_REMOVE, center, SoundSource::Blocks, 0.25, 0.5);
                host.block_entity_sync(pos);
                Some(InteractionResult::Success)
            }
            Action::Insert(n) => {
                let held = held?;
                let mut next = match stored {
                    Some(s) => s,
                    None => held.clone().split(n).taken?,
                };
                if next.count() != n || stored_info.is_some() {
                    // merging onto an existing stack
                    if stored_info.is_some() {
                        next.grow(n).ok()?;
                    }
                }
                host.transaction(|host, tx| {
                    be.set_item_slot(host, tx, 0, Some(next.clone()))
                        .map_err(|_| ())?;
                    Ok::<_, ()>(())
                })
                .ok()?;
                mirror_inventory(host, be, Some(&next));
                if !creative {
                    let living = host.living(player.entity())?;
                    let _ = living.set_hand_item(host, Hand::Main, held.shrink(n));
                }
                level.play_sound(host, SOUND_PLACE, center, SoundSource::Blocks, 1.0, 0.8);
                host.block_entity_sync(pos);
                Some(InteractionResult::SuccessServer)
            }
            Action::UseTool => {
                let tool = held?;
                if self.cut(host, player, level, be, pos, &tool, stored, carved) {
                    Some(InteractionResult::Success)
                } else {
                    Some(InteractionResult::Consume)
                }
            }
        }
    }

    /// `CuttingBoardBlockEntity#processStoredItemUsingTool`.
    #[allow(clippy::too_many_arguments)]
    fn cut(
        &self,
        host: &mut dyn ServerHost,
        player: PlayerRef,
        level: LevelRef,
        be: BlockEntityRef,
        pos: [i32; 3],
        tool: &ItemStack,
        stored: Option<ItemStack>,
        carved: bool,
    ) -> bool {
        if carved {
            return false;
        }
        let table = self.recipes.cutting.snapshot();
        let found = stored.as_ref().and_then(|input| {
            match cutting::find(&table, |i| i.test(&*host, input), |i| i.test(&*host, tool)) {
                cutting::Lookup::Found(r) => Some(r.clone()),
                // Reference quirk (Q16): `matches` tests input and tool together, so a
                // tool mismatch also reports "invalid item".
                _ => None,
            }
        });
        let (Some(recipe), Some(input)) = (found, stored) else {
            let _ = player.action_bar(
                host,
                Text::translate("block.farmersdelight.cutting_board.invalid_item", vec![]),
            );
            return false;
        };
        let facing = level
            .block_property(&*host, pos, "facing")
            .unwrap_or_else(|| "north".into());
        let [dx, dz] = counter_clockwise_step(&facing);
        let fortune = 0; // Fortune on cutting tools: M8-03 (no enchantment read yet).
        let bonus = crate::config::cutting_board_fortune_bonus();
        let mut random = level.random(&*host);
        for result in &recipe.results {
            let kept =
                cutting::roll_count(result.stack.count(), result.chance, bonus, fortune, || {
                    random.as_mut().map_or(0.0, |r| r.next_float())
                });
            if kept == 0 {
                continue;
            }
            let mut stack = result.stack.clone();
            if stack.set_count(kept).is_err() {
                continue;
            }
            let _ = level.spawn_item_with_motion(
                host,
                pos[0] as f64 + 0.5 + dx * 0.2,
                pos[1] as f64 + 0.2,
                pos[2] as f64 + 0.5 + dz * 0.2,
                stack,
                [dx * 0.2, 0.0, dz * 0.2],
                0,
            );
        }
        if let Some(r) = random {
            level.set_random(host, r);
        }
        if let Some(living) = host.living(player.entity()) {
            let _ = living.hurt_and_break(host, EquipmentSlot::MainHand, 1);
        }
        let tool_id = tool.item().id(&*host);
        player.award_stat(host, StatType::Used, &tool_id, 1);
        let particles = ParticleOptions::item_stack(self.item_particle, input.clone());
        level.send_particles(
            host,
            &particles,
            None,
            [
                pos[0] as f64 + 0.5,
                pos[1] as f64 + 0.2,
                pos[2] as f64 + 0.5,
            ],
            [0.1, 0.1, 0.1],
            0.05,
            5,
        );
        let (sound, volume, pitch) = processing_sound(&*host, recipe.sound.as_deref(), tool);
        level.play_sound(
            host,
            &sound,
            [
                pos[0] as f64 + 0.5,
                pos[1] as f64 + 0.5,
                pos[2] as f64 + 0.5,
            ],
            SoundSource::Blocks,
            volume,
            pitch,
        );
        let remaining = input.shrink(1);
        let left = remaining.as_ref().map_or(0, ItemStack::count);
        let _ = host.transaction(|host, tx| {
            be.set_item_slot(host, tx, 0, remaining.clone())
                .map(|_| ())
                .map_err(|_| ())
        });
        mirror_inventory(host, be, remaining.as_ref());
        if let Ok(trigger) = Identifier::parse(USE_CUTTING_BOARD) {
            let _ = player.trigger_criterion(host, &trigger, Arc::new(Always));
        }
        let line = if left > 0 {
            Text::translate(
                "block.farmersdelight.cutting_board.remaining_items",
                vec![Text::literal(left.to_string())],
            )
        } else {
            Text::literal("")
        };
        let _ = player.action_bar(host, line);
        host.block_entity_sync(pos);
        true
    }
}

/// `playProcessingSound`: the recipe's sound, else by tool, else by the board's item.
fn processing_sound(
    host: &dyn Host,
    recipe_sound: Option<&str>,
    tool: &ItemStack,
) -> (String, f32, f32) {
    if let Some(s) = recipe_sound {
        return (s.to_owned(), 1.0, 1.0);
    }
    if in_item_tag(host, tool, SHEAR_TAG) {
        return ("minecraft:entity.sheep.shear".into(), 1.0, 1.0);
    }
    if in_item_tag(host, tool, KNIFE_TAG) {
        return (SOUND_KNIFE.into(), 0.8, 1.0);
    }
    // BlockItem → its block's break sound at pitch 0.8: M8-03 (no tool on M1's path reaches it).
    ("minecraft:block.wood.break".into(), 1.0, 0.8)
}

fn in_item_tag(host: &dyn Host, stack: &ItemStack, tag: &str) -> bool {
    let tags = host.tags();
    Identifier::parse(tag)
        .ok()
        .and_then(|id| tags.key(&id))
        .and_then(|key| tags.is_in_tag(stack.item(), key))
        == Some(true)
}

/// Keep the reference's `Inventory` list (one stack or none) in the block entity's NBT, so the
/// update tag can send what the board holds. `Items` stays the engine-owned saved copy.
fn mirror_inventory(host: &mut dyn ServerHost, be: BlockEntityRef, stack: Option<&ItemStack>) {
    let list = stack
        .and_then(|s| s.to_nbt(&*host))
        .map(|c| vec![Tag::Compound(c)])
        .unwrap_or_default();
    let mut data = BTreeMap::new();
    data.insert(INVENTORY.into(), Tag::List(list));
    let _ = be.merge_data(host, data);
}

fn clear_carved(host: &mut dyn ServerHost, be: BlockEntityRef, carved: bool) {
    if carved {
        let mut data = BTreeMap::new();
        data.insert(CARVED.into(), Tag::Byte(0));
        let _ = be.merge_data(host, data);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for CuttingBoard {
    fn block_entity_update_tag(
        &self,
        event: &mut BlockEntityUpdateTagEvent<'_>,
    ) -> Option<Compound> {
        // The client copy never receives `items` (seen in game, U-6): the update tag carries the
        // reference's `Inventory` list, mirrored into the NBT on each server-side change.
        let mut tag = BTreeMap::new();
        tag.insert(CARVED.into(), Tag::Byte(is_carved(&event.entity.nbt) as i8));
        if let Some(inv) = event.entity.nbt.get(&INVENTORY.into()) {
            tag.insert(INVENTORY.into(), inv.clone());
        }
        Some(tag)
    }

    fn block_entity_submit(&self, host: &dyn ClientHost, event: &mut BlockEntitySubmitEvent<'_>) {
        let Some(stack) = stored_on_client(host, event.entity) else {
            return;
        };
        let name = host.block_state_string(event.state).unwrap_or_default();
        let facing = state_value(&name, "facing").unwrap_or("north");
        let pose = if is_carved(&event.entity.nbt) {
            let tool = in_item_tag(host, &stack, "minecraft:pickaxes")
                || in_item_tag(host, &stack, "minecraft:hoes");
            // TridentItem → 135: M8-03 (tool carving)
            Pose::Carved {
                angle: if tool { 225.0 } else { 180.0 },
            }
        } else if host.item_block(stack.item()).is_some() && !in_item_tag(host, &stack, FLAT_TAG) {
            Pose::Block
        } else {
            Pose::Flat
        };
        let max = host.max_stack_size(stack.item()).unwrap_or(64);
        let seed = stack.item().id(host).len() as i32; // stand-in for Item.getId + damage
        draw_board(
            event.draw,
            &stack,
            facing,
            pose,
            model_count(stack.count(), max),
            seed,
        );
    }
}

/// The stored stack as the client sees it: the mirrored item list if the engine sends it,
/// else the `Inventory` list of the update tag.
fn stored_on_client(host: &dyn Host, entity: &ModBlockEntity) -> Option<ItemStack> {
    if let Some(Some(stack)) = entity.items.first() {
        return Some(stack.clone());
    }
    match entity.nbt.get(&INVENTORY.into()) {
        Some(Tag::List(list)) => list.iter().find_map(|t| match t {
            Tag::Compound(c) => ItemStack::from_nbt(host, c),
            _ => None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_hand() {
        assert_eq!(decide(None, None, false), Action::Nothing);
        assert_eq!(decide(None, Some((3, false)), false), Action::Take);
        assert_eq!(decide(None, Some((3, false)), true), Action::Take);
    }

    #[test]
    fn insert_whole_stack_up_to_limit() {
        assert_eq!(decide(Some((3, 64)), None, false), Action::Insert(3));
        assert_eq!(decide(Some((1, 1)), None, false), Action::Insert(1)); // a knife goes on an empty board
        assert_eq!(
            decide(Some((10, 64)), Some((60, true)), false),
            Action::Insert(4)
        );
        assert_eq!(
            decide(Some((10, 16)), Some((16, true)), false),
            Action::UseTool
        );
    }

    #[test]
    fn different_item_or_carved_is_a_tool_use() {
        assert_eq!(
            decide(Some((1, 1)), Some((3, false)), false),
            Action::UseTool
        );
        assert_eq!(decide(Some((3, 64)), None, true), Action::UseTool);
    }

    #[test]
    fn placement_faces_the_placer() {
        // looking south (yaw 0) → the board faces north, and so on
        assert_eq!(facing_for_yaw(0.0), "north");
        assert_eq!(facing_for_yaw(90.0), "east");
        assert_eq!(facing_for_yaw(180.0), "south");
        assert_eq!(facing_for_yaw(-90.0), "west");
        assert_eq!(facing_for_yaw(44.0), "north");
        assert_eq!(facing_for_yaw(46.0), "east");
    }

    #[test]
    fn results_drop_counter_clockwise() {
        assert_eq!(counter_clockwise_step("north"), [-1.0, 0.0]); // west
        assert_eq!(counter_clockwise_step("east"), [0.0, -1.0]); // north
    }
}
