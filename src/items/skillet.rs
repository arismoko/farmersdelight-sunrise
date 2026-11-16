//! Skillet item (J/common/item/SkilletItem.java) — M2-06: combat wear, attack sounds, sneak placement.
use mod_api::*;
use std::collections::BTreeMap;

pub const ITEM: &str = "farmersdelight:skillet";
pub const BLOCK: &str = "farmersdelight:skillet";
const SOUND_WEAK: &str = "farmersdelight:item.skillet.attack.weak";
const SOUND_STRONG: &str = "farmersdelight:item.skillet.attack.strong";
const TRAY_HEAT_SOURCES: &str = "farmersdelight:tray_heat_sources";

/// `SkilletEvents#playSkilletAttackSound`'s choice: a player's blow at or below 0.8 charge is weak.
pub fn attack_sound(attacker_is_player: bool, charge: f32) -> (&'static str, f32, Option<f32>) {
    if attacker_is_player && charge <= 0.8 {
        (SOUND_WEAK, 0.8, Some(0.9))
    } else {
        // pitch 0.9 + random * 0.2
        (SOUND_STRONG, 1.0, None)
    }
}

pub struct SkilletItem;

// ---------------------------------------------------------------- held cooking (M2-07)

pub const INGREDIENT: &str = "farmersdelight:skillet_ingredient";
pub const COOKING_TIME: &str = "farmersdelight:cooking_time_length";
static SIZZLE_TICKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub const FLIP_TIMESTAMP: &str = "farmersdelight:skillet_flip_timestamp";
pub const FLIPPED: &str = "farmersdelight:skillet_flipped";
/// The four FD data components the skillet carries (registered in `component_types`).
pub const COMPONENT_TYPES: [&str; 4] = [INGREDIENT, COOKING_TIME, FLIP_TIMESTAMP, FLIPPED];
const HEAT_SOURCES: &str = "farmersdelight:heat_sources";
/// `SkilletBlock.MINIMUM_COOKING_TIME`.
pub const MINIMUM_COOKING_TIME: i32 = 60;

/// `SkilletBlock#getSkilletCookingTime`: the recipe time (600 when absent) as whole seconds × 0.2,
/// less 0.05 per Fire Aspect level, back in ticks, clamped to `[60, original]`.
pub fn skillet_cooking_time(original: i32, fire_aspect: i32) -> i32 {
    let time = if original > 0 { original } else { 600 };
    let seconds = time / 20;
    let mut reduction = 0.2f32;
    if fire_aspect > 0 {
        reduction -= fire_aspect as f32 * 0.05;
    }
    let result = (seconds as f32 * reduction) as i32 * 20;
    // Mth.clamp(value, min, max) = value < min ? min : min(value, max)
    if result < MINIMUM_COOKING_TIME {
        MINIMUM_COOKING_TIME
    } else {
        result.min(original)
    }
}

pub(crate) fn patch_of(stack: &ItemStack) -> Compound {
    stack
        .components()
        .map(|c| c.as_nbt().clone())
        .unwrap_or_default()
}

fn ingredient_of(host: &dyn Host, stack: &ItemStack) -> Option<ItemStack> {
    match patch_of(stack).get(&INGREDIENT.into()) {
        Some(Tag::Compound(c)) => ItemStack::from_nbt(host, c),
        _ => None,
    }
}

fn cooking_time_of(stack: &ItemStack) -> Option<i32> {
    match patch_of(stack).get(&COOKING_TIME.into()) {
        Some(Tag::Int(t)) => Some(*t),
        _ => None,
    }
}

/// The skillet with FD's cooking components set (`Some`) or all four removed (`None`).
fn with_cooking(
    host: &dyn Host,
    skillet: &ItemStack,
    cooking: Option<(&ItemStack, i32)>,
) -> ItemStack {
    let mut patch = patch_of(skillet);
    for key in COMPONENT_TYPES {
        patch.remove(&key.into());
    }
    if let Some((unit, time)) = cooking
        && let Some(nbt) = unit.to_nbt(host)
    {
        patch.insert(INGREDIENT.into(), Tag::Compound(nbt));
        patch.insert(COOKING_TIME.into(), Tag::Int(time));
    }
    let mut out = skillet.clone();
    let _ = out.set_components(patch);
    out
}

fn fire_aspect(stack: &ItemStack) -> i32 {
    match patch_of(stack).get(&"minecraft:enchantments".into()) {
        Some(Tag::Compound(levels)) => match levels.get(&"minecraft:fire_aspect".into()) {
            Some(Tag::Int(l)) => *l,
            _ => 0,
        },
        _ => 0,
    }
}

/// `SkilletItem#isPlayerNearHeatSource`: on fire, or any heat source (lit or not, as the reference
/// reads only the tag) in the 3×3×3 around the player's block.
fn near_heat(host: &dyn Host, player: PlayerRef) -> bool {
    let e = player.entity();
    if e.fire_ticks(host).is_some_and(|t| t > 0) {
        return true;
    }
    let (Some(level), Some(pos)) = (player.level(host), e.block_position(host)) else {
        return false;
    };
    (-1..=1).any(|dx| {
        (-1..=1).any(|dy| {
            (-1..=1).any(|dz| {
                level
                    .block_state(host, [pos[0] + dx, pos[1] + dy, pos[2] + dz])
                    .is_some_and(|s| host.block_in_tag(s.block(), HEAT_SOURCES) == Some(true))
            })
        })
    })
}

fn under_water(host: &dyn Host, player: PlayerRef) -> bool {
    let e = player.entity();
    let (Some(level), Some(eye)) = (player.level(host), e.eye_position(host)) else {
        return false;
    };
    let at = [
        eye[0].floor() as i32,
        eye[1].floor() as i32,
        eye[2].floor() as i32,
    ];
    let water = host
        .block_state_id("minecraft:water")
        .and_then(|w| host.state_fluid_state(w));
    level
        .fluid_state(host, at)
        .zip(water)
        .is_some_and(|(f, w)| f.fluid == w.fluid)
}

fn campfire_result(host: &dyn Host, level: LevelRef, stack: &ItemStack) -> Option<RecipeMatch> {
    let kind = Identifier::parse("minecraft:campfire_cooking").ok()?;
    level
        .recipe_for(host, &kind, &RecipeInput::Single(stack.clone()))
        .ok()
        .flatten()
}

fn other(hand: Hand) -> Hand {
    match hand {
        Hand::Main => Hand::Off,
        Hand::Off => Hand::Main,
    }
}

/// The duration a bare skillet answers so a click starts a use; `on_use_start` replaces it.
const USE_PROBE: i32 = 72_000;

/// `SkilletItem#use`, at the use's start on both sides (`LivingEntity.startUsingItem`): near a heat
/// source, with an ingredient already in the skillet or a campfire-cookable stack in the other hand
/// (not under water), the use runs for `getSkilletCookingTime`; otherwise it is cancelled. The
/// server stores one unit in the skillet and leaves the rest in the other hand.
pub fn on_use_start(host: HostSide<'_>, event: &mut LivingEntityUseItemStartEvent<'_>) -> Flow {
    if event.entity.is_some() {
        return Flow::Continue;
    }
    let (read, player): (&dyn Host, Option<PlayerRef>) = match &host {
        HostSide::Server(h) => (&**h, h.players().acting_player()),
        HostSide::Client(h) => (&**h, h.local_player()),
    };
    if event.stack.item().id(read) != ITEM {
        return Flow::Continue;
    }
    let Some(player) = player else {
        return Flow::Continue;
    };
    let fa = fire_aspect(event.stack);
    if !near_heat(read, player) {
        event.cancelled = true;
        return Flow::Continue;
    }
    if ingredient_of(read, event.stack).is_some()
        && let Some(time) = cooking_time_of(event.stack)
    {
        event.duration = skillet_cooking_time(time, fa);
        return Flow::Continue;
    }
    let hand = event.hand;
    let cooking = read
        .living(player.entity())
        .and_then(|l| l.hand_item(read, other(hand)));
    let recipe = player
        .level(read)
        .zip(cooking.as_ref())
        .and_then(|(level, c)| campfire_result(read, level, c));
    let underwater = under_water(read, player);
    let (Some(cooking), Some(recipe)) = (cooking, recipe) else {
        event.cancelled = true;
        if let HostSide::Server(h) = host {
            let _ = player.action_bar(
                h,
                Text::translate("item.farmersdelight.skillet.how_to_cook", vec![]),
            );
        }
        return Flow::Continue;
    };
    if underwater {
        event.cancelled = true;
        if let HostSide::Server(h) = host {
            let _ = player.action_bar(
                h,
                Text::translate("item.farmersdelight.skillet.underwater", vec![]),
            );
        }
        return Flow::Continue;
    }
    let time = recipe.processing_time as i32;
    event.duration = skillet_cooking_time(time, fa);
    if let HostSide::Server(h) = host
        && let Some(living) = h.living(player.entity())
    {
        let split = cooking.split(1);
        if let Some(unit) = split.taken {
            let next = with_cooking(&*h, event.stack, Some((&unit, time)));
            let _ = living.set_hand_item(h, hand, Some(next));
            // The reference sets the other hand to the copy the unit was split from (count − 1).
            let _ = living.set_hand_item(h, other(hand), split.remainder);
        }
    }
    Flow::Continue
}

#[mod_api::horizon_part]
impl ItemPart for SkilletItem {
    /// `SkilletItem#getUseDuration`. Always above 0 so a right click starts a use on both sides
    /// (the engine then treats the skillet as used over time); `on_use_start` sets the real
    /// time or cancels.
    fn use_duration(&self, _: &dyn Host, event: &mut UseDurationEvent<'_>) {
        event.duration = match cooking_time_of(event.stack) {
            Some(time) => skillet_cooking_time(time, fire_aspect(event.stack)),
            None => USE_PROBE,
        };
    }

    /// `SkilletItem#onUseTick` (client, no flip in progress): a sizzle 1 tick in 50 while cooking held food.
    fn on_use_tick(&self, host: HostSide<'_>, event: &mut OnUseTickEvent<'_>) {
        let HostSide::Client(host) = host else {
            return;
        };
        if !is_cooking(event.stack) {
            return;
        }
        let tick = SIZZLE_TICKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut rng = crate::blocks::stove::ClientRng::new([event.remaining, 0, 0], tick as i64);
        if rng.next() >= 0.02 {
            return;
        }
        let Some(at) = host
            .local_player()
            .and_then(|p| p.entity().position(&*host))
        else {
            return;
        };
        crate::blocks::skillet::sizzle(&*host, at, SoundSource::Players, rng.next());
    }

    /// `SkilletItem#releaseUsing`: an early release gives the ingredient back.
    fn release_using(&self, host: HostSide<'_>, event: &mut ReleaseUsingEvent<'_>) {
        let HostSide::Server(host) = host else { return };
        let Some(ingredient) = ingredient_of(&*host, event.stack) else {
            return;
        };
        let Some(player) = host.players().acting_player() else {
            return;
        };
        let Some(living) = host.living(player.entity()) else {
            return;
        };
        let hand = living.used_item_hand(&*host).unwrap_or(Hand::Main);
        let cleared = with_cooking(&*host, event.stack, None);
        let _ = living.set_hand_item(host, hand, Some(cleared));
        let _ = host.transaction(|host, tx| {
            crate::inventory::give_or_drop(host, tx, player, ingredient.clone());
            Ok::<_, ()>(())
        });
    }

    /// The bar shows cooking progress in `0xFF8B4F` (`getBarWidth`/`getBarColor`).
    fn item_bar(&self, host: &dyn ClientHost, event: &mut ItemBarEvent<'_>) {
        let Some(time) = cooking_time_of(event.stack) else {
            return;
        };
        let total = skillet_cooking_time(time, fire_aspect(event.stack)).max(1);
        let remaining = host
            .local_player()
            .and_then(|p| host.living(p.entity()))
            .and_then(|l| l.item_use_remaining(host))
            .unwrap_or(total);
        let width = (13.0 - (total + remaining) as f32 * 13.0 / total as f32 + 13.0)
            .round()
            .clamp(0.0, 13.0) as i32;
        *event.bar = Some((width, 0xFF8B4F));
    }

    /// `SkilletItem#postHurtEnemy`: 1 more wear per hit, on top of `Weapon(1)` (reference: 2 per hit, Q17).
    fn hurt_enemy(&self, host: &mut dyn ServerHost, _: &mut HurtEnemyEvent<'_>) {
        if let Some(living) = host
            .players()
            .acting_player()
            .and_then(|p| host.living(p.entity()))
        {
            let _ = living.hurt_and_break(host, EquipmentSlot::MainHand, 1);
        }
    }

    /// `SkilletItem#place`: the skillet goes down as a block only while sneaking. Otherwise the
    /// click is left alone so the item's own `use` (held cooking) runs.
    fn use_on(&self, host: &mut dyn ServerHost, event: &mut UseOnEvent<'_>) -> Flow {
        let Some(player) = host.players().acting_player() else {
            return Flow::Continue;
        };
        if player.entity().is_shift_key_down(&*host) != Some(true) {
            return Flow::Continue;
        }
        place(host, player, event);
        Flow::Cancel
    }
}

fn place(host: &mut dyn ServerHost, player: PlayerRef, event: &UseOnEvent<'_>) {
    let Some(level) = player.level(&*host) else {
        return;
    };
    let clicked = event.pos;
    let replaceable = |host: &dyn Host, pos: [i32; 3]| {
        // BlockState#canBeReplaced for placement: air or #minecraft:replaceable (water included).
        level.block_state(host, pos).is_some_and(|s| {
            host.block_state_string(s).is_some_and(|n| {
                n.starts_with("minecraft:air") || n.starts_with("minecraft:cave_air")
            }) || host.block_in_tag(s.block(), "minecraft:replaceable") == Some(true)
        })
    };
    let pos = if replaceable(&*host, clicked) {
        clicked
    } else {
        [
            clicked[0] + event.face[0],
            clicked[1] + event.face[1],
            clicked[2] + event.face[2],
        ]
    };
    if !replaceable(&*host, pos) {
        return;
    }
    let Some(state) = placement_state(host, level, player, pos) else {
        return;
    };
    if level.set_block(
        host,
        pos,
        state,
        BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
    ) != Ok(true)
    {
        return;
    }
    // SkilletBlockEntity keeps the placed stack (enchantments, damage) for the drop (M2-09).
    if let Some(be) = host.block_entity(level, pos)
        && let Some(nbt) = event.stack.to_nbt(&*host)
    {
        let mut data = BTreeMap::new();
        data.insert("Skillet".into(), Tag::Compound(nbt));
        let _ = be.merge_data(host, data);
    }
    let at = [
        pos[0] as f64 + 0.5,
        pos[1] as f64 + 0.5,
        pos[2] as f64 + 0.5,
    ];
    // BlockItem#place: the block's place sound at (volume + 1) / 2, pitch × 0.8 (lantern: 1.0, 1.0).
    level.play_sound(
        host,
        "minecraft:block.lantern.place",
        at,
        SoundSource::Blocks,
        1.0,
        0.8,
    );
    // BlockItem#place fires `minecraft:placed_block`; a mod cannot fire a `minecraft:` trigger
    // (E-33), so FD's own criterion for it is granted directly (A44).
    grant_criterion(host, player, "farmersdelight:main/place_skillet", "skillet");
    if player.is_creative(&*host) != Some(true)
        && let Some(living) = host.living(player.entity())
    {
        let _ = living.set_hand_item(host, event.hand, event.stack.clone().shrink(1));
    }
}

/// Grants one criterion of one of FD's advancements to `player` (the console's
/// `advancement grant … only`), for vanilla triggers a mod cannot fire (E-33, E-34; A44).
pub fn grant_criterion(
    host: &mut dyn ServerHost,
    player: PlayerRef,
    advancement: &str,
    criterion: &str,
) {
    if let Some(name) = player.name(&*host) {
        let _ = host.commands().run_command(&format!(
            "advancement grant {name} only {advancement} {criterion}"
        ));
    }
}

/// `SkilletBlock#getStateForPlacement`: facing the way the player looks, waterlogged in water,
/// `support` over a tray heat source.
fn placement_state(
    host: &dyn Host,
    level: LevelRef,
    player: PlayerRef,
    pos: [i32; 3],
) -> Option<BlockState> {
    let base = host.block_state_id(BLOCK)?;
    let facing = player.entity().direction(host).unwrap_or("north");
    let water = level.fluid_state(host, pos).is_some_and(|f| {
        f.source
            && host
                .block_state_id("minecraft:water")
                .and_then(|w| host.state_fluid_state(w))
                .is_some_and(|w| w.fluid == f.fluid)
    });
    let below = level.block_state(host, [pos[0], pos[1] - 1, pos[2]]);
    let support =
        below.is_some_and(|s| host.block_in_tag(s.block(), TRAY_HEAT_SOURCES) == Some(true));
    let with = |s: BlockState, k: &str, v: &str| {
        host.block_state_id(&set_prop(&host.block_state_string(s)?, k, v))
    };
    let s = with(base, "facing", facing)?;
    let s = with(s, "waterlogged", if water { "true" } else { "false" })?;
    with(s, "support", if support { "true" } else { "false" })
}

/// Replace one property in a state string `ns:block[a=b,c=d]`.
pub fn set_prop(state: &str, key: &str, value: &str) -> String {
    let Some((name, rest)) = state.split_once('[') else {
        return format!("{state}[{key}={value}]");
    };
    let props: Vec<String> = rest
        .trim_end_matches(']')
        .split(',')
        .map(|kv| match kv.split_once('=') {
            Some((k, _)) if k == key => format!("{k}={value}"),
            _ => kv.to_owned(),
        })
        .collect();
    format!("{name}[{}]", props.join(","))
}

/// `PlayerMixin` → `playSkilletAttackSound`: only when the *victim* is a player (Q2), the
/// attacker holds a skillet in the main hand; weak at ≤ 0.8 charge, else strong.
pub fn on_attack(host: &mut dyn ServerHost, event: &AttackEntityEvent<'_>) {
    if event.kind != "minecraft:player" {
        return;
    }
    let Some(attacker) = host.players().acting_player() else {
        return;
    };
    let Some(living) = host.living(attacker.entity()) else {
        return;
    };
    let Some(held) = living.hand_item(&*host, Hand::Main) else {
        return;
    };
    if held.item().id(&*host) != ITEM {
        return;
    }
    let charge = attacker.attack_strength_scale(&*host, 0.5).unwrap_or(1.0);
    let Some(level) = attacker.level(&*host) else {
        return;
    };
    let (sound, volume, pitch) = attack_sound(true, charge);
    let pitch = pitch
        .unwrap_or_else(|| 0.9 + level.random(&*host).map_or(0.5, |mut r| r.next_float()) * 0.2);
    let at = attacker.entity().position(&*host).unwrap_or(event.pos);
    level.play_sound(host, sound, at, SoundSource::Players, volume, pitch);
}

/// `SkilletItem#finishUsingItem`: the cooked result to the inventory (or dropped), one more of the
/// other hand used up, the components cleared.
pub fn on_finish(host: HostSide<'_>, event: &mut LivingEntityUseItemFinishEvent<'_>) {
    let HostSide::Server(host) = host else { return };
    if event.stack.item().id(&*host) != ITEM {
        return;
    }
    let Some(player) = host.players().acting_player() else {
        return;
    };
    // ServerPlayer#completeUsingItem fires `minecraft:consume_item` for any finished use; the
    // engine fires it for consumables only and a mod cannot fire a `minecraft:` trigger (E-34),
    // so FD's own criterion for the skillet is granted directly (A44).
    grant_criterion(host, player, "farmersdelight:main/use_skillet", "skillet");
    let Some(ingredient) = ingredient_of(&*host, event.stack) else {
        return;
    };
    let Some(level) = player.level(&*host) else {
        return;
    };
    if let Some(recipe) = campfire_result(&*host, level, &ingredient) {
        let result = recipe.output;
        let _ = host.transaction(|host, tx| {
            crate::inventory::give_or_drop(host, tx, player, result.clone());
            Ok::<_, ()>(())
        });
    }
    if let Some(living) = host.living(player.entity()) {
        let other_hand = other(event.hand);
        let rest = living
            .hand_item(&*host, other_hand)
            .and_then(|s| s.shrink(1));
        let _ = living.set_hand_item(host, other_hand, rest);
    }
    *event.result = Some(with_cooking(&*host, event.stack, None));
}

/// The mod's item-model node type for the cooking skillet (see the carry override of
/// `items/skillet.json`).
pub const FLIP_MODEL: &str = "ported_farmersdelight:skillet_flip";
pub const IS_COOKING: &str = "farmersdelight:skillet/is_cooking";

/// `SkilletCookingConditionalItemModelProperty`: the stack holds an ingredient.
pub fn is_cooking(stack: &ItemStack) -> bool {
    patch_of(stack).contains_key(&INGREDIENT.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooking_time_formula() {
        assert_eq!(skillet_cooking_time(600, 0), 120); // 30 s × 0.2
        assert_eq!(skillet_cooking_time(600, 1), 80); // × 0.15 → 4.5 s → 4 s
        assert_eq!(skillet_cooking_time(600, 2), 60); // × 0.10 → 3 s
        assert_eq!(skillet_cooking_time(600, 3), 60); // clamped to the minimum
        assert_eq!(skillet_cooking_time(0, 0), 0); // the max is the *original* time (0): no use
        assert_eq!(skillet_cooking_time(100, 0), 60);
        assert_eq!(skillet_cooking_time(200, 0), 60); // 10 s × 0.2 = 2 s = 40 → min 60
    }

    #[test]
    fn attack_sound_choice() {
        assert_eq!(attack_sound(true, 0.5).0, SOUND_WEAK);
        assert_eq!(attack_sound(true, 0.8).0, SOUND_WEAK);
        assert_eq!(attack_sound(true, 0.81).0, SOUND_STRONG);
        assert_eq!(attack_sound(false, 0.1).0, SOUND_STRONG);
    }

    #[test]
    fn state_strings() {
        assert_eq!(
            set_prop(
                "farmersdelight:skillet[facing=north,half=bottom,support=false]",
                "facing",
                "east"
            ),
            "farmersdelight:skillet[facing=east,half=bottom,support=false]"
        );
    }
}
