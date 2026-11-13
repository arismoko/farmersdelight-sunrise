//! Placed skillet (J/common/block/SkilletBlock.java, J/common/block/entity/SkilletBlockEntity.java).
//!
//! One slot holding a whole stack, cooked one unit at a time with campfire recipes while heated
//! (heat from below, conductors count): +1/tick, at the total the result pops out toward
//! `facing.getClockWise()` and one unit is used; unheated, progress −2/tick. Waterlogged, the
//! contents drop and nothing goes in. `Skillet` keeps the placed item (enchantments, damage) for
//! the drop. NBT: `CookTime`, `CookTimeTotal`, `Skillet`; `Inventory` mirrored for the client.
use super::cutting_board::state_value;
use crate::items::skillet::{set_prop, skillet_cooking_time};
use mod_api::*;
use std::collections::BTreeMap;

pub const ID: &str = "farmersdelight:skillet";
pub const COOK_TIME: &str = "CookTime";
pub const COOK_TIME_TOTAL: &str = "CookTimeTotal";
pub const SKILLET: &str = "Skillet";
pub const INVENTORY: &str = "Inventory";
const TRAY_HEAT_SOURCES: &str = "farmersdelight:tray_heat_sources";
const SOUND_ADD_FOOD: &str = "farmersdelight:block.skillet.add_food";

/// One heated tick (`cookAndOutputItems`): returns the new progress and whether a unit finished.
pub fn heated_step(progress: i32, total: i32) -> (i32, bool) {
    let p = progress + 1;
    if p >= total { (0, true) } else { (p, false) }
}

/// One cold tick: −2, clamped to `[0, total]`.
pub fn cool_step(progress: i32, total: i32) -> i32 {
    if progress > 0 {
        (progress - 2).clamp(0, total.max(0))
    } else {
        progress
    }
}

fn int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}

fn clockwise_step(facing: &str) -> [f64; 2] {
    match facing {
        "north" => [1.0, 0.0],  // east
        "east" => [0.0, 1.0],   // south
        "south" => [-1.0, 0.0], // west
        _ => [0.0, -1.0],       // north
    }
}

fn fire_aspect_of(host: &dyn Host, nbt: &Compound) -> i32 {
    let Some(Tag::Compound(c)) = nbt.get(&SKILLET.into()) else {
        return 0;
    };
    let Some(stack) = ItemStack::from_nbt(host, c) else {
        return 0;
    };
    match stack
        .components()
        .and_then(|p| p.as_nbt().get(&"minecraft:enchantments".into()).cloned())
    {
        Some(Tag::Compound(levels)) => match levels.get(&"minecraft:fire_aspect".into()) {
            Some(Tag::Int(l)) => *l,
            _ => 0,
        },
        _ => 0,
    }
}

fn campfire(host: &dyn Host, level: LevelRef, stack: &ItemStack) -> Option<RecipeMatch> {
    let kind = Identifier::parse("minecraft:campfire_cooking").ok()?;
    level
        .recipe_for(host, &kind, &RecipeInput::Single(stack.clone()))
        .ok()
        .flatten()
}

fn inventory_tag(host: &dyn Host, stack: Option<&ItemStack>) -> Tag {
    Tag::List(
        stack
            .and_then(|s| s.to_nbt(host))
            .map(|c| vec![Tag::Compound(c)])
            .unwrap_or_default(),
    )
}

fn stored_on_client(host: &dyn Host, nbt: &Compound) -> Option<ItemStack> {
    match nbt.get(&INVENTORY.into()) {
        Some(Tag::List(l)) => l.iter().find_map(|t| match t {
            Tag::Compound(c) => ItemStack::from_nbt(host, c),
            _ => None,
        }),
        _ => None,
    }
}

pub struct Skillet;

#[mod_api::horizon_part]
impl BlockPart for Skillet {
    /// `SkilletBlock#updateShape`: water ticks while waterlogged; a vertical neighbour change
    /// recomputes `support` (a tray heat source below).
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
        if matches!(event.direction, Direction::Up | Direction::Down) {
            let support = level
                .block_state([pos[0], pos[1] - 1, pos[2]])
                .is_some_and(|s| level.block_in_tag(s.block(), TRAY_HEAT_SOURCES) == Some(true));
            return level.block_state_id(&set_prop(
                &name,
                "support",
                if support { "true" } else { "false" },
            ));
        }
        None
    }

    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        Some(interact(host, event.click, event.state))
    }

    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        Some(interact(host, event.click, event.state))
    }
}

/// `SkilletBlock#useItemOn`.
fn interact(
    host: &mut dyn ServerHost,
    click: &BlockClick<'_>,
    state: BlockState,
) -> InteractionResult {
    let Some(level) = host.level(click.dimension.key().id()) else {
        return InteractionResult::Pass;
    };
    let pos = click.pos;
    let Some(be) = host.block_entity(level, pos) else {
        return InteractionResult::TryWithEmptyHand;
    };
    let Some(player) = host.players().acting_player() else {
        return InteractionResult::Pass;
    };
    let Some(living) = host.living(player.entity()) else {
        return InteractionResult::Pass;
    };
    let creative = player.is_creative(&*host).unwrap_or(false);
    let stored = be.item_slot(&*host, 0).ok().flatten();
    let held = living.hand_item(&*host, click.hand);
    let Some(held) = held else {
        // Empty hand: take the whole stack into that hand.
        if host
            .transaction(|host, tx| {
                be.set_item_slot(host, tx, 0, None)
                    .map(|_| ())
                    .map_err(|_| ())
            })
            .is_ok()
        {
            mirror(host, be, None);
            if !creative {
                let _ = living.set_hand_item(host, click.hand, stored);
            }
        }
        return InteractionResult::Success;
    };
    if stored.is_some() {
        return InteractionResult::Consume;
    }
    let Some(recipe) = campfire(&*host, level, &held) else {
        let _ = player.action_bar(
            host,
            Text::translate("block.farmersdelight.skillet.invalid_item", vec![]),
        );
        return InteractionResult::Consume;
    };
    let name = host.block_state_string(state).unwrap_or_default();
    if state_value(&name, "waterlogged") == Some("true") {
        let _ = player.action_bar(
            host,
            Text::translate("block.farmersdelight.skillet.underwater", vec![]),
        );
        return InteractionResult::Consume;
    }
    // ItemStackHandler.insertItem: the whole stack fits (slot limit 64, at most the item's max).
    let data = be.data(&*host).unwrap_or_default();
    let total = skillet_cooking_time(recipe.processing_time as i32, fire_aspect_of(&*host, &data));
    if host
        .transaction(|host, tx| {
            be.set_item_slot(host, tx, 0, Some(held.clone()))
                .map(|_| ())
                .map_err(|_| ())
        })
        .is_err()
    {
        return InteractionResult::Consume;
    }
    let mut merge = BTreeMap::new();
    merge.insert(COOK_TIME_TOTAL.into(), Tag::Int(total));
    merge.insert(COOK_TIME.into(), Tag::Int(0));
    merge.insert(INVENTORY.into(), inventory_tag(&*host, Some(&held)));
    let _ = be.merge_data(host, merge);
    let at = [
        pos[0] as f64 + 0.5,
        pos[1] as f64 + 0.5,
        pos[2] as f64 + 0.5,
    ];
    if crate::heat::is_heated_in_level_ref(&*host, level, pos) {
        level.play_sound(host, SOUND_ADD_FOOD, at, SoundSource::Blocks, 0.8, 1.0);
    }
    if !creative {
        let _ = living.set_hand_item(host, click.hand, None);
    }
    level.play_sound(
        host,
        "minecraft:block.lantern.place",
        at,
        SoundSource::Blocks,
        0.7,
        1.0,
    );
    host.block_entity_sync(pos);
    InteractionResult::Success
}

fn mirror(host: &mut dyn ServerHost, be: BlockEntityRef, stack: Option<&ItemStack>) {
    let mut merge = BTreeMap::new();
    merge.insert(INVENTORY.into(), inventory_tag(&*host, stack));
    let _ = be.merge_data(host, merge);
    if let Some(pos) = be.position(&*host) {
        host.block_entity_sync(pos);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Skillet {
    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// `SkilletBlockEntity#cookingTick`.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let pos = event.pos;
        let entity = &mut event.entity;
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        let name = level
            .block_state(&*host, pos)
            .and_then(|s| host.block_state_string(s))
            .unwrap_or_default();
        let heated = crate::heat::is_heated_in_level_ref(&*host, level, pos);
        let total = int(&entity.nbt, COOK_TIME_TOTAL);
        let progress = int(&entity.nbt, COOK_TIME);
        if state_value(&name, "waterlogged") == Some("true") {
            if let Some(stack) = entity.items[0].take() {
                let _ = level.pop_resource(host, pos, stack);
                let tag = inventory_tag(&*host, None);
                entity.nbt.insert(INVENTORY.into(), tag);
                host.block_entity_sync(pos);
            }
        } else if heated {
            let Some(stack) = entity.items[0].clone() else {
                entity.nbt.insert(COOK_TIME.into(), Tag::Int(0));
                return;
            };
            let (next, done) = heated_step(progress, total);
            if done {
                // The reference keeps counting when no recipe matches any more (no reset, no output).
                let Some(recipe) = campfire(&*host, level, &stack) else {
                    entity.nbt.insert(COOK_TIME.into(), Tag::Int(progress + 1));
                    return;
                };
                let facing = state_value(&name, "facing").unwrap_or("north");
                let [dx, dz] = clockwise_step(facing);
                let _ = level.spawn_item_with_motion(
                    host,
                    pos[0] as f64 + 0.5,
                    pos[1] as f64 + 0.3,
                    pos[2] as f64 + 0.5,
                    recipe.output,
                    [dx * 0.08, 0.25, dz * 0.08],
                    0,
                );
                entity.items[0] = stack.shrink(1);
                let tag = inventory_tag(&*host, entity.items[0].as_ref());
                entity.nbt.insert(INVENTORY.into(), tag);
                host.block_entity_sync(pos);
            }
            entity.nbt.insert(COOK_TIME.into(), Tag::Int(next));
        } else if progress > 0 {
            entity
                .nbt
                .insert(COOK_TIME.into(), Tag::Int(cool_step(progress, total)));
        }
    }

    fn block_entity_update_tag(
        &self,
        event: &mut BlockEntityUpdateTagEvent<'_>,
    ) -> Option<Compound> {
        // Full update tag, as SyncedBlockEntity: the client draws the stack.
        Some(event.entity.nbt.clone())
    }

    fn block_entity_submit(&self, host: &dyn ClientHost, event: &mut BlockEntitySubmitEvent<'_>) {
        let Some(stack) = stored_on_client(host, &event.entity.nbt) else {
            return;
        };
        let name = host.block_state_string(event.state).unwrap_or_default();
        let facing = state_value(&name, "facing").unwrap_or("north");
        let max = host.max_stack_size(stack.item()).unwrap_or(64);
        let layers = crate::client::render::model_count(stack.count(), max);
        crate::client::render::draw_skillet(
            event.draw,
            &stack,
            facing,
            layers,
            stack.item().id(host).len() as i32,
        );
    }
}

// `farmersdelight:copy_skillet` (loot function) would copy the placed stack from the block
// entity, which Horizon's loot context does not carry; the copy happens in `loot_drops`, where
// the block entity still stands, and the function itself keeps the rolled stack.

/// The block's drop is the skillet it was placed with (enchantments, damage, name).
pub fn on_loot_drops(host: &mut dyn ServerHost, event: &mut LootDropsEvent<'_>) {
    if event.context.table != "farmersdelight:blocks/skillet" {
        return;
    }
    let Some((pos, _)) = event.context.block else {
        return;
    };
    let Some(level) = host.level(event.context.dimension) else {
        return;
    };
    let Some(data) = host.block_entity(level, pos).and_then(|be| be.data(&*host)) else {
        return;
    };
    let Some(Tag::Compound(c)) = data.get(&SKILLET.into()) else {
        return;
    };
    let Some(placed) = ItemStack::from_nbt(&*host, c) else {
        return;
    };
    for drop in event.drops.iter_mut() {
        if drop.item().id(&*host) == crate::items::skillet::ITEM {
            *drop = placed.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps() {
        assert_eq!(heated_step(118, 120), (119, false));
        assert_eq!(heated_step(119, 120), (0, true));
        assert_eq!(cool_step(5, 120), 3);
        assert_eq!(cool_step(1, 120), 0);
        assert_eq!(cool_step(0, 120), 0);
    }

    #[test]
    fn results_go_clockwise() {
        assert_eq!(clockwise_step("north"), [1.0, 0.0]);
        assert_eq!(clockwise_step("west"), [0.0, -1.0]);
    }
}
