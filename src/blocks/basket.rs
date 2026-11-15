//! Baskets (J/common/block/BasketBlock.java, J/common/block/entity/BasketBlockEntity.java,
//! J/common/block/entity/Basket.java; PORT_PLAN M7-04).
//!
//! A 27-slot container shown in the vanilla 9×3 chest menu, open to automation on every face
//! (BasketInvWrapper over the whole container). Every tick off cooldown, an enabled basket that is
//! not full pulls the item entities in the box in front of its opening, one whole entity per
//! transfer, then waits 8 ticks; it never pushes (Q15). A redstone signal clears `enabled`.
use crate::blocks::cooking_pot::comparator;
use crate::blocks::cutting_board::state_value;
use mod_api::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const WOODEN: &str = "farmersdelight:wooden_basket";
pub const BAMBOO: &str = "farmersdelight:bamboo_basket";
pub const SLOTS: usize = 27;
pub const MENU: &str = "minecraft:generic_9x3";
const COOLDOWN: &str = "TransferCooldown";
/// `tryTransfer`: the cooldown after a successful collection.
const TRANSFER_COOLDOWN: i32 = 8;

// ---------------------------------------------------------------- pure rules

/// `Direction.orderedByNearest(entity)[0]` for a view of `yaw`/`pitch` degrees (Minecraft angles:
/// yaw 0 looks south, pitch 90 looks down).
pub fn nearest_looking(yaw: f32, pitch: f32) -> &'static str {
    let f = (pitch as f64).to_radians();
    let g = -(yaw as f64).to_radians();
    let (h, i, j, k) = (f.sin(), f.cos(), g.sin(), g.cos());
    let east = j > 0.0;
    let up = h < 0.0;
    let south = k > 0.0;
    let l = if east { j } else { -j };
    let m = if up { -h } else { h };
    let n = if south { k } else { -k };
    let o = l * i;
    let p = n * i;
    let horizontal_x = if east { "east" } else { "west" };
    let vertical = if up { "up" } else { "down" };
    let horizontal_z = if south { "south" } else { "north" };
    if l > n {
        if m > o { vertical } else { horizontal_x }
    } else if m > p {
        vertical
    } else {
        horizontal_z
    }
}

pub fn opposite(direction: &str) -> &'static str {
    match direction {
        "up" => "down",
        "down" => "up",
        "north" => "south",
        "south" => "north",
        "west" => "east",
        _ => "west",
    }
}

/// `Basket.COLLECTION_AREA_SHAPES` for a facing, in blocks relative to the basket's corner.
pub fn collection_area(facing: &str) -> ([f64; 3], [f64; 3]) {
    match facing {
        "down" => ([0.0, -1.0, 0.0], [1.0, 1.0, 1.0]),
        "north" => ([0.0, 0.0, -1.0], [1.0, 1.0, 1.0]),
        "south" => ([0.0, 0.0, 0.0], [1.0, 1.0, 2.0]),
        "west" => ([-1.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        "east" => ([0.0, 0.0, 0.0], [2.0, 1.0, 1.0]),
        _ => ([0.0, 0.0, 0.0], [1.0, 2.0, 1.0]),
    }
}

/// One slot as `Basket#insert(slot, stack)` sees it: (count, max stack, merges with the stack).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Empty,
    Holds { count: u32, max: u32, same: bool },
}

/// `Basket#insert(stack)`: how many of a `count`-stack (max stack `max`) go into each slot, in
/// slot order. An empty slot takes the whole rest; a slot with the same item and components
/// takes up to the stack's max stack size.
pub fn insert_plan(slots: &[Slot], mut count: u32, max: u32) -> Vec<(usize, u32)> {
    let mut plan = Vec::new();
    for (i, slot) in slots.iter().enumerate() {
        if count == 0 {
            break;
        }
        let take = match *slot {
            Slot::Empty => count,
            Slot::Holds {
                count: have,
                max: slot_max,
                same: true,
            } if have <= slot_max => count.min(max.saturating_sub(have)),
            _ => 0,
        };
        if take > 0 {
            plan.push((i, take));
            count -= take;
        }
    }
    plan
}

/// `BasketBlockEntity#isFull`: every slot holds a full stack.
pub fn is_full(slots: &[Slot]) -> bool {
    slots
        .iter()
        .all(|s| matches!(*s, Slot::Holds { count, max, .. } if count == max))
}

/// `pushItemsTick` up to the transfer: the cooldown after the decrement, and whether to try.
pub fn tick_cooldown(saved: i32) -> (i32, bool) {
    let cooldown = saved - 1;
    if cooldown > 0 {
        (cooldown, false)
    } else {
        (0, true)
    }
}

fn int(nbt: &Compound, key: &str) -> Option<i32> {
    match nbt.get(&key.into()) {
        Some(Tag::Int(v)) => Some(*v),
        _ => None,
    }
}

// ---------------------------------------------------------------- the Parts

/// The comparator signal of each basket, as its last tick computed it.
pub type Signals = Arc<Mutex<HashMap<[i32; 3], i32>>>;

pub struct Basket {
    pub signals: Signals,
}

impl Basket {
    fn signals(&self) -> std::sync::MutexGuard<'_, HashMap<[i32; 3], i32>> {
        self.signals.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[mod_api::horizon_part]
impl BlockPart for Basket {
    /// `getStateForPlacement`: opening toward the placer (opposite the nearest looking
    /// direction), waterlogged in water.
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let facing = opposite(nearest_looking(event.context.yaw, event.context.pitch));
        let water = crate::blocks::rope::is_water(level, event.fluid);
        let state = level.state_with_property(event.state, "facing", facing)?;
        Some(level.state_with_property(state, "waterlogged", if water { "true" } else { "false" }))
    }

    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let name = level.state_string(event.state).unwrap_or_default();
        if state_value(&name, "waterlogged") == Some("true") {
            level.schedule_fluid_tick(
                [event.pos.x, event.pos.y, event.pos.z],
                "minecraft:water",
                5,
            );
        }
        None
    }

    /// Water stays where a waterlogged basket is removed (E-13, as the cutting board).
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

    /// `useWithoutItem`: open the menu (CONSUME on the server).
    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        let click = event.click;
        if let Some(player) = host.players().acting_player()
            && let Some(level) = host.level(click.dimension.key().id())
            && let Some(be) = host.block_entity(level, click.pos)
        {
            let _ = player.open_menu(host, be);
        }
        Some(InteractionResult::Consume)
    }

    fn has_analog_output(&self, _: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }

    fn analog_output(&self, _: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        event.output = self.signals().get(&pos).copied().unwrap_or(0);
    }

    fn is_pathfindable(&self, _: &dyn Host, event: &mut IsPathfindableEvent) {
        event.pathfindable = false;
    }

    /// FD registers no fire odds for baskets (the oak-planks template's are dropped).
    fn fire_odds(&self, _: &dyn LevelHost, _: &mut FireOddsEvent) -> Option<(i32, i32)> {
        Some((0, 0))
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Basket {
    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// `BasketBlockEntity#pushItemsTick` and `tryTransfer`.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let pos = event.pos;
        let entity = &mut event.entity;
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        if entity.items.len() < SLOTS {
            entity.items.resize(SLOTS, None);
        }
        let saved = int(&entity.nbt, COOLDOWN).unwrap_or(-1);
        let (mut cooldown, try_now) = tick_cooldown(saved);
        let state = level.block_state(&*host, pos);
        let name = state
            .and_then(|s| host.block_state_string(s))
            .unwrap_or_default();
        // BasketBlock#neighborChanged: `enabled` is "no neighbour signal", written with flag 4 (no
        // client update, as FD; no model reads it). The neighbour hook's host cannot write, so the
        // tick reads the signal before its transfer, as the block update would have landed first.
        let enabled = !level.has_neighbor_signal(&*host, pos);
        if state_value(&name, "enabled").is_some_and(|v| (v == "true") != enabled) {
            host.set_block_property_with(pos, "enabled", if enabled { "true" } else { "false" }, 4);
        }
        if try_now && enabled {
            let slots = self.slots_of(&*host, &entity.items, None);
            if !is_full(&slots) && self.collect(host, level, pos, &name, &mut entity.items) {
                cooldown = TRANSFER_COOLDOWN;
            }
        }
        if int(&entity.nbt, COOLDOWN) != Some(cooldown) {
            entity.nbt.insert(COOLDOWN.into(), Tag::Int(cooldown));
        }
        // Comparator: AbstractContainerMenu.getRedstoneSignalFromContainer.
        let counts: Vec<_> = entity
            .items
            .iter()
            .map(|s| {
                s.as_ref()
                    .map(|s| (s.count(), host.max_stack_size(s.item()).unwrap_or(64)))
            })
            .collect();
        let signal = comparator(&counts);
        if self.signals().insert(pos, signal) != Some(signal)
            && let Some(state) = state
        {
            let _ = level.update_neighbors(host, pos, state, NeighbourUpdate::OutputSignal);
        }
    }

    fn pre_remove_side_effects(
        &self,
        _: &mut dyn LevelRandomHost,
        event: &mut PreRemoveSideEffectsEvent<'_>,
    ) {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        self.signals().remove(&pos);
    }
}

impl Basket {
    fn slots_of(
        &self,
        host: &dyn Host,
        items: &[Option<ItemStack>],
        incoming: Option<&ItemStack>,
    ) -> Vec<Slot> {
        items
            .iter()
            .map(|s| match s {
                None => Slot::Empty,
                Some(s) => Slot::Holds {
                    count: s.count(),
                    max: host.max_stack_size(s.item()).unwrap_or(64),
                    same: incoming.is_some_and(|i| i.is_stackable_with(s)),
                },
            })
            .collect()
    }

    /// `Basket#collectItems`: true once one item entity went in whole.
    fn collect(
        &self,
        host: &mut dyn ServerHost,
        level: LevelRef,
        pos: [i32; 3],
        state_name: &str,
        items: &mut [Option<ItemStack>],
    ) -> bool {
        let facing = state_value(state_name, "facing").unwrap_or("up");
        let (min, max) = collection_area(facing);
        let at = |v: [f64; 3]| {
            [
                pos[0] as f64 + v[0],
                pos[1] as f64 + v[1],
                pos[2] as f64 + v[2],
            ]
        };
        let Some(found) = level.entities_in(&*host, at(min), at(max)) else {
            return false;
        };
        for entity in found {
            let Some(item) = host.item_entity(entity) else {
                continue;
            };
            let Some(stack) = item.stack(&*host) else {
                continue;
            };
            let max = host.max_stack_size(stack.item()).unwrap_or(64);
            let slots = self.slots_of(&*host, items, Some(&stack));
            let plan = insert_plan(&slots, stack.count(), max);
            let moved: u32 = plan.iter().map(|(_, n)| n).sum();
            if moved == 0 {
                continue;
            }
            for (slot, n) in plan {
                let mut part = stack.clone();
                let new = match &items[slot] {
                    None => part.set_count(n).ok().map(|_| part),
                    Some(have) => {
                        let mut have = have.clone();
                        have.set_count(have.count() + n).ok().map(|_| have)
                    }
                };
                if let Some(new) = new {
                    items[slot] = Some(new);
                }
            }
            if moved == stack.count() {
                let _ = entity.discard(host);
                return true;
            }
            if let Some(rest) = stack.shrink(moved) {
                let _ = item.set_stack(host, rest);
            }
        }
        false
    }
}

/// The client's prediction of a basket click (E-28): `useWithoutItem` answers SUCCESS there for the
/// main hand (so a held block is not placed against the basket); the off hand passes.
pub fn predict(host: HostSide<'_>, event: &mut UseItemOnBlockEvent<'_>) -> Flow {
    let HostSide::Client(host) = host else {
        return Flow::Continue;
    };
    if event.phase != UsePhase::Block {
        return Flow::Continue;
    }
    let host: &dyn ClientHost = &*host;
    let Some(level) = host.level(host.dimension()) else {
        return Flow::Continue;
    };
    let Some(state) = level.block_state(host, event.click.pos) else {
        return Flow::Continue;
    };
    let name = host.block_state_string(state).unwrap_or_default();
    let id = name.split('[').next().unwrap_or(&name);
    if id != WOODEN && id != BAMBOO {
        return Flow::Continue;
    }
    *event.cancellation_result = if event.click.hand == Hand::Main {
        InteractionResult::Success
    } else {
        InteractionResult::Pass
    };
    Flow::Cancel
}

pub fn register(registrar: &mut dyn Registrar, signals: &Signals) {
    let ty = registrar
        .block_entity_type(WOODEN)
        .blocks(&[WOODEN, BAMBOO])
        .build();
    registrar
        .block_entity_part(
            &ty,
            Basket {
                signals: signals.clone(),
            },
        )
        .expect("basket block entity type");
}

pub fn container() -> ModContainer {
    ModContainer {
        slots: SLOTS,
        faces: std::array::from_fn(|_| (0..SLOTS).collect()),
        inputs: (0..SLOTS).collect(),
        outputs: (0..SLOTS).collect(),
        menu: Some(MENU.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_faces_the_placer() {
        // Looking straight down places the basket open upward; looking north, open south.
        assert_eq!(opposite(nearest_looking(0.0, 90.0)), "up");
        assert_eq!(opposite(nearest_looking(180.0, 10.0)), "south");
        assert_eq!(opposite(nearest_looking(90.0, 0.0)), "east"); // looking west
        assert_eq!(opposite(nearest_looking(0.0, -60.0)), "down");
    }

    #[test]
    fn insert_fills_slot_order_and_merges_up_to_max() {
        let same = |count| Slot::Holds {
            count,
            max: 64,
            same: true,
        };
        let other = Slot::Holds {
            count: 3,
            max: 64,
            same: false,
        };
        // A partial stack of the same item first, then the first empty slot takes the rest.
        let slots = [other, same(60), Slot::Empty, Slot::Empty];
        assert_eq!(insert_plan(&slots, 10, 64), vec![(1, 4), (2, 6)]);
        // An empty slot before a matching one takes everything.
        assert_eq!(insert_plan(&[Slot::Empty, same(1)], 5, 64), vec![(0, 5)]);
        assert!(insert_plan(&[other], 5, 64).is_empty());
    }

    #[test]
    fn cooldown_and_full() {
        assert_eq!(tick_cooldown(-1), (0, true));
        assert_eq!(tick_cooldown(8), (7, false));
        assert_eq!(tick_cooldown(1), (0, true));
        let full = Slot::Holds {
            count: 16,
            max: 16,
            same: false,
        };
        assert!(is_full(&[full, full]));
        assert!(!is_full(&[full, Slot::Empty]));
    }
}
