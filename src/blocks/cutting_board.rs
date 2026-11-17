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

#[crate::engine::part]
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
            return left_behind(&*level, event.state);
        }
        None
    }

    /// Water stays where a waterlogged board is removed (`Level.removeBlock` leaves the fluid's
    /// block). The engine at the pin reads the fluid from the template (stone cutter: none), not
    /// from the declared `waterlogged` property, so the board says it itself (E-13).
    fn remove_block(
        &self,
        level: &dyn LevelHost,
        event: &mut RemoveBlockEvent,
    ) -> Option<BlockState> {
        let name = level.state_string(event.state)?;
        (state_value(&name, "waterlogged") == Some("true"))
            .then(|| level.block_state_id("minecraft:water"))
            .flatten()
    }

    fn has_analog_output(&self, _: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }

    fn analog_output(&self, _: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = signal([event.pos.x, event.pos.y, event.pos.z]);
    }

    /// Automation (hoppers, any side) changed the slot: keep the client mirror and the
    /// comparator level in step.
    fn contents_changed(&self, host: &mut dyn ServerHost, event: &mut ContentsChangedEvent) {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        let Some(be) = host.block_entity(event.level, pos) else {
            return;
        };
        let stored = be.item_slot(&*host, 0).ok().flatten();
        mirror_inventory(host, be, stored.as_ref());
        host.block_entity_sync(pos);
    }

    /// `CuttingBoardDispenserMixin` + `CuttingBoardDispenseBehavior`: with the config on, a
    /// dispenser facing a board uses whatever it dispenses as the tool and keeps it (damaged);
    /// click sound on a cut, fail sound otherwise, never vanilla's dispense.
    fn dispense_method(
        &self,
        host: &mut dyn ServerHost,
        event: &mut DispenseMethodEvent<'_>,
    ) -> Option<Dispensed> {
        if !crate::config::dispenser_tools_cutting_board() {
            return None;
        }
        let level = host.level(event.dimension)?;
        let s = match event.facing {
            Direction::North => [0, 0, -1],
            Direction::South => [0, 0, 1],
            Direction::West => [-1, 0, 0],
            Direction::East => [1, 0, 0],
            Direction::Up => [0, 1, 0],
            Direction::Down => [0, -1, 0],
        };
        let pos = [
            event.pos[0] + s[0],
            event.pos[1] + s[1],
            event.pos[2] + s[2],
        ];
        let Some(be) = host.block_entity(level, pos) else {
            return Some((Some(event.stack.clone()), 1001));
        };
        let stored = be.item_slot(&*host, 0).ok().flatten();
        let carved = be.data(&*host).is_some_and(|d| is_carved(&d));
        if stored.is_none() {
            return Some((Some(event.stack.clone()), 1001));
        }
        match self.cut(host, None, level, be, pos, event.stack, stored, carved) {
            Some(after) => Some((after, 1000)),
            None => Some((Some(event.stack.clone()), 1001)),
        }
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

/// What a board leaves when it goes: water if it was waterlogged, else air.
fn left_behind(level: &dyn LevelHost, state: BlockState) -> Option<BlockState> {
    let name = level.state_string(state).unwrap_or_default();
    let block = if state_value(&name, "waterlogged") == Some("true") {
        "minecraft:water"
    } else {
        "minecraft:air"
    };
    level.block_state_id(block)
}

/// `Block.canSupportRigidBlock(below) || Block.canSupportCenter(below, UP)`, approximated: the
/// level-scoped host these hooks get has no face-sturdiness or shape query at the build pin, so
/// any block below that is not air and not `#minecraft:replaceable` supports the board
/// (accepted delta A9, PROGRESS M1-02).
pub(crate) fn supported(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
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
                level.play_sound(
                    host,
                    SOUND_REMOVE,
                    center,
                    SoundSource::Blocks,
                    0.25,
                    0.5,
                    None,
                );
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
                level.play_sound(
                    host,
                    SOUND_PLACE,
                    center,
                    SoundSource::Blocks,
                    1.0,
                    0.8,
                    None,
                );
                host.block_entity_sync(pos);
                Some(InteractionResult::SuccessServer)
            }
            Action::UseTool => {
                let tool = held?;
                if self
                    .cut(host, Some(player), level, be, pos, &tool, stored, carved)
                    .is_some()
                {
                    Some(InteractionResult::Success)
                } else {
                    Some(InteractionResult::Consume)
                }
            }
        }
    }

    /// `CuttingBoardBlockEntity#processStoredItemUsingTool`. `player` is None for a dispenser,
    /// which then gets the tool back damaged (`Some(tool)`, None when it broke); with a player
    /// the hand's stack is damaged in place. None when nothing was cut.
    #[allow(clippy::too_many_arguments)]
    fn cut(
        &self,
        host: &mut dyn ServerHost,
        player: Option<PlayerRef>,
        level: LevelRef,
        be: BlockEntityRef,
        pos: [i32; 3],
        tool: &ItemStack,
        stored: Option<ItemStack>,
        carved: bool,
    ) -> Option<Option<ItemStack>> {
        if carved {
            return None;
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
            if let Some(player) = player {
                let _ = player.action_bar(
                    host,
                    Text::translate("block.farmersdelight.cutting_board.invalid_item", vec![]),
                );
            }
            return None;
        };
        let facing = level
            .block_property(&*host, pos, "facing")
            .unwrap_or_else(|| "north".into());
        let [dx, dz] = counter_clockwise_step(&facing);
        let fortune = enchantment_level(tool, "minecraft:fortune");
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
        let tool_after = match player {
            Some(player) => {
                if let Some(living) = host.living(player.entity()) {
                    let _ = living.hurt_and_break(host, EquipmentSlot::MainHand, 1);
                }
                let tool_id = tool.item().id(&*host);
                player.award_stat(host, StatType::Used, &tool_id, 1);
                None
            }
            None => damaged(&*host, tool),
        };
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
        let (sound, volume, pitch) =
            processing_sound(&*host, recipe.sound.as_deref(), tool, &input);
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
            None,
        );
        let remaining = input.shrink(1);
        let left = remaining.as_ref().map_or(0, ItemStack::count);
        let _ = host.transaction(|host, tx| {
            be.set_item_slot(host, tx, 0, remaining.clone())
                .map(|_| ())
                .map_err(|_| ())
        });
        mirror_inventory(host, be, remaining.as_ref());
        if let Some(player) = player {
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
        }
        host.block_entity_sync(pos);
        Some(tool_after)
    }
}

/// `playProcessingSound`: the recipe's sound, else by tool, else by the board's item.
fn processing_sound(
    host: &dyn Host,
    recipe_sound: Option<&str>,
    tool: &ItemStack,
    board_item: &ItemStack,
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
    // `boardItem.getItem() instanceof BlockItem`: its block's default state's break sound.
    if let Some(sound) = host
        .item_block(board_item.item())
        .and_then(|b| host.block_state_default(b))
        .and_then(|state| host.block_states().sound(state))
    {
        return (sound.break_sound.to_string(), 1.0, 0.8);
    }
    ("minecraft:block.wood.break".into(), 1.0, 0.8)
}

/// `EnchantmentHelper.getItemEnchantmentLevel`: the level in the stack's `minecraft:enchantments`.
pub fn enchantment_level(stack: &ItemStack, id: &str) -> u32 {
    match stack
        .components()
        .and_then(|p| p.as_nbt().get(&"minecraft:enchantments".into()).cloned())
    {
        Some(Tag::Compound(levels)) => match levels.get(&id.into()) {
            Some(Tag::Int(l)) => (*l).max(0) as u32,
            _ => 0,
        },
        _ => 0,
    }
}

/// `hurtAndBreak(1, level, null, …)` for a stack held by no one (a dispenser's): one more damage
/// on a damageable stack, None once it breaks. Unbreaking is not applied (A40).
fn damaged(host: &dyn Host, tool: &ItemStack) -> Option<ItemStack> {
    let max = match tool
        .components()
        .and_then(|p| p.as_nbt().get(&"minecraft:max_damage".into()).cloned())
        .or_else(|| host.item_default_component(tool.item(), "minecraft:max_damage"))
    {
        Some(Tag::Int(m)) => m.max(0) as u32,
        Some(Tag::Long(m)) => m.max(0) as u32,
        _ => return Some(tool.clone()),
    };
    let unbreakable = tool
        .components()
        .is_some_and(|p| p.as_nbt().get(&"minecraft:unbreakable".into()).is_some())
        || host
            .item_default_component(tool.item(), "minecraft:unbreakable")
            .is_some();
    if max == 0 || unbreakable {
        return Some(tool.clone());
    }
    let next = tool.damage() + 1;
    if next >= max {
        return tool.clone().shrink(1);
    }
    let mut out = tool.clone();
    out.set_damage(next);
    Some(out)
}

// ---------------------------------------------------------------- sneak carving, dispenser, comparator

/// `ToolCarvingEvent#onSneakPlaceTool`: what the main hand may be carved into a board.
fn carvable(host: &dyn Host, stack: &ItemStack) -> bool {
    let has_tool = stack
        .components()
        .and_then(|p| p.as_nbt().get(&"minecraft:tool".into()).cloned())
        .is_some()
        || host
            .item_default_component(stack.item(), "minecraft:tool")
            .is_some();
    let id = stack.item().id(host);
    has_tool || id == "minecraft:trident" || id == "minecraft:shears"
}

/// `UseBlockCallback` `ToolCarvingEvent#onSneakPlaceTool`: sneaking with a tool in the main hand
/// on an empty board carves it in (the whole stack, one used up outside creative). Decided the
/// same way on both sides; the server changes the board.
pub fn on_right_click(mut host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
    let click = event.click;
    // FD reads the main hand on either hand's event; the off hand's is reached only after the
    // main hand's passed under the same test, so only the main hand's event can carve.
    if click.hand != Hand::Main {
        return Flow::Continue;
    }
    match host.reborrow() {
        HostSide::Client(h) => {
            let h: &dyn ClientHost = &*h;
            let Some(player) = h.local_player() else {
                return Flow::Continue;
            };
            if player.is_spectator(h) == Some(true)
                || player.entity().is_shift_key_down(h) != Some(true)
            {
                return Flow::Continue;
            }
            let Some(held) = click.stack else {
                return Flow::Continue;
            };
            if !carvable(h, held) {
                return Flow::Continue;
            }
            let Some(level) = h.level(h.dimension()) else {
                return Flow::Continue;
            };
            let board = level
                .block_state(h, click.pos)
                .and_then(|s| h.block_state_string(s))
                .is_some_and(|n| n.split('[').next() == Some(ID));
            if !board {
                return Flow::Continue;
            }
            let empty = h
                .block_entity(level, click.pos)
                .and_then(|e| e.data(h))
                .is_none_or(
                    |d| !matches!(d.get(&INVENTORY.into()), Some(Tag::List(l)) if !l.is_empty()),
                );
            if !empty {
                return Flow::Continue;
            }
        }
        HostSide::Server(h) => {
            if !carve(h, click) {
                return Flow::Continue;
            }
        }
    }
    *event.cancellation_result = InteractionResult::Success;
    Flow::Cancel
}

fn carve(host: &mut dyn ServerHost, click: &BlockClick<'_>) -> bool {
    let Some(player) = host.players().acting_player() else {
        return false;
    };
    if player.is_spectator(&*host) == Some(true)
        || player.entity().is_shift_key_down(&*host) != Some(true)
    {
        return false;
    }
    let Some(living) = host.living(player.entity()) else {
        return false;
    };
    let Some(held) = living.hand_item(&*host, Hand::Main) else {
        return false;
    };
    if !carvable(&*host, &held) {
        return false;
    }
    let Some(level) = host.level(click.dimension.key().id()) else {
        return false;
    };
    let Some(be) = host.block_entity(level, click.pos) else {
        return false;
    };
    let is_board = level
        .block_state(&*host, click.pos)
        .and_then(|s| host.block_state_string(s))
        .is_some_and(|n| n.split('[').next() == Some(ID));
    if !is_board || be.item_slot(&*host, 0).ok().flatten().is_some() {
        return false;
    }
    let creative = player.is_creative(&*host).unwrap_or(false);
    // addItem(stack) inserts up to the slot limit; FD counts it carved only when all went in.
    let limit = SLOT_LIMIT.min(host.max_stack_size(held.item()).unwrap_or(64));
    if held.count() > limit {
        return false;
    }
    if host
        .transaction(|host, tx| {
            be.set_item_slot(host, tx, 0, Some(held.clone()))
                .map(|_| ())
                .map_err(|_| ())
        })
        .is_err()
    {
        return false;
    }
    let mut data = BTreeMap::new();
    data.insert(CARVED.into(), Tag::Byte(1));
    let _ = be.merge_data(host, data);
    mirror_inventory(host, be, Some(&held));
    if !creative {
        let _ = living.set_hand_item(host, Hand::Main, held.clone().shrink(1));
    }
    let pos = click.pos;
    let centre = [
        pos[0] as f64 + 0.5,
        pos[1] as f64 + 0.5,
        pos[2] as f64 + 0.5,
    ];
    level.play_sound(
        host,
        "minecraft:block.wood.place",
        centre,
        SoundSource::Blocks,
        1.0,
        0.8,
        None,
    );
    host.block_entity_sync(pos);
    true
}

/// `getAnalogOutputSignal`: `floor(count / min(64, max stack) * 14) + 1`, 0 when empty.
pub fn comparator(count: u32, max_stack: u32) -> i32 {
    if count == 0 {
        return 0;
    }
    let proportion = count as f32 / SLOT_LIMIT.min(max_stack).max(1) as f32;
    (proportion * 14.0).floor() as i32 + 1
}

static SIGNALS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<[i32; 3], i32>>> =
    std::sync::LazyLock::new(Default::default);

fn set_signal(pos: [i32; 3], signal: i32) {
    let mut map = SIGNALS.lock().unwrap_or_else(|e| e.into_inner());
    if signal == 0 {
        map.remove(&pos);
    } else {
        map.insert(pos, signal);
    }
}

fn signal(pos: [i32; 3]) -> i32 {
    SIGNALS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&pos)
        .copied()
        .unwrap_or(0)
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
    if let Some(pos) = be.position(&*host) {
        let level = stack.map_or(0, |s| {
            comparator(s.count(), host.max_stack_size(s.item()).unwrap_or(64))
        });
        set_signal(pos, level);
    }
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

#[crate::engine::part]
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
            let trident = stack.item().id(host) == "minecraft:trident";
            Pose::Carved {
                angle: if tool {
                    225.0
                } else if trident {
                    135.0
                } else {
                    180.0
                },
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
    fn comparator_levels() {
        assert_eq!(comparator(0, 64), 0);
        assert_eq!(comparator(1, 64), 1);
        assert_eq!(comparator(32, 64), 8);
        assert_eq!(comparator(64, 64), 15);
        assert_eq!(comparator(1, 1), 15);
        assert_eq!(comparator(8, 16), 8);
    }

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
