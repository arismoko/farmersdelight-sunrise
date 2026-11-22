//! Cooking pot (J/common/block/CookingPotBlock.java, J/common/block/entity/CookingPotBlockEntity.java,
//! …/container/CookingPotMenu.java, CookingPotMealSlot.java, CookingPotResultSlot.java,
//! …/inventory/CookingPotItemHandler.java, J/client/gui/CookingPotScreen.java).
//!
//! Nine slots: 0–5 ingredients, 6 meal display (up to `max(64, max stack)`), 7 container input,
//! 8 output. Heated (conductors count), a matching `farmersdelight:cooking` recipe whose result
//! fits the meal slot advances by one per tick; at the recipe's time the result goes to the meal
//! slot, every input slot gives up one item and ejects its remainder counter-clockwise of
//! `facing`. Otherwise progress decays by 2. A meal with no container moves to the output; one
//! that needs a container is served from the container slot, or into a held container.
//! NBT: `CookTime`, `CookTimeTotal`, `Container`, `RecipesUsed` (the slots are the engine's
//! `Items`); update tag `Inventory` (slot list) + `Container` for the client.
use super::cutting_board::{counter_clockwise_step, facing_for_yaw, state_value};
use crate::recipes::RecipeTables;
use crate::recipes::cooking::{self, CookingRecipe};
use mod_api::*;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

pub const ID: &str = "farmersdelight:cooking_pot";
pub const MENU: &str = ID;
pub const SLOTS: usize = 9;
pub const INPUTS: usize = 6;
pub const MEAL: usize = 6;
pub const CONTAINER: usize = 7;
pub const OUTPUT: usize = 8;
pub const COOK_TIME: &str = "CookTime";
pub const COOK_TIME_TOTAL: &str = "CookTimeTotal";
pub const CONTAINER_KEY: &str = "Container";
pub const RECIPES_USED: &str = "RecipesUsed";
pub const INVENTORY: &str = "Inventory";
/// The meal display, kept in the saved data rather than container slot 6: the game drops a mod
/// container's slots when the block goes, and the meal must ride on the pot item instead.
pub const MEAL_KEY: &str = "Meal";
/// Item components of the pot's item form (`ModDataComponents.MEAL`, `CONTAINER`).
pub const MEAL_COMPONENT: &str = "farmersdelight:meal";
pub const CONTAINER_COMPONENT: &str = "farmersdelight:container";
pub const COMPONENT_TYPES: [&str; 2] = [MEAL_COMPONENT, CONTAINER_COMPONENT];
const TRAY_HEAT_SOURCES: &str = "farmersdelight:tray_heat_sources";
const SERVING_CONTAINERS: &str = "farmersdelight:serving_containers";
/// A recipe property set holding `#farmersdelight:serving_containers`' items, so the menu's slot
/// rules (which get no tag queries) can keep them out of the ingredient slots. The values are the
/// tag's as carried; the data-load context has no item tags to read them from.
pub const SERVING_SET: &str = "farmersdelight:serving_containers";
pub const SERVING_ITEMS: [&str; 3] = [
    "minecraft:bowl",
    "minecraft:glass_bottle",
    "minecraft:bucket",
];
const GUI: &str = "farmersdelight:gui/cooking_pot";

/// `CookingPotBlockEntity.INGREDIENT_REMAINDER_OVERRIDES`.
pub fn remainder_override(item: &str) -> Option<&'static str> {
    Some(match item {
        "minecraft:powder_snow_bucket"
        | "minecraft:axolotl_bucket"
        | "minecraft:cod_bucket"
        | "minecraft:pufferfish_bucket"
        | "minecraft:salmon_bucket"
        | "minecraft:tropical_fish_bucket" => "minecraft:bucket",
        "minecraft:suspicious_stew"
        | "minecraft:mushroom_stew"
        | "minecraft:rabbit_stew"
        | "minecraft:beetroot_soup" => "minecraft:bowl",
        "minecraft:potion"
        | "minecraft:splash_potion"
        | "minecraft:lingering_potion"
        | "minecraft:experience_bottle" => "minecraft:glass_bottle",
        _ => return None,
    })
}

/// `Mth.clamp(cookTime - 2, 0, cookTimeTotal)`.
pub fn cool(cook_time: i32, total: i32) -> i32 {
    (cook_time - 2).clamp(0, total.max(0))
}

/// `canCook`, past the recipe match: the result fits the meal slot.
pub fn result_fits(meal: Option<(u32, bool, u32)>, result_count: u32) -> bool {
    match meal {
        None => true,
        // (count, same item and components, meal's max stack)
        Some((_, false, _)) => false,
        Some((count, true, max)) => count + result_count <= max.max(64),
    }
}

/// `moveMealToOutput`: how many meals move to an output holding `output` (count, same item).
pub fn meal_to_output(meal: u32, meal_max: u32, output: Option<(u32, bool)>) -> u32 {
    match output {
        None => meal.min(meal_max),
        Some((_, false)) => 0,
        Some((count, true)) => meal.min(meal_max.saturating_sub(count)),
    }
}

/// `useStoredContainersOnMeal`: how many servings fill from `containers` into the output.
pub fn servings_to_output(
    meal: u32,
    meal_max: u32,
    containers: u32,
    output: Option<(u32, bool, u32)>,
) -> u32 {
    let smaller = meal.min(containers);
    match output {
        None => smaller.min(meal_max),
        // (count, same item, output's max stack)
        Some((count, same, max)) if same && count < max => {
            smaller.min(meal_max.saturating_sub(count))
        }
        _ => 0,
    }
}

/// `MathUtils.calcRedstoneFromItemHandler`: each slot `(count, min(64, max stack))`.
pub fn comparator(slots: &[Option<(u32, u32)>]) -> i32 {
    let mut filled = 0;
    let mut f = 0.0f32;
    for (count, limit) in slots.iter().flatten() {
        f += *count as f32 / (*limit).clamp(1, 64) as f32;
        filled += 1;
    }
    f /= slots.len().max(1) as f32;
    (f * 14.0).floor() as i32 + i32::from(filled > 0)
}

/// `splitAndSpawnExperience`: `floor(n·xp)`, plus one with the fraction as its chance.
pub fn experience(crafted: i32, xp: f32, roll: f64) -> u32 {
    let total = crafted as f32 * xp;
    let whole = total.floor();
    let fraction = total - whole;
    whole as u32 + u32::from(fraction != 0.0 && roll < fraction as f64)
}

/// `getCookProgressionScaled`: the arrow's width in pixels.
pub fn arrow_width(cook_time: i32, total: i32) -> i32 {
    if total != 0 && cook_time != 0 {
        cook_time * 24 / total
    } else {
        0
    }
}

fn int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}

fn stack_tag(host: &dyn Host, nbt: &Compound, key: &str) -> Option<ItemStack> {
    match nbt.get(&key.into()) {
        Some(Tag::Compound(c)) => ItemStack::from_nbt(host, c),
        _ => None,
    }
}

fn meal_of(host: &dyn Host, nbt: &Compound) -> Option<ItemStack> {
    stack_tag(host, nbt, MEAL_KEY)
}

/// The meal as saved: its stack, or an empty list for none (a merge cannot remove a key).
fn meal_tag(host: &dyn Host, meal: Option<&ItemStack>) -> Tag {
    meal.and_then(|m| m.to_nbt(host))
        .map_or(Tag::List(Vec::new()), Tag::Compound)
}

fn recipes_used(nbt: &Compound) -> BTreeMap<String, i32> {
    match nbt.get(&RECIPES_USED.into()) {
        Some(Tag::Compound(c)) => c
            .iter()
            .filter_map(|(k, v)| match v {
                Tag::Int(n) => Some((k.to_string_lossy(), *n)),
                _ => None,
            })
            .collect(),
        _ => BTreeMap::new(),
    }
}

fn recipes_used_tag(used: &BTreeMap<String, i32>) -> Tag {
    Tag::Compound(
        used.iter()
            .map(|(k, v)| (k.as_str().into(), Tag::Int(*v)))
            .collect(),
    )
}

fn stack_of(host: &dyn Host, id: &str) -> Option<ItemStack> {
    ItemStack::new(host.item_key(&Identifier::parse(id).ok()?)?, 1)
}

fn max_stack(host: &dyn Host, stack: &ItemStack) -> u32 {
    host.max_stack_size(stack.item()).unwrap_or(64)
}

fn in_item_tag(host: &dyn Host, stack: &ItemStack, tag: &str) -> bool {
    crate::blocks::stove::in_item_tag(host, stack, tag)
}

/// A crafting remainder that is something (`getCraftingRemainder() != null && !isEmpty`).
fn remainder(host: &dyn Host, stack: &ItemStack) -> Option<ItemStack> {
    host.item_crafting_remainder(stack)
}

/// The client's copy of the slots: a list of stack compounds with a `Slot` byte.
pub fn inventory_tag(host: &dyn Host, items: &[Option<ItemStack>]) -> Tag {
    crate::blocks::stove::inventory_tag(host, items)
}

/// Slots from the update tag's `Inventory` list (client side).
pub fn inventory_from_tag(host: &dyn Host, nbt: &Compound) -> [Option<ItemStack>; SLOTS] {
    let mut out: [Option<ItemStack>; SLOTS] = Default::default();
    if let Some(Tag::List(list)) = nbt.get(&INVENTORY.into()) {
        for t in list {
            if let Tag::Compound(c) = t
                && let Some(Tag::Byte(slot)) = c.get(&"Slot".into())
                && (0..SLOTS as i8).contains(slot)
            {
                out[*slot as usize] = ItemStack::from_nbt(host, c);
            }
        }
    }
    out
}

/// A pot being removed: what `loot_drops` still needs once the block entity is gone.
#[derive(Clone, Debug)]
pub struct RemovedPot {
    pub recipes_used: BTreeMap<String, i32>,
    pub meal: Option<Compound>,
    pub container: Option<Compound>,
}

/// State shared by the pot's parts and hooks, by block position.
#[derive(Default)]
pub struct Shared {
    /// `RecipeManager.CachedCheck`: the recipe that matched last, per pot.
    last_recipe: HashMap<[i32; 3], String>,
    /// Who opened the menu last, and the output count last seen, for the XP of a take
    /// (`CookingPotResultSlot#checkTakeAchievements`; no take hook at the pin, U-11).
    opener: HashMap<[i32; 3], u128>,
    output_seen: HashMap<[i32; 3], u32>,
    removed: HashMap<[i32; 3], RemovedPot>,
    /// The comparator signal of each pot, as its last tick computed it.
    signal: HashMap<[i32; 3], i32>,
}

pub type SharedRef = Arc<Mutex<Shared>>;

fn lock(shared: &SharedRef) -> std::sync::MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct CookingPot {
    pub recipes: Arc<RecipeTables>,
    pub shared: SharedRef,
    pub bubble_pop: ParticleKey,
    pub steam: ParticleKey,
    pub client_ticks: std::sync::atomic::AtomicU64,
}

/// `getTrayState`: a tray over a tray heat source below.
fn tray_below(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    level
        .block_state([pos[0], pos[1] - 1, pos[2]])
        .is_some_and(|s| level.block_in_tag(s.block(), TRAY_HEAT_SOURCES) == Some(true))
}

#[crate::engine::part]
impl BlockPart for CookingPot {
    /// `getStateForPlacement`: facing opposite the placer, waterlogged in water, a handle when
    /// placed against a ceiling, else a tray over a tray heat source.
    fn state_for_placement(
        &self,
        level: LevelSide<'_>,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        let pos = [
            event.context.pos.x,
            event.context.pos.y,
            event.context.pos.z,
        ];
        let water = level
            .block_state_id("minecraft:water")
            .and_then(|w| level.state_fluid_state(w))
            .is_some_and(|w| w.fluid == event.fluid.fluid);
        let support = if event.context.face == Direction::Down {
            "handle"
        } else if tray_below(level, pos) {
            "tray"
        } else {
            "none"
        };
        let state =
            level.state_with_property(event.state, "facing", facing_for_yaw(event.context.yaw))?;
        let state = level.state_with_property(
            state,
            "waterlogged",
            if water { "true" } else { "false" },
        )?;
        Some(level.state_with_property(state, "support", support))
    }

    /// `updateShape`: water ticks while waterlogged; a vertical neighbour change recomputes
    /// the tray unless the pot hangs from its handle.
    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        let name = level.state_string(event.state)?;
        if state_value(&name, "waterlogged") == Some("true") {
            level.schedule_fluid_tick(pos, "minecraft:water", 5);
        }
        if matches!(event.direction, Direction::Up | Direction::Down)
            && state_value(&name, "support") != Some("handle")
        {
            let tray = if tray_below(&*level, pos) {
                "tray"
            } else {
                "none"
            };
            return level.state_with_property(event.state, "support", tray);
        }
        None
    }

    /// Water stays where a waterlogged pot is removed (template has no fluid; E-13).
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

    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        Some(self.interact(host, event.click, event.state))
    }

    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        Some(self.interact(host, event.click, event.state))
    }

    fn has_analog_output(&self, _: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }

    /// Read from the signal the tick keeps (the hook's level host has no block-entity read).
    fn analog_output(&self, _: &mut dyn LevelRandomHost, event: &mut AnalogOutputEvent) {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        event.output = lock(&self.shared).signal.get(&pos).copied().unwrap_or(0);
    }
}

impl CookingPot {
    /// `CookingPotBlock#useItemOn`, main hand only (it never passes, so vanilla never tries the
    /// off hand; the client's predicted off-hand round does nothing).
    fn interact(
        &self,
        host: &mut dyn ServerHost,
        click: &BlockClick<'_>,
        state: BlockState,
    ) -> InteractionResult {
        if click.hand == Hand::Off {
            return InteractionResult::Consume;
        }
        let pos = click.pos;
        let Some(player) = host.players().acting_player() else {
            return InteractionResult::Pass;
        };
        let held = click.stack.cloned();
        let sneaking = player.entity().is_shift_key_down(&*host).unwrap_or(false);
        if held.is_none() && sneaking {
            let name = host.block_state_string(state).unwrap_or_default();
            let next = if state_value(&name, "support") == Some("handle") {
                let level = host.level(click.dimension.key().id());
                let tray = level.is_some_and(|l| {
                    l.block_state(&*host, [pos[0], pos[1] - 1, pos[2]])
                        .is_some_and(|s| {
                            host.block_in_tag(s.block(), TRAY_HEAT_SOURCES) == Some(true)
                        })
                });
                if tray { "tray" } else { "none" }
            } else {
                "handle"
            };
            host.set_block_property_with(pos, "support", next, 3);
            if let Some(level) = host.level(click.dimension.key().id()) {
                let at = [
                    pos[0] as f64 + 0.5,
                    pos[1] as f64 + 0.5,
                    pos[2] as f64 + 0.5,
                ];
                level.play_sound(
                    host,
                    "minecraft:block.lantern.place",
                    at,
                    SoundSource::Blocks,
                    0.7,
                    1.0,
                    None,
                );
            }
            return InteractionResult::Success;
        }
        let Some(level) = host.level(click.dimension.key().id()) else {
            return InteractionResult::Success;
        };
        let Some(be) = host.block_entity(level, pos) else {
            return InteractionResult::Success;
        };
        // `useHeldItemOnMeal`: a valid container in hand takes one serving.
        let data = be.data(&*host).unwrap_or_default();
        let meal = meal_of(&*host, &data);
        let container = stack_tag(&*host, &data, CONTAINER_KEY);
        if let (Some(held), Some(meal), Some(container)) = (&held, &meal, &container)
            && held.same_item(container)
        {
            let split = meal.clone().split(1);
            let serving = split.taken.expect("meal is not empty");
            let creative = player.is_creative(&*host).unwrap_or(false);
            let mut merge = Compound::new();
            merge.insert(MEAL_KEY.into(), meal_tag(&*host, split.remainder.as_ref()));
            let ok = be.merge_data(host, merge).unwrap_or(false)
                && host
                    .transaction(|host, tx| {
                        crate::inventory::give_or_drop(host, tx, player, serving.clone());
                        Ok::<_, ()>(())
                    })
                    .is_ok();
            if ok {
                if !creative && let Some(living) = host.living(player.entity()) {
                    let _ = living.set_hand_item(host, Hand::Main, held.clone().shrink(1));
                }
                let at = [
                    pos[0] as f64 + 0.5,
                    pos[1] as f64 + 0.5,
                    pos[2] as f64 + 0.5,
                ];
                level.play_sound(
                    host,
                    "minecraft:item.armor.equip_generic",
                    at,
                    SoundSource::Blocks,
                    1.0,
                    1.0,
                    None,
                );
                self.mirror(host, be);
            }
            return InteractionResult::Success;
        }
        crate::recipes::book::close_if_disabled(host, player);
        let _ = player.open_menu(host, be);
        {
            let mut s = lock(&self.shared);
            s.opener.insert(pos, player.uuid());
            let out = be
                .item_slot(&*host, OUTPUT as u32)
                .ok()
                .flatten()
                .map_or(0, |s| s.count());
            s.output_seen.insert(pos, out);
        }
        InteractionResult::Success
    }

    /// Sends the client the slots and the container (`getUpdateTag`).
    fn mirror(&self, host: &mut dyn ServerHost, be: BlockEntityRef) {
        let mut items: Vec<_> = (0..SLOTS)
            .map(|i| be.item_slot(&*host, i as u32).ok().flatten())
            .collect();
        items[MEAL] = be.data(&*host).and_then(|d| meal_of(&*host, &d));
        let mut merge = Compound::new();
        merge.insert(INVENTORY.into(), inventory_tag(&*host, &items));
        let _ = be.merge_data(host, merge);
        if let Some(pos) = be.position(&*host) {
            host.block_entity_sync(pos);
        }
    }

    /// The meal's container: the recipe's, else the result's crafting remainder, else the
    /// override map's (`CookingPotRecipe#container`).
    fn container_for(&self, host: &dyn Host, recipe: &CookingRecipe) -> Option<ItemStack> {
        recipe
            .container
            .clone()
            .or_else(|| remainder(host, &recipe.result))
            .or_else(|| {
                remainder_override(&recipe.result.item().id(host)).and_then(|id| stack_of(host, id))
            })
    }
}

/// `awardUsedRecipes` / `getUsedRecipesAndPopExperience`: orbs at `at` and the recipes to
/// `player`, for every recipe cooked since the last award.
pub fn award(
    recipes: &RecipeTables,
    host: &mut dyn ServerHost,
    level: LevelRef,
    at: [f64; 3],
    player: Option<PlayerRef>,
    used: &BTreeMap<String, i32>,
) {
    let table = recipes.cooking.snapshot();
    let mut awarded = Vec::new();
    for (id, count) in used {
        let Some(recipe) = table.iter().find(|r| &r.id == id) else {
            continue;
        };
        awarded.push(id.as_str());
        let roll = level
            .random(&*host)
            .map_or(0.5, |mut r| r.next_float() as f64);
        let xp = experience(*count, recipe.experience, roll);
        if xp > 0 {
            let _ = level.award_experience(host, at[0], at[1], at[2], xp);
        }
    }
    if let Some(player) = player
        && !awarded.is_empty()
    {
        let _ = player.award_recipes(host, &awarded);
    }
}

#[crate::engine::part]
impl BlockEntityPart for CookingPot {
    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// `CookingPotBlockEntity#cookingTick`.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let pos = event.pos;
        let entity = &mut event.entity;
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        if entity.items.len() < SLOTS {
            entity.items.resize(SLOTS, None);
        }
        // The meal works in slot 6 for the tick and goes back to the saved data at its end.
        let saved_meal = meal_of(&*host, &entity.nbt);
        if let Some(stray) = entity.items[MEAL].take() {
            entity.items[MEAL] = Some(stray);
        } else {
            entity.items[MEAL] = saved_meal;
        }
        let out_start = entity.items[OUTPUT].as_ref().map_or(0, |s| s.count());
        let mut cook_time = int(&entity.nbt, COOK_TIME);
        let mut total = int(&entity.nbt, COOK_TIME_TOTAL);
        let mut changed = false;
        let has_input = entity.items[..INPUTS].iter().any(Option::is_some);
        let heated = crate::heat::is_heated_in_level_ref(&*host, level, pos);
        if heated && has_input {
            let table = self.recipes.cooking.snapshot();
            let last = lock(&self.shared).last_recipe.get(&pos).cloned();
            let found = cooking::find(&table, last.as_deref(), &entity.items[..INPUTS], |i, s| {
                i.test(&*host, s)
            });
            let fits = found.is_some_and(|r| {
                let meal = entity.items[MEAL].as_ref().map(|m| {
                    (
                        m.count(),
                        m.is_stackable_with(&r.result),
                        max_stack(&*host, m),
                    )
                });
                result_fits(meal, r.result.count())
            });
            match found {
                Some(recipe) if fits => {
                    lock(&self.shared)
                        .last_recipe
                        .insert(pos, recipe.id.clone());
                    cook_time += 1;
                    total = recipe.cooking_time;
                    if cook_time >= total {
                        cook_time = 0;
                        self.finish(host, level, pos, entity, recipe);
                        changed = true;
                    }
                }
                _ => cook_time = cool(cook_time, total),
            }
        } else if cook_time > 0 {
            cook_time = cool(cook_time, total);
        }
        if let Some(meal) = entity.items[MEAL].clone() {
            let container = stack_tag(&*host, &entity.nbt, CONTAINER_KEY);
            let needs_container = container.is_some() || remainder(&*host, &meal).is_some();
            let meal_max = max_stack(&*host, &meal);
            if !needs_container {
                let out = entity.items[OUTPUT]
                    .as_ref()
                    .map(|o| (o.count(), o.is_stackable_with(&meal)));
                let n = meal_to_output(meal.count(), meal_max, out);
                changed |= self.move_meal(entity, n, None);
            } else if let Some(held) = entity.items[CONTAINER].clone() {
                let valid = container.as_ref().is_some_and(|c| c.same_item(&held));
                let out = entity.items[OUTPUT]
                    .as_ref()
                    .map(|o| (o.count(), o.is_stackable_with(&meal), max_stack(&*host, o)));
                let n = if valid {
                    servings_to_output(meal.count(), meal_max, held.count(), out)
                } else {
                    0
                };
                changed |= self.move_meal(entity, n, Some(held));
            }
        }
        entity.nbt.insert(COOK_TIME.into(), Tag::Int(cook_time));
        entity.nbt.insert(COOK_TIME_TOTAL.into(), Tag::Int(total));
        entity.data.resize(2, 0);
        entity.data[0] = cook_time;
        entity.data[1] = total;
        // A take from the output while a player has the menu open (the result slot's XP).
        let out_now = entity.items[OUTPUT].as_ref().map_or(0, |s| s.count());
        let taken_by = {
            let mut s = lock(&self.shared);
            let seen = s.output_seen.insert(pos, out_now).unwrap_or(out_now);
            (out_start < seen)
                .then(|| s.opener.get(&pos).copied())
                .flatten()
        };
        if let Some(uuid) = taken_by
            && let Some(player) = host.player(uuid)
        {
            let used = recipes_used(&entity.nbt);
            if !used.is_empty() {
                let at = player.entity().position(&*host).unwrap_or([
                    pos[0] as f64,
                    pos[1] as f64,
                    pos[2] as f64,
                ]);
                award(&self.recipes, host, level, at, Some(player), &used);
                entity.nbt.remove(&RECIPES_USED.into());
            }
        }
        let slots: Vec<_> = entity
            .items
            .iter()
            .map(|s| s.as_ref().map(|s| (s.count(), max_stack(&*host, s))))
            .collect();
        let signal = comparator(&slots);
        if lock(&self.shared).signal.insert(pos, signal) != Some(signal)
            && let Some(state) = level.block_state(&*host, pos)
        {
            let _ = level.update_neighbors(host, pos, state, NeighbourUpdate::OutputSignal);
        }
        if changed {
            let tag = inventory_tag(&*host, &entity.items);
            entity.nbt.insert(INVENTORY.into(), tag);
            host.block_entity_sync(pos);
        }
        let meal = entity.items[MEAL].take();
        let tag = meal_tag(&*host, meal.as_ref());
        entity.nbt.insert(MEAL_KEY.into(), tag);
    }

    /// The meal, its container and the pending experience go to
    /// `loot_drops` (the block entity is gone by then).
    fn pre_remove_side_effects(
        &self,
        _: &mut dyn LevelRandomHost,
        event: &mut PreRemoveSideEffectsEvent<'_>,
    ) {
        // The game drops the container's own slots; the meal is not one of them.
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        let container = match event.entity.nbt.get(&CONTAINER_KEY.into()) {
            Some(Tag::Compound(c)) => Some(c.clone()),
            _ => None,
        };
        let removed = RemovedPot {
            recipes_used: recipes_used(&event.entity.nbt),
            meal: match event.entity.nbt.get(&MEAL_KEY.into()) {
                Some(Tag::Compound(c)) => Some(c.clone()),
                _ => None,
            },
            container,
        };
        let mut s = lock(&self.shared);
        s.removed.insert(pos, removed);
        s.last_recipe.remove(&pos);
        s.opener.remove(&pos);
        s.output_seen.remove(&pos);
        s.signal.remove(&pos);
    }

    fn block_entity_update_tag(
        &self,
        event: &mut BlockEntityUpdateTagEvent<'_>,
    ) -> Option<Compound> {
        let mut tag = Compound::new();
        for key in [INVENTORY, CONTAINER_KEY] {
            if let Some(v) = event.entity.nbt.get(&key.into()) {
                tag.insert(key.into(), v.clone());
            }
        }
        Some(tag)
    }

    /// `CookingPotMenu#quickMoveStack`, from the inventory: serving containers and the meal's
    /// container go to the container slot, the rest to the ingredient slots. Only a partial
    /// move is taken here; a stack that moves whole is left to the game's routing, as the
    /// hook cannot empty an inventory slot (E-17).
    fn menu_quick_move(
        &self,
        host: &mut dyn ServerHost,
        event: &mut MenuQuickMoveEvent<'_>,
    ) -> Flow {
        if event.slot == OUTPUT {
            // The result slot's take: experience and recipes to the player.
            if let (Some(player), Some(level)) =
                (host.players().acting_player(), host.level(host.dimension()))
            {
                let used = recipes_used(&event.entity.nbt);
                if !used.is_empty() {
                    let at = player.entity().position(&*host).unwrap_or([0.0; 3]);
                    award(&self.recipes, host, level, at, Some(player), &used);
                    event.entity.nbt.remove(&RECIPES_USED.into());
                }
            }
            return Flow::Continue;
        }
        if event.slot < event.container_slots {
            return Flow::Continue;
        }
        let Some(stack) = event.stack.clone() else {
            return Flow::Continue;
        };
        let container = stack_tag(&*host, &event.entity.nbt, CONTAINER_KEY).or_else(|| {
            event
                .entity
                .items
                .get(MEAL)
                .cloned()
                .flatten()
                .and_then(|m| remainder(&*host, &m))
        });
        let to_container = in_item_tag(&*host, &stack, SERVING_CONTAINERS)
            || container.is_some_and(|c| c.same_item(&stack));
        let targets: Vec<usize> = if to_container {
            vec![CONTAINER]
        } else {
            (0..INPUTS).collect()
        };
        let max = max_stack(&*host, &stack).min(64);
        let before = stack.count();
        let left = move_into(&mut event.entity.items, &targets, stack, max);
        match left {
            Some(rest) if rest.count() < before => {
                *event.stack = Some(rest);
                Flow::Cancel
            }
            _ => Flow::Continue,
        }
    }

    fn block_entity_client_ticks(
        &self,
        _: &dyn ClientHost,
        event: &mut BlockEntityClientTicksEvent<'_>,
    ) {
        event.ticks = true;
    }

    /// `CookingPotBlockEntity#animationTick` (bubble pop 20%, steam 5%) and
    /// `CookingPotBlock#animateTick` (boil sounds 1 in 10), while heated.
    fn block_entity_client_tick(
        &self,
        host: &mut dyn ClientHost,
        event: &mut BlockEntityClientTickEvent<'_>,
    ) {
        let pos = event.pos;
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        if !crate::heat::is_heated_in_level_ref(&*host, level, pos) {
            return;
        }
        let tick = self
            .client_ticks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut rng = crate::blocks::stove::ClientRng::new(pos, tick as i64);
        let (x, y, z) = (pos[0] as f64 + 0.5, pos[1] as f64, pos[2] as f64 + 0.5);
        if rng.next() < 0.2 {
            let at = [
                x + rng.next() * 0.6 - 0.3,
                y + 0.7,
                z + rng.next() * 0.6 - 0.3,
            ];
            let _ = host.add_particle(&ParticleOptions::simple(self.bubble_pop), at, [0.0; 3]);
        }
        if rng.next() < 0.05 {
            let at = [
                x + rng.next() * 0.4 - 0.2,
                y + 0.5,
                z + rng.next() * 0.4 - 0.2,
            ];
            let vy = if rng.next() < 0.5 { 0.015 } else { 0.005 };
            let _ = host.add_particle(&ParticleOptions::simple(self.steam), at, [0.0, vy, 0.0]);
        }
        if rng.next() < 0.1 {
            let meal = inventory_from_tag(&*host, &event.entity.nbt)[MEAL].is_some();
            let id = if meal {
                "farmersdelight:block.cooking_pot.boil_soup"
            } else {
                "farmersdelight:block.cooking_pot.boil"
            };
            let mut sound = SoundPlay::ui(id);
            sound.source = SoundSource::Blocks;
            sound.position = [x, y, z];
            sound.relative = false;
            sound.attenuation = SoundAttenuation::Linear;
            sound.volume = 0.5;
            sound.pitch = (rng.next() * 0.2 + 0.9) as f32;
            let _ = host.play_local_sound(&sound, 0);
        }
    }
}

fn client_nbt(host: &dyn ClientHost, pos: [i32; 3]) -> Option<Compound> {
    let level = host.level(host.dimension())?;
    host.block_entity(level, pos)?.data(host)
}

fn client_meal(host: &dyn ClientHost, pos: [i32; 3]) -> Option<ItemStack> {
    let nbt = client_nbt(host, pos)?;
    inventory_from_tag(host, &nbt)[MEAL].clone()
}

fn client_container(host: &dyn ClientHost, pos: [i32; 3]) -> Option<ItemStack> {
    stack_tag(host, &client_nbt(host, pos)?, CONTAINER_KEY)
}

/// `moveItemStackTo` (not reversed): first top up stacks of the same item, then fill empty
/// slots, each up to `max`. Returns what is left.
pub fn move_into(
    items: &mut [Option<ItemStack>],
    targets: &[usize],
    stack: ItemStack,
    max: u32,
) -> Option<ItemStack> {
    let mut left = Some(stack);
    for &i in targets {
        let Some(rest) = left.take() else { break };
        match &mut items[i] {
            Some(s) if s.is_stackable_with(&rest) && s.count() < max => {
                let n = (max - s.count()).min(rest.count());
                let _ = s.set_count(s.count() + n);
                left = rest.shrink(n);
            }
            _ => left = Some(rest),
        }
    }
    for &i in targets {
        let Some(rest) = left.take() else { break };
        if items[i].is_none() {
            let split = rest.split(max);
            items[i] = split.taken;
            left = split.remainder;
        } else {
            left = Some(rest);
        }
    }
    left
}

impl CookingPot {
    /// `processCooking`, at the finish: the meal, the container, the recipe count, and one of
    /// each ingredient used with its remainder ejected.
    fn finish(
        &self,
        host: &mut dyn ServerHost,
        level: LevelRef,
        pos: [i32; 3],
        entity: &mut ModBlockEntityView<'_>,
        recipe: &CookingRecipe,
    ) {
        match self
            .container_for(&*host, recipe)
            .and_then(|c| c.to_nbt(&*host))
        {
            Some(c) => {
                entity.nbt.insert(CONTAINER_KEY.into(), Tag::Compound(c));
            }
            None => {
                entity.nbt.remove(&CONTAINER_KEY.into());
            }
        }
        match &mut entity.items[MEAL] {
            None => entity.items[MEAL] = Some(recipe.result.clone()),
            Some(meal) if meal.is_stackable_with(&recipe.result) => {
                let _ = meal.set_count(meal.count() + recipe.result.count());
            }
            Some(_) => {}
        }
        let mut used = recipes_used(&entity.nbt);
        *used.entry(recipe.id.clone()).or_default() += 1;
        entity
            .nbt
            .insert(RECIPES_USED.into(), recipes_used_tag(&used));
        let name = level
            .block_state(&*host, pos)
            .and_then(|s| host.block_state_string(s))
            .unwrap_or_default();
        let [dx, dz] = counter_clockwise_step(state_value(&name, "facing").unwrap_or("north"));
        for i in 0..INPUTS {
            let Some(stack) = entity.items[i].clone() else {
                continue;
            };
            let rest = remainder(&*host, &stack).or_else(|| {
                remainder_override(&stack.item().id(&*host)).and_then(|id| stack_of(&*host, id))
            });
            if let Some(rest) = rest {
                let _ = level.spawn_item_with_motion(
                    host,
                    pos[0] as f64 + 0.5 + dx * 0.25,
                    pos[1] as f64 + 0.7,
                    pos[2] as f64 + 0.5 + dz * 0.25,
                    rest,
                    [dx * 0.08, 0.25, dz * 0.08],
                    0,
                );
            }
            entity.items[i] = stack.shrink(1);
        }
    }

    /// Moves `n` meals to the output, using as many of `containers` (the container slot).
    fn move_meal(
        &self,
        entity: &mut ModBlockEntityView<'_>,
        n: u32,
        containers: Option<ItemStack>,
    ) -> bool {
        if n == 0 {
            return false;
        }
        let Some(meal) = entity.items[MEAL].take() else {
            return false;
        };
        let split = meal.split(n);
        entity.items[MEAL] = split.remainder;
        let taken = split.taken.expect("n > 0");
        match &mut entity.items[OUTPUT] {
            None => entity.items[OUTPUT] = Some(taken),
            Some(out) => {
                let _ = out.set_count(out.count() + n);
            }
        }
        if let Some(c) = containers {
            entity.items[CONTAINER] = c.shrink(n);
        }
        true
    }
}

/// `CookingPotScreen`'s geometry: six inputs, the meal display (drawn, never a slot), container and output.
/// `CookingPotMenu.EMPTY_CONTAINER_SLOT_BOWL`.
const EMPTY_CONTAINER_SLOT_BOWL: &str = "farmersdelight:container/slot/bowl";

pub fn screen_layout() -> Screen {
    let slot = |x: f32, y: f32, icon: Option<&str>| {
        Some(ScreenSlot {
            position: [x, y],
            no_item_icon: icon.map(Into::into),
        })
    };
    let mut slots: Vec<Option<ScreenSlot>> = (0..INPUTS)
        .map(|i| {
            slot(
                30.0 + 18.0 * (i % 3) as f32,
                17.0 + 18.0 * (i / 3) as f32,
                None,
            )
        })
        .collect();
    slots.push(None); // meal display: drawn below, never clicked (CookingPotMealSlot)
    // `CookingPotMenu`'s container slot shows `EMPTY_CONTAINER_SLOT_BOWL` while empty.
    slots.push(slot(92.0, 55.0, Some(EMPTY_CONTAINER_SLOT_BOWL)));
    slots.push(slot(124.0, 55.0, None));
    Screen {
        width: 176.0,
        height: 166.0,
        slots,
        inventory_x: 8.0,
        inventory_y: 84.0,
        own_labels: false,
        own_background: true,
        ..Screen::default()
    }
}

/// The pot's menu type: the meal slot takes nothing (`CookingPotMealSlot#mayPlace`).
pub struct CookingPotMenu;

#[crate::engine::part]
impl MenuPart for CookingPotMenu {
    fn screen(&self, _: &dyn Host, _: &mut ScreenEvent<'_>) -> Option<Screen> {
        Some(crate::recipes::book::screen())
    }

    /// `CookingPotScreen#renderBg`: the background, the heat icon while heated, the arrow, and
    /// the meal display drawn from the pot's synced slots.
    fn screen_draw(&self, host: &dyn ClientHost, event: &mut ScreenDrawEvent<'_>) {
        let [x, y] = event.origin;
        event
            .draw
            .sprite(GUI, [0.0, 0.0, 176.0, 166.0], [x, y, 176.0, 166.0], None);
        let Some(pos) = host.screen_pos() else {
            return;
        };
        let level = host.level(host.dimension());
        if level.is_some_and(|l| crate::heat::is_heated_in_level_ref(host, l, pos)) {
            event.draw.sprite(
                GUI,
                [176.0, 0.0, 17.0, 15.0],
                [x + 47.0, y + 55.0, 17.0, 15.0],
                None,
            );
        }
        let cook = event.data.first().copied().unwrap_or(0);
        let total = event.data.get(1).copied().unwrap_or(0);
        let w = (arrow_width(cook, total) + 1) as f32;
        event.draw.sprite(
            GUI,
            [176.0, 15.0, w, 17.0],
            [x + 89.0, y + 25.0, w, 17.0],
            None,
        );
        if let Some(meal) = client_meal(host, pos) {
            event.draw.item_stack(&meal, [x + 124.0, y + 26.0], 16.0);
            if meal.count() > 1 {
                let n = meal.count().to_string();
                let tw = event.draw.text_width(&n);
                event
                    .draw
                    .text(&n, [x + 141.0 - tw, y + 35.0], 0xFFFF_FFFF, true);
            }
        }
    }

    /// Heat icon (heated / needs heat) and meal tooltips (`CookingPotScreen#extractTooltip`). The
    /// meal slot's tooltip is replaced by its name and "Served on: …" in gray, as FD does.
    fn screen_tooltip(&self, host: &dyn ClientHost, event: &mut ScreenTooltipEvent<'_>) {
        const WHITE: u32 = 0xFFFF_FFFF;
        const GRAY: u32 = 0xFFAA_AAAA;
        let [mx, my] = event.at;
        let inside =
            |rx: f32, ry: f32, w: f32, h: f32| mx >= rx && mx < rx + w && my >= ry && my < ry + h;
        let Some(pos) = host.screen_pos() else {
            return;
        };
        let tr = |k: &str| host.translate(k).unwrap_or_else(|| k.to_owned());
        if inside(47.0, 55.0, 17.0, 15.0) {
            let heated = host
                .level(host.dimension())
                .is_some_and(|l| crate::heat::is_heated_in_level_ref(host, l, pos));
            let line = tr(if heated {
                "container.farmersdelight.cooking_pot.heated"
            } else {
                "container.farmersdelight.cooking_pot.not_heated"
            });
            event.lines.push((Text::literal(line), WHITE));
        } else if inside(124.0, 26.0, 16.0, 16.0)
            && let Some(meal) = client_meal(host, pos)
        {
            let key = |s: &ItemStack| {
                let id = s.item().id(host);
                let (ns, path) = id.split_once(':').unwrap_or(("minecraft", &id));
                format!("item.{ns}.{path}")
            };
            event.lines.clear();
            event.lines.push((Text::literal(tr(&key(&meal))), WHITE));
            let container =
                client_container(host, pos).or_else(|| host.item_crafting_remainder(&meal));
            if let Some(c) = container {
                let served = tr("container.farmersdelight.cooking_pot.served_on");
                event
                    .lines
                    .push((Text::literal(served.replace("%s", &tr(&key(&c)))), GRAY));
            }
        }
    }

    /// Serving containers stay out of the ingredient slots (accepted delta A13): no recipe uses
    /// one, and the game's routing then sends a whole shift-clicked stack of them to the container
    /// slot, as `quickMoveStack` does (E-17).
    fn menu_slot_rules(&self, context: &dyn Host, event: &mut MenuSlotRulesEvent<'_>) {
        if event.slot == MEAL
            || (event.slot < INPUTS
                && context.mod_recipe_property_contains(SERVING_SET, event.stack.item()))
        {
            event.may_place = false;
        }
    }
}

/// The dropped pot keeps its meal and container (`minecraft:copy_components` of
/// `farmersdelight:meal`/`container`, which needs the block entity Horizon's loot context does
/// not carry), and the pending experience pops at the block.
pub fn on_loot_drops(
    recipes: &RecipeTables,
    shared: &SharedRef,
    host: &mut dyn ServerHost,
    event: &mut LootDropsEvent<'_>,
) {
    if event.context.table != "farmersdelight:blocks/cooking_pot" {
        return;
    }
    let Some((pos, _)) = event.context.block else {
        return;
    };
    let Some(removed) = lock(shared).removed.remove(&pos) else {
        return;
    };
    if let Some(level) = host.level(event.context.dimension) {
        let at = [
            pos[0] as f64 + 0.5,
            pos[1] as f64 + 0.5,
            pos[2] as f64 + 0.5,
        ];
        award(recipes, host, level, at, None, &removed.recipes_used);
    }
    let Some(meal_nbt) = removed.meal else {
        return;
    };
    // `getContainer`: the stored container, else the meal's crafting remainder.
    let container = removed.container.or_else(|| {
        ItemStack::from_nbt(&*host, &meal_nbt)
            .and_then(|m| remainder(&*host, &m))
            .and_then(|c| c.to_nbt(&*host))
    });
    for drop in event.drops.iter_mut() {
        if drop.item().id(&*host) != ID {
            continue;
        }
        let mut patch = crate::items::skillet::patch_of(drop);
        patch.insert(MEAL_COMPONENT.into(), Tag::Compound(meal_nbt.clone()));
        // Placing the pot brings the meal back: the engine merges `minecraft:block_entity_data`
        // into the new block entity (`applyImplicitComponents`' job at the pin).
        let mut data = Compound::new();
        data.insert("id".into(), Tag::String(ID.into()));
        data.insert(MEAL_KEY.into(), Tag::Compound(meal_nbt.clone()));
        if let Some(c) = &container {
            patch.insert(CONTAINER_COMPONENT.into(), Tag::Compound(c.clone()));
            data.insert(CONTAINER_KEY.into(), Tag::Compound(c.clone()));
        }
        patch.insert("minecraft:block_entity_data".into(), Tag::Compound(data));
        let _ = drop.set_components(patch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mod_api::prelude::ItemsHost;

    #[test]
    fn cooling_clamps_to_the_total() {
        assert_eq!(cool(5, 200), 3);
        assert_eq!(cool(1, 200), 0);
        // the clamp's upper bound is the (possibly smaller) recipe total
        assert_eq!(cool(150, 100), 100);
    }

    #[test]
    fn result_fit_uses_at_least_64() {
        assert!(result_fits(None, 1));
        assert!(!result_fits(Some((1, false, 16)), 1));
        // bowl meals stack to 16 but the meal slot holds 64 of them
        assert!(result_fits(Some((63, true, 16)), 1));
        assert!(!result_fits(Some((64, true, 16)), 1));
        assert!(result_fits(Some((62, true, 64)), 2));
    }

    #[test]
    fn meals_move_up_to_the_item_stack_size() {
        assert_eq!(meal_to_output(40, 16, None), 16);
        assert_eq!(meal_to_output(40, 16, Some((10, true))), 6);
        assert_eq!(meal_to_output(40, 16, Some((10, false))), 0);
        assert_eq!(meal_to_output(3, 64, Some((60, true))), 3);
    }

    #[test]
    fn servings_take_one_container_each() {
        assert_eq!(servings_to_output(10, 16, 3, None), 3);
        assert_eq!(servings_to_output(2, 16, 64, None), 2);
        assert_eq!(servings_to_output(10, 16, 10, Some((12, true, 16))), 4);
        // a full output, or another item in it, takes nothing
        assert_eq!(servings_to_output(10, 16, 10, Some((16, true, 16))), 0);
        assert_eq!(servings_to_output(10, 16, 10, Some((1, false, 64))), 0);
    }

    #[test]
    fn comparator_follows_the_handler_formula() {
        let mut slots = vec![None; SLOTS];
        assert_eq!(comparator(&slots), 0);
        slots[0] = Some((1, 64));
        assert_eq!(comparator(&slots), 1);
        // 9 full slots → 15
        let full = vec![Some((64, 64)); SLOTS];
        assert_eq!(comparator(&full), 15);
        // a 16-stack item counts against 16: 16/16 in one slot of nine → floor(14/9)+1 = 2
        slots[0] = Some((16, 16));
        assert_eq!(comparator(&slots), 2);
    }

    #[test]
    fn experience_rounds_by_chance() {
        assert_eq!(experience(10, 1.0, 0.99), 10);
        assert_eq!(experience(3, 0.35, 0.04), 2); // 1.05 → 1 + (0.04 < 0.05)
        assert_eq!(experience(3, 0.35, 0.06), 1);
        assert_eq!(experience(0, 1.0, 0.0), 0);
    }

    #[test]
    fn arrow_scales_to_24() {
        assert_eq!(arrow_width(0, 200), 0);
        assert_eq!(arrow_width(100, 200), 12);
        assert_eq!(arrow_width(199, 200), 23);
        assert_eq!(arrow_width(5, 0), 0);
    }

    #[test]
    fn remainder_overrides() {
        assert_eq!(
            remainder_override("minecraft:cod_bucket"),
            Some("minecraft:bucket")
        );
        assert_eq!(
            remainder_override("minecraft:mushroom_stew"),
            Some("minecraft:bowl")
        );
        assert_eq!(
            remainder_override("minecraft:experience_bottle"),
            Some("minecraft:glass_bottle")
        );
        assert_eq!(remainder_override("minecraft:water_bucket"), None);
    }

    fn stack(id: &str, n: u32) -> ItemStack {
        let mut fake = mod_api::testing::FakeHost::new();
        fake.registries
            .resolve_items(
                ["minecraft:bowl", "minecraft:carrot"]
                    .iter()
                    .map(|i| Identifier::parse(i).unwrap()),
            )
            .unwrap();
        let key = fake.item_key(&Identifier::parse(id).unwrap()).unwrap();
        ItemStack::new(key, n).unwrap()
    }

    #[test]
    fn move_into_tops_up_then_fills() {
        let mut items: Vec<Option<ItemStack>> = vec![None; SLOTS];
        items[1] = Some(stack("minecraft:carrot", 60));
        let left = move_into(&mut items, &[0, 1, 2], stack("minecraft:carrot", 10), 64);
        assert!(left.is_none());
        assert_eq!(items[1].as_ref().unwrap().count(), 64);
        assert_eq!(items[0].as_ref().unwrap().count(), 6);
        // the container slot alone, already holding another item: nothing moves
        items[CONTAINER] = Some(stack("minecraft:carrot", 1));
        let left = move_into(&mut items, &[CONTAINER], stack("minecraft:bowl", 5), 64).unwrap();
        assert_eq!(left.count(), 5);
    }
}
