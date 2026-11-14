//! Stove (J/common/block/{AbstractStoveBlock,StoveBlock}.java,
//! J/common/block/entity/{AbstractStoveBlockEntity,StoveBlockEntity}.java).
//!
//! Six one-item slots cooked with `minecraft:campfire_cooking` recipes while lit; unlit, progress
//! decays by 2 per tick. NBT keeps the reference's swapped names (Q4): `CookingTimes` holds each
//! slot's total, `CookingTotalTimes` its progress. No XP (Q12). Placed lit (Q7).
use super::cutting_board::state_value;
use mod_api::*;
use std::collections::BTreeMap;

pub const ID: &str = "farmersdelight:stove";
pub const SLOTS: usize = 6;
/// Q4: totals live under `CookingTimes`.
pub const TOTALS: &str = "CookingTimes";
/// Q4: progress lives under `CookingTotalTimes`.
pub const PROGRESS: &str = "CookingTotalTimes";
pub const INVENTORY: &str = "Inventory";
const CAMPFIRE: &str = "minecraft:campfire_cooking";
/// FD burns with `farmersdelight:stove_burn`, but Horizon at the pin loads no mod damage types
/// (the damage catalog takes no mod pack, E-15). `minecraft:hot_floor` is in the same tags
/// (`is_fire`, `burn_from_stepping`); only the death message differs (accepted delta A11).
const BURN_DAMAGE: &str = "minecraft:hot_floor";
/// `AbstractStoveBlock.GRILLING_AREA` in the block above, in blocks.
const GRILLING_AREA: [f64; 6] = [
    3.0 / 16.0,
    0.0,
    3.0 / 16.0,
    13.0 / 16.0,
    1.0 / 16.0,
    13.0 / 16.0,
];

// ---------------------------------------------------------------- pure rules

/// One lit tick (`cookAndOutputItems`): progress +1 for each occupied slot; returns the slots
/// whose progress reached their total.
pub fn cook_step(
    occupied: [bool; SLOTS],
    totals: &[i32; SLOTS],
    progress: &mut [i32; SLOTS],
) -> Vec<usize> {
    let mut done = Vec::new();
    for i in 0..SLOTS {
        if !occupied[i] {
            continue;
        }
        progress[i] += 1;
        if progress[i] >= totals[i] {
            done.push(i);
        }
    }
    done
}

/// One unlit tick (`coolItems`): progress −2, clamped to `[0, total]`. Returns whether anything changed.
pub fn cool_step(totals: &[i32; SLOTS], progress: &mut [i32; SLOTS]) -> bool {
    let mut changed = false;
    for i in 0..SLOTS {
        if progress[i] <= 0 {
            continue;
        }
        changed = true;
        progress[i] = (progress[i] - 2).clamp(0, totals[i].max(0));
    }
    changed
}

/// Whether any box of the block above overlaps the grilling area (`isStoveTopCovered`).
pub fn covers_grill(above_boxes: &[[f64; 6]]) -> bool {
    let g = GRILLING_AREA;
    above_boxes.iter().any(|b| {
        b[0] < g[3] && b[3] > g[0] && b[1] < g[4] && b[4] > g[1] && b[2] < g[5] && b[5] > g[2]
    })
}

pub fn read_ints(nbt: &Compound, key: &str) -> [i32; SLOTS] {
    let mut out = [0; SLOTS];
    if let Some(Tag::IntArray(values)) = nbt.get(&key.into()) {
        for (o, v) in out.iter_mut().zip(values.iter()) {
            *o = *v;
        }
    }
    out
}

pub fn write_ints(nbt: &mut Compound, key: &str, values: &[i32; SLOTS]) {
    nbt.insert(key.into(), Tag::IntArray(values.to_vec()));
}

// ---------------------------------------------------------------- the Part

pub struct Stove {
    pub smoke: ParticleKey,
    pub flame: ParticleKey,
    /// Client tick counter, a seed for effect randomness.
    pub client_ticks: std::sync::atomic::AtomicU64,
}

fn level_of(host: &dyn Host, dimension: &str) -> Option<LevelRef> {
    host.level(dimension)
}

fn lit_of(name: &str) -> bool {
    state_value(name, "lit") == Some("true")
}

#[mod_api::horizon_part]
impl BlockPart for Stove {
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        // facing comes from the template's own placement (stonecutter: opposite the placer).
        let facing = super::cutting_board::facing_for_yaw(event.context.yaw);
        let state = level.state_with_property(event.state, "facing", facing)?;
        Some(Some(level.state_with_property(state, "lit", "true")?))
    }

    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        Some(self.interact(host, event.click, event.state))
    }

    /// The template's own empty-hand use (the stone cutter's menu) never runs.
    fn use_without_item(
        &self,
        _: &mut dyn ServerHost,
        _: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        Some(InteractionResult::Pass)
    }
}

impl Stove {
    fn interact(
        &self,
        host: &mut dyn ServerHost,
        click: &BlockClick<'_>,
        state: BlockState,
    ) -> InteractionResult {
        let Some(level) = level_of(&*host, click.dimension.key().id()) else {
            return InteractionResult::Pass;
        };
        let Some(stack) = click.stack.cloned() else {
            return InteractionResult::TryWithEmptyHand;
        };
        let name = host.block_state_string(state).unwrap_or_default();
        let result = if lit_of(&name) {
            self.try_extinguish(host, level, click, &stack)
        } else {
            self.try_ignite(host, level, click, &stack)
        };
        if result != InteractionResult::TryWithEmptyHand {
            return result;
        }
        self.try_place_food(host, level, click, stack)
    }

    /// `AbstractStoveBlock#ignite`/`#extinguish`: `setBlock(lit, 11)` and a block-change game event.
    fn set_lit(
        &self,
        host: &mut dyn ServerHost,
        level: LevelRef,
        pos: [i32; 3],
        lit: bool,
        player: Option<EntityRef>,
    ) {
        host.set_block_property_with(pos, "lit", if lit { "true" } else { "false" }, 11);
        let affected_state = level.block_state(&*host, pos);
        let _ = level.game_event(
            host,
            "minecraft:block_change",
            centre(pos),
            GameEventContext {
                source_entity: player,
                affected_state,
            },
        );
        host.block_entity_sync(pos);
    }

    fn try_ignite(
        &self,
        host: &mut dyn ServerHost,
        level: LevelRef,
        click: &BlockClick<'_>,
        stack: &ItemStack,
    ) -> InteractionResult {
        let id = stack.item().id(&*host);
        let player = host.players().acting_player();
        let pos = click.pos;
        let at = centre(pos);
        let pitch_rand = level.random(&*host).map_or(0.5, |mut r| r.next_float());
        match id.as_str() {
            "minecraft:flint_and_steel" => {
                level.play_sound(
                    host,
                    "minecraft:item.flintandsteel.use",
                    at,
                    SoundSource::Blocks,
                    1.0,
                    pitch_rand * 0.4 + 0.8,
                );
                self.set_lit(host, level, pos, true, player.map(PlayerRef::entity));
                if let Some(living) = player.and_then(|p| host.living(p.entity())) {
                    let _ = living.hurt_and_break(host, slot_of(click.hand), 1);
                }
                InteractionResult::Success
            }
            "minecraft:fire_charge" => {
                level.play_sound(
                    host,
                    "minecraft:item.firecharge.use",
                    at,
                    SoundSource::Blocks,
                    1.0,
                    (pitch_rand - 0.5) * 0.2 + 1.0,
                );
                self.set_lit(host, level, pos, true, player.map(PlayerRef::entity));
                shrink_hand(host, player, click.hand, stack);
                InteractionResult::Success
            }
            _ => InteractionResult::TryWithEmptyHand,
        }
    }

    fn try_extinguish(
        &self,
        host: &mut dyn ServerHost,
        level: LevelRef,
        click: &BlockClick<'_>,
        stack: &ItemStack,
    ) -> InteractionResult {
        let player = host.players().acting_player();
        let pos = click.pos;
        // ItemAbility.SHOVEL_DIG → #minecraft:shovels
        if in_item_tag(&*host, stack, "minecraft:shovels") {
            // LevelEvent.SOUND_EXTINGUISH_FIRE
            let _ = level.level_event(host, 1009, pos, 0);
            self.set_lit(host, level, pos, false, player.map(PlayerRef::entity));
            if let Some(living) = player.and_then(|p| host.living(p.entity())) {
                let _ = living.hurt_and_break(host, slot_of(click.hand), 1);
            }
            return InteractionResult::Success;
        }
        if in_item_tag(&*host, stack, "c:buckets/water") {
            level.play_sound(
                host,
                "minecraft:block.fire.extinguish",
                centre(pos),
                SoundSource::Blocks,
                1.0,
                1.0,
            );
            self.set_lit(host, level, pos, false, player.map(PlayerRef::entity));
            let creative = player.and_then(|p| p.is_creative(&*host)).unwrap_or(false);
            if !creative
                && let Some(living) = player.and_then(|p| host.living(p.entity()))
                && let Some(bucket) = host
                    .item_key(&Identifier::parse("minecraft:bucket").expect("id"))
                    .and_then(|k| ItemStack::new(k, 1))
            {
                let _ = living.set_hand_item(host, click.hand, Some(bucket));
            }
            return InteractionResult::Success;
        }
        InteractionResult::TryWithEmptyHand
    }

    fn try_place_food(
        &self,
        host: &mut dyn ServerHost,
        level: LevelRef,
        click: &BlockClick<'_>,
        stack: ItemStack,
    ) -> InteractionResult {
        let pos = click.pos;
        if top_covered(&*host, level, pos) {
            return InteractionResult::TryWithEmptyHand;
        }
        let Some(be) = host.block_entity(level, pos) else {
            return InteractionResult::TryWithEmptyHand;
        };
        let Some(total) = campfire_recipe(&*host, level, &stack).map(|m| m.processing_time as i32)
        else {
            return InteractionResult::TryWithEmptyHand;
        };
        let slots: Vec<Option<ItemStack>> = (0..SLOTS as u32)
            .map(|i| be.item_slot(&*host, i).ok().flatten())
            .collect();
        let Some(slot) = slots.iter().position(Option::is_none) else {
            return InteractionResult::Consume;
        };
        let Some(one) = stack.clone().split(1).taken else {
            return InteractionResult::Consume;
        };
        let data = be.data(&*host).unwrap_or_default();
        let mut totals = read_ints(&data, TOTALS);
        let mut progress = read_ints(&data, PROGRESS);
        totals[slot] = total;
        progress[slot] = 0;
        if host
            .transaction(|host, tx| {
                be.set_item_slot(host, tx, slot as u32, Some(one.clone()))
                    .map(|_| ())
                    .map_err(|_| ())
            })
            .is_err()
        {
            return InteractionResult::Consume;
        }
        let mut next = slots;
        next[slot] = Some(one);
        let mut merge = BTreeMap::new();
        write_ints(&mut merge, TOTALS, &totals);
        write_ints(&mut merge, PROGRESS, &progress);
        merge.insert(INVENTORY.into(), inventory_tag(&*host, &next));
        let _ = be.merge_data(host, merge);
        let player = host.players().acting_player();
        let creative = player.and_then(|p| p.is_creative(&*host)).unwrap_or(false);
        if !creative {
            shrink_hand(host, player, click.hand, &stack);
        }
        level.play_sound(
            host,
            "minecraft:block.lantern.place",
            centre(pos),
            SoundSource::Blocks,
            0.5,
            1.0,
        );
        let _ = level.game_event(
            host,
            "minecraft:block_change",
            centre(pos),
            GameEventContext {
                source_entity: player.map(PlayerRef::entity),
                affected_state: None,
            },
        );
        host.block_entity_sync(pos);
        InteractionResult::SuccessServer
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Stove {
    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// `AbstractStoveBlockEntity#serverTick`.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let pos = event.pos;
        let entity = &mut event.entity;
        let Some(level) = level_of(&*host, host.dimension()) else {
            return;
        };
        let lit = level
            .block_state(&*host, pos)
            .and_then(|s| host.block_state_string(s))
            .is_some_and(|n| lit_of(&n));
        if lit {
            burn_standing(host, level, pos);
        }
        if entity.items.iter().all(Option::is_none) {
            return;
        }
        if top_covered(&*host, level, pos) {
            for slot in entity.items.iter_mut() {
                if let Some(stack) = slot.take() {
                    let _ = level.pop_resource(host, pos, stack);
                }
            }
            let tag = inventory_tag(&*host, &entity.items);
            entity.nbt.insert(INVENTORY.into(), tag);
            host.block_entity_sync(pos);
            return;
        }
        let totals = read_ints(&entity.nbt, TOTALS);
        let mut progress = read_ints(&entity.nbt, PROGRESS);
        if lit {
            let occupied: [bool; SLOTS] =
                std::array::from_fn(|i| entity.items.get(i).is_some_and(Option::is_some));
            let done = cook_step(occupied, &totals, &mut progress);
            for i in &done {
                let Some(ingredient) = entity.items[*i].clone() else {
                    continue;
                };
                let result =
                    campfire_recipe(&*host, level, &ingredient).map_or(ingredient, |m| m.output);
                let (gx, gz) = level
                    .random(&*host)
                    .map_or((0.0, 0.0), |mut r| (r.next_gaussian(), r.next_gaussian()));
                let _ = level.spawn_item_with_motion(
                    host,
                    pos[0] as f64 + 0.5,
                    pos[1] as f64 + 1.0,
                    pos[2] as f64 + 0.5,
                    result,
                    [gx * 0.01, 0.1, gz * 0.01],
                    0,
                );
                entity.items[*i] = None;
            }
            write_ints(&mut entity.nbt, PROGRESS, &progress);
            if !done.is_empty() {
                let tag = inventory_tag(&*host, &entity.items);
                entity.nbt.insert(INVENTORY.into(), tag);
                host.block_entity_sync(pos);
            }
        } else if cool_step(&totals, &mut progress) {
            write_ints(&mut entity.nbt, PROGRESS, &progress);
        }
    }

    fn block_entity_client_ticks(
        &self,
        _: &dyn ClientHost,
        event: &mut BlockEntityClientTicksEvent<'_>,
    ) {
        event.ticks = true;
    }

    /// `StoveBlock#animateTick` (crackle 1 in 10, smoke and flame at the front) and
    /// `StoveBlockEntity#addSmokeParticles` (each cooking item steams 20% of ticks, 3 puffs); lit only.
    fn block_entity_client_tick(
        &self,
        host: &mut dyn ClientHost,
        event: &mut BlockEntityClientTickEvent<'_>,
    ) {
        let name = host.block_state_string(event.state).unwrap_or_default();
        if !lit_of(&name) {
            return;
        }
        let pos = event.pos;
        let tick = self
            .client_ticks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut rng = ClientRng::new(pos, tick as i64);
        let (x, y, z) = (pos[0] as f64 + 0.5, pos[1] as f64, pos[2] as f64 + 0.5);
        if rng.next() < 0.1 {
            let mut sound = SoundPlay::ui("farmersdelight:block.stove.crackle");
            sound.source = SoundSource::Blocks;
            sound.position = [x, y, z];
            sound.relative = false;
            sound.attenuation = SoundAttenuation::Linear;
            let _ = host.play_local_sound(&sound, 0);
        }
        let facing = state_value(&name, "facing").unwrap_or("north");
        let [sx, sz] = step(facing);
        let horizontal = rng.next() * 0.6 - 0.3;
        let ox = if sx != 0.0 { sx * 0.52 } else { horizontal };
        let oz = if sz != 0.0 { sz * 0.52 } else { horizontal };
        let oy = rng.next() * 6.0 / 16.0;
        let at = [x + ox, y + oy, z + oz];
        let _ = host.add_particle(&ParticleOptions::simple(self.smoke), at, [0.0; 3]);
        let _ = host.add_particle(&ParticleOptions::simple(self.flame), at, [0.0; 3]);
        let stacks = inventory_from_tag(&*host, &event.entity.nbt);
        let [cx, cz] = step(clockwise(facing));
        for (i, stack) in stacks.iter().enumerate() {
            if stack.is_none() || rng.next() >= 0.2 {
                continue;
            }
            let [mut a, mut b] = crate::client::render::STOVE_OFFSETS[i].map(f64::from);
            if matches!(facing, "west" | "east") {
                std::mem::swap(&mut a, &mut b);
            }
            let px = x - sx * a + cx * a;
            let pz = z - sz * b + cz * b;
            for _ in 0..3 {
                let _ = host.add_particle(
                    &ParticleOptions::simple(self.smoke),
                    [px, y + 1.0, pz],
                    [0.0, 5.0e-4, 0.0],
                );
            }
        }
    }

    fn block_entity_submit(&self, host: &dyn ClientHost, event: &mut BlockEntitySubmitEvent<'_>) {
        let stacks = inventory_from_tag(host, &event.entity.nbt);
        if stacks.iter().all(Option::is_none) {
            return;
        }
        let name = host.block_state_string(event.state).unwrap_or_default();
        let facing = state_value(&name, "facing").unwrap_or("north");
        crate::client::render::draw_stove(event.draw, &stacks, facing);
    }

    fn block_entity_update_tag(
        &self,
        event: &mut BlockEntityUpdateTagEvent<'_>,
    ) -> Option<Compound> {
        let mut tag = BTreeMap::new();
        if let Some(inv) = event.entity.nbt.get(&INVENTORY.into()) {
            tag.insert(INVENTORY.into(), inv.clone());
        }
        Some(tag)
    }
}

/// `AbstractStoveBlock#burnEntitySteppingOnStove`, run from the stove's tick: Horizon's
/// `block_step_on` gets no host to hurt with, so the lit stove looks for living entities on the
/// ground whose box meets the grilling area one block up and that are not stepping carefully
/// (sneaking). Same condition, checked every tick as `stepOn` is.
fn burn_standing(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    let g = GRILLING_AREA;
    let base = [pos[0] as f64, pos[1] as f64 + 1.0, pos[2] as f64];
    let min = [base[0] + g[0], base[1] + g[1], base[2] + g[2]];
    let max = [base[0] + g[3], base[1] + g[4], base[2] + g[5]];
    let Some(found) = level.entities_in(&*host, min, max) else {
        return;
    };
    for e in found {
        if host.living(e).is_none()
            || e.on_ground(&*host) != Some(true)
            || e.is_shift_key_down(&*host) == Some(true)
        {
            continue;
        }
        let _ = e.hurt(host, BURN_DAMAGE, 1.0, None, None);
    }
}

// ---------------------------------------------------------------- helpers

fn step(facing: &str) -> [f64; 2] {
    match facing {
        "north" => [0.0, -1.0],
        "south" => [0.0, 1.0],
        "west" => [-1.0, 0.0],
        _ => [1.0, 0.0],
    }
}

fn clockwise(facing: &str) -> &'static str {
    match facing {
        "north" => "east",
        "east" => "south",
        "south" => "west",
        _ => "north",
    }
}

/// Client-only effect randomness (no gameplay depends on it).
pub(crate) struct ClientRng(u64);
impl ClientRng {
    pub(crate) fn new(pos: [i32; 3], time: i64) -> Self {
        let seed = (pos[0] as i64).wrapping_mul(3_129_871)
            ^ (pos[2] as i64).wrapping_mul(116_129_781)
            ^ (pos[1] as i64)
            ^ time.wrapping_mul(0x9E37_79B9);
        Self(seed as u64 | 1)
    }
    pub(crate) fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn centre(pos: [i32; 3]) -> [f64; 3] {
    [
        pos[0] as f64 + 0.5,
        pos[1] as f64 + 0.5,
        pos[2] as f64 + 0.5,
    ]
}

fn slot_of(hand: Hand) -> EquipmentSlot {
    match hand {
        Hand::Main => EquipmentSlot::MainHand,
        Hand::Off => EquipmentSlot::OffHand,
    }
}

fn shrink_hand(
    host: &mut dyn ServerHost,
    player: Option<PlayerRef>,
    hand: Hand,
    stack: &ItemStack,
) {
    let creative = player.and_then(|p| p.is_creative(&*host)).unwrap_or(false);
    if creative {
        return;
    }
    if let Some(living) = player.and_then(|p| host.living(p.entity())) {
        let _ = living.set_hand_item(host, hand, stack.clone().shrink(1));
    }
}

pub fn in_item_tag(host: &dyn Host, stack: &ItemStack, tag: &str) -> bool {
    let tags = host.tags();
    Identifier::parse(tag)
        .ok()
        .and_then(|id| tags.key(&id))
        .and_then(|key| tags.is_in_tag(stack.item(), key))
        == Some(true)
}

fn campfire_recipe(host: &dyn Host, level: LevelRef, stack: &ItemStack) -> Option<RecipeMatch> {
    let kind = Identifier::parse(CAMPFIRE).ok()?;
    level
        .recipe_for(host, &kind, &RecipeInput::Single(stack.clone()))
        .ok()
        .flatten()
}

fn top_covered(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> bool {
    let above = [pos[0], pos[1] + 1, pos[2]];
    level
        .block_state(host, above)
        .and_then(|s| level.block_shape(host, s, above, BlockShapeKind::Outline))
        .is_some_and(|boxes| covers_grill(&boxes))
}

/// The client's copy of the six slots: a list of stack compounds with a `Slot` byte.
pub fn inventory_tag(host: &dyn Host, items: &[Option<ItemStack>]) -> Tag {
    let list = items
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let mut c = s.as_ref()?.to_nbt(host)?;
            c.insert("Slot".into(), Tag::Byte(i as i8));
            Some(Tag::Compound(c))
        })
        .collect();
    Tag::List(list)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooking_finishes_at_total() {
        let totals = [600, 600, 0, 0, 0, 0];
        let mut progress = [598, 0, 0, 0, 0, 0];
        let occ = [true, true, false, false, false, false];
        assert!(cook_step(occ, &totals, &mut progress).is_empty());
        assert_eq!(cook_step(occ, &totals, &mut progress), vec![0]);
        assert_eq!(progress, [600, 2, 0, 0, 0, 0]);
    }

    #[test]
    fn cooling_decays_by_two_and_clamps() {
        let totals = [600, 600, 0, 0, 0, 0];
        let mut progress = [5, 1, 0, 0, 0, 0];
        assert!(cool_step(&totals, &mut progress));
        assert_eq!(progress, [3, 0, 0, 0, 0, 0]);
        let mut idle = [0; SLOTS];
        assert!(!cool_step(&totals, &mut idle));
    }

    #[test]
    fn q4_swapped_key_names() {
        let mut nbt = BTreeMap::new();
        write_ints(&mut nbt, TOTALS, &[600, 0, 0, 0, 0, 0]);
        write_ints(&mut nbt, PROGRESS, &[12, 0, 0, 0, 0, 0]);
        assert_eq!(TOTALS, "CookingTimes");
        assert_eq!(read_ints(&nbt, "CookingTimes")[0], 600);
        assert_eq!(read_ints(&nbt, "CookingTotalTimes")[0], 12);
    }

    #[test]
    fn grill_cover() {
        assert!(covers_grill(&[[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]])); // bottom slab
        assert!(!covers_grill(&[[0.0, 0.5, 0.0, 1.0, 1.0, 1.0]])); // top slab
        assert!(!covers_grill(&[])); // air
        assert!(!covers_grill(&[[0.0, 0.0, 0.0, 0.125, 1.0, 1.0]])); // thin wall at the edge
    }
}
