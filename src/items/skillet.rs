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

#[mod_api::horizon_part]
impl ItemPart for SkilletItem {
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
    if player.is_creative(&*host) != Some(true)
        && let Some(living) = host.living(player.entity())
    {
        let _ = living.set_hand_item(host, event.hand, event.stack.clone().shrink(1));
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

#[cfg(test)]
mod tests {
    use super::*;

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
