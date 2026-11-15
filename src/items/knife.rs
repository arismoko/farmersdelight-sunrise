//! Knives (`KnifeItem`). The six items are data declarations (`tools/decl/items.toml.in`); this
//! module holds their code-side behaviour.
use mod_api::*;
use std::sync::OnceLock;

/// FD's six `KnifeItem`s (`ModItems.java`), in registration order.
pub const KNIVES: [&str; 6] = [
    "farmersdelight:flint_knife",
    "farmersdelight:copper_knife",
    "farmersdelight:iron_knife",
    "farmersdelight:diamond_knife",
    "farmersdelight:netherite_knife",
    "farmersdelight:golden_knife",
];

/// `KnifeEvents#onKnifeKnockback`: `strength - 0.1F`.
pub const KNOCKBACK_CUT: f64 = 0.1f32 as f64;

/// `ModTags.Items.KNIVES`.
const KNIVES_TAG: &str = "farmersdelight:tools/knives";
/// `ModTags.Blocks.DROPS_CAKE_SLICE` (the 17 candle cakes).
const DROPS_CAKE_SLICE: &str = "farmersdelight:drops_cake_slice";
const CAKE: &str = "minecraft:cake";
const PUMPKIN: &str = "minecraft:pumpkin";
const CAKE_SLICE: &str = "farmersdelight:cake_slice";

static KEYS: OnceLock<[ItemKey; 6]> = OnceLock::new();

pub fn register(registrar: &mut dyn Registrar) {
    let keys = KNIVES.map(|id| registrar.item_key(id));
    for key in keys {
        registrar.item_part(key, Knife);
    }
    let _ = KEYS.set(keys);
}

fn block_id(name: &str) -> &str {
    name.split('[').next().unwrap_or(name)
}

fn stack_of(host: &dyn Host, id: &str, count: u32) -> Option<ItemStack> {
    ItemStack::new(host.item_key(&Identifier::parse(id).ok()?)?, count)
}

fn slot_of(hand: Hand) -> EquipmentSlot {
    match hand {
        Hand::Main => EquipmentSlot::MainHand,
        Hand::Off => EquipmentSlot::OffHand,
    }
}

/// `Direction.getStepX/Z` of a horizontal direction name.
fn step(direction: &str) -> [i32; 2] {
    match direction {
        "north" => [0, -1],
        "south" => [0, 1],
        "west" => [-1, 0],
        _ => [1, 0],
    }
}

fn opposite(direction: &str) -> &'static str {
    match direction {
        "north" => "south",
        "south" => "north",
        "west" => "east",
        _ => "west",
    }
}

/// The carved pumpkin's facing: the clicked side, or (top/bottom) toward the player.
pub fn carve_facing(face: [i32; 3], player_facing: &str) -> &'static str {
    match face {
        [0, 0, -1] => "north",
        [0, 0, 1] => "south",
        [-1, 0, 0] => "west",
        [1, 0, 0] => "east",
        _ => opposite(player_facing),
    }
}

/// What a knife does to a cake with `bites` (0-6): the next bite, or None (removed).
pub fn cake_after_cut(bites: u32) -> Option<u32> {
    (bites < 6).then_some(bites + 1)
}

// ---------------------------------------------------------------- pumpkin carving (KnifeItem#useOn)

pub struct Knife;

#[mod_api::horizon_part]
impl ItemPart for Knife {
    fn use_on(&self, host: &mut dyn ServerHost, event: &mut UseOnEvent<'_>) -> Flow {
        let Some(player) = host.players().acting_player() else {
            return Flow::Continue;
        };
        let Some(level) = player.level(&*host) else {
            return Flow::Continue;
        };
        let pos = event.pos;
        let Some(state) = level.block_state(&*host, pos) else {
            return Flow::Continue;
        };
        let name = host.block_state_string(state).unwrap_or_default();
        if block_id(&name) != PUMPKIN
            || !crate::blocks::stove::in_item_tag(&*host, event.stack, KNIVES_TAG)
        {
            return Flow::Continue;
        }
        let looking = player.entity().direction(&*host).unwrap_or("north");
        let facing = carve_facing(event.face, looking);
        let centre = [
            pos[0] as f64 + 0.5,
            pos[1] as f64 + 0.5,
            pos[2] as f64 + 0.5,
        ];
        level.play_sound(
            host,
            "minecraft:block.pumpkin.carve",
            centre,
            SoundSource::Blocks,
            1.0,
            1.0,
        );
        if let Some(carved) =
            host.block_state_id(&format!("minecraft:carved_pumpkin[facing={facing}]"))
        {
            let _ = level.set_block(
                host,
                pos,
                carved,
                BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
            );
        }
        let [sx, sz] = step(facing);
        let (r1, r2) = level
            .random(&*host)
            .map_or((0.5, 0.5), |mut r| (r.next_double(), r.next_double()));
        if let Some(seeds) = stack_of(&*host, "minecraft:pumpkin_seeds", 4) {
            let _ = level.spawn_item_with_motion(
                host,
                pos[0] as f64 + 0.5 + sx as f64 * 0.65,
                pos[1] as f64 + 0.1,
                pos[2] as f64 + 0.5 + sz as f64 * 0.65,
                seeds,
                [
                    0.05 * sx as f64 + r1 * 0.02,
                    0.05,
                    0.05 * sz as f64 + r2 * 0.02,
                ],
                0,
            );
        }
        if let Some(living) = host.living(player.entity()) {
            let _ = living.hurt_and_break(host, slot_of(event.hand), 1);
        }
        Flow::Cancel
    }
}

// ---------------------------------------------------------------- cakes (KnifeEvents#onCakeInteraction)

enum CakeCut {
    Candle,
    Cake(u32),
}

fn cake_cut(host: &dyn Host, state: BlockState) -> Option<CakeCut> {
    if host.block_in_tag(state.block(), DROPS_CAKE_SLICE) == Some(true) {
        return Some(CakeCut::Candle);
    }
    let name = host.block_state_string(state)?;
    if block_id(&name) != CAKE {
        return None;
    }
    let bites = crate::blocks::cutting_board::state_value(&name, "bites")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    Some(CakeCut::Cake(bites))
}

/// `UseBlockCallback` (before the block's and the item's use, sneaking too): a knife (tag) on a
/// cake cuts a slice; on a candle cake it drops the candle and leaves a cake with one bite taken.
/// Both sides decide the same way; only the server changes the world.
pub fn on_right_click(mut host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
    let click = event.click;
    let Some(stack) = click.stack else {
        return Flow::Continue;
    };
    let decided = match host.reborrow() {
        HostSide::Client(h) => {
            let h: &dyn ClientHost = &*h;
            let Some(player) = h.local_player() else {
                return Flow::Continue;
            };
            if player.is_spectator(h) == Some(true)
                || !crate::blocks::stove::in_item_tag(h, stack, KNIVES_TAG)
            {
                return Flow::Continue;
            }
            h.level(h.dimension())
                .and_then(|l| l.block_state(h, click.pos))
                .and_then(|s| cake_cut(h, s))
        }
        HostSide::Server(h) => {
            let Some(player) = h.players().acting_player() else {
                return Flow::Continue;
            };
            if player.is_spectator(&*h) == Some(true)
                || !crate::blocks::stove::in_item_tag(&*h, stack, KNIVES_TAG)
            {
                return Flow::Continue;
            }
            let Some(level) = h.level(click.dimension.key().id()) else {
                return Flow::Continue;
            };
            let Some(state) = level.block_state(&*h, click.pos) else {
                return Flow::Continue;
            };
            let cut = cake_cut(&*h, state);
            if let Some(cut) = &cut {
                cut_cake(h, level, click.pos, cut);
            }
            cut
        }
    };
    let Some(cut) = decided else {
        return Flow::Continue;
    };
    let server = matches!(host, HostSide::Server(_));
    *event.cancellation_result = match cut {
        CakeCut::Cake(_) if server => InteractionResult::Consume,
        _ => InteractionResult::Success,
    };
    Flow::Cancel
}

fn cut_cake(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], cut: &CakeCut) {
    let flags = BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS;
    let x;
    match *cut {
        CakeCut::Candle => {
            // Block.dropResources(state): the candle cake's own loot (its candle), no tool or entity.
            let drops = level
                .block_drops(host, pos, None, None)
                .ok()
                .flatten()
                .unwrap_or_default();
            if let Some(cake) = host.block_state_id("minecraft:cake[bites=1]") {
                let _ = level.set_block(host, pos, cake, flags);
            }
            for drop in drops {
                let _ = level.pop_resource(host, pos, drop);
            }
            x = pos[0] as f64;
        }
        CakeCut::Cake(bites) => {
            match cake_after_cut(bites) {
                Some(next) => {
                    if let Some(cake) =
                        host.block_state_id(&format!("minecraft:cake[bites={next}]"))
                    {
                        let _ = level.set_block(host, pos, cake, flags);
                    }
                }
                None => {
                    if let Some(air) = host.block_state_id("minecraft:air") {
                        let _ = level.set_block(host, pos, air, flags);
                    }
                }
            }
            x = pos[0] as f64 + bites as f64 * 0.1;
        }
    }
    if let Some(slice) = stack_of(&*host, CAKE_SLICE, 1) {
        let _ = level.spawn_item_with_motion(
            host,
            x,
            pos[1] as f64 + 0.2,
            pos[2] as f64 + 0.5,
            slice,
            [-0.05, 0.0, 0.0],
            0,
        );
    }
    let centre = [
        pos[0] as f64 + 0.5,
        pos[1] as f64 + 0.5,
        pos[2] as f64 + 0.5,
    ];
    level.play_sound(
        host,
        "minecraft:block.wool.break",
        centre,
        SoundSource::Players,
        0.8,
        0.8,
    );
}

/// The client's answer for a knife on a pumpkin (`KnifeItem#useOn` returns SUCCESS there); the
/// stick template would predict a pass (E-28).
pub fn predict_use(host: HostSide<'_>, event: &mut UseItemOnBlockEvent<'_>) -> Flow {
    let HostSide::Client(host) = host else {
        return Flow::Continue;
    };
    let host: &dyn ClientHost = &*host;
    let click = event.click;
    if event.phase != UsePhase::ItemAfterBlock {
        return Flow::Continue;
    }
    let Some(stack) = click.stack else {
        return Flow::Continue;
    };
    if !is_knife(stack) || !crate::blocks::stove::in_item_tag(host, stack, KNIVES_TAG) {
        return Flow::Continue;
    }
    let pumpkin = host
        .level(host.dimension())
        .and_then(|l| l.block_state(host, click.pos))
        .and_then(|s| host.block_state_string(s))
        .is_some_and(|n| block_id(&n) == PUMPKIN);
    if !pumpkin {
        return Flow::Continue;
    }
    *event.cancellation_result = InteractionResult::Success;
    Flow::Cancel
}

/// `toolStack.getItem() instanceof KnifeItem` (the six items, not the knives tag).
pub fn is_knife(stack: &ItemStack) -> bool {
    KEYS.get().is_some_and(|keys| keys.contains(&stack.item()))
}

/// `LivingEntityMixin#handleKnifeKnockback`: a knife in the attacker's main hand pushes 0.1 less.
/// FD reads the victim's kill credit; the engine names the hit's attacker (A39).
pub fn on_knockback(event: &mut LivingKnockbackEvent<'_>) {
    if event.attacker_main.is_some_and(is_knife) {
        event.strength = knocked(event.strength);
    }
}

fn knocked(strength: f64) -> f64 {
    // Java: `double strength = strength - 0.1F` (float widened to double).
    strength - KNOCKBACK_CUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carving_faces_the_clicked_side_or_the_player() {
        assert_eq!(carve_facing([0, 0, -1], "south"), "north");
        assert_eq!(carve_facing([1, 0, 0], "north"), "east");
        // top face: player looks north → the face points south, at the player.
        assert_eq!(carve_facing([0, 1, 0], "north"), "south");
        assert_eq!(carve_facing([0, -1, 0], "west"), "east");
    }

    #[test]
    fn a_cake_loses_a_bite_until_the_seventh_cut_removes_it() {
        assert_eq!(cake_after_cut(0), Some(1));
        assert_eq!(cake_after_cut(5), Some(6));
        assert_eq!(cake_after_cut(6), None);
    }

    #[test]
    fn knockback_loses_a_float_tenth() {
        assert_eq!(knocked(0.4), 0.4 - 0.10000000149011612);
    }
}
