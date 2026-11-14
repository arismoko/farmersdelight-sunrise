//! Mob effects and consume effects (PORT_PLAN §6.1, M4-02…M4-04).
//!
//! - Nourishment (J/common/effect/NourishmentEffect.java): every tick on a player, unless they are
//!   healing from food (natural regeneration on, hurt, food ≥ 18), exhaustion drops by
//!   `min(exhaustion, 4.0)`. NourishmentAlwaysEatMixin (always edible while nourished) is not
//!   ported: `can_eat` gives no player at the pin (E-18, delta A16).
//!   The `naturalHealthRegeneration` game rule cannot be read at the build pin, so it is taken as
//!   its default `true` (accepted delta A15; E-01).
//! - Comfort (J/common/effect/ComfortEffect.java): every 80 ticks of remaining duration, heal 1.0
//!   unless the holder has Regeneration or (a player) saturation above 0.
//! - Consume effect types `farmersdelight:remove_random_effects`, `heal`, `extinguish`.
use mod_api::*;
use std::sync::OnceLock;

pub const NOURISHMENT: &str = "farmersdelight:nourishment";
pub const COMFORT: &str = "farmersdelight:comfort";
pub const REMOVE_RANDOM_EFFECTS: &str = "farmersdelight:remove_random_effects";
pub const HEAL: &str = "farmersdelight:heal";
pub const EXTINGUISH: &str = "farmersdelight:extinguish";

/// Every effect the mod refers to, requested at registration (keys by name are the registrar's).
pub const USED: [&str; 9] = [
    NOURISHMENT,
    COMFORT,
    "minecraft:regeneration",
    "minecraft:speed",
    "minecraft:strength",
    "minecraft:resistance",
    "minecraft:jump_boost",
    "minecraft:absorption",
    "minecraft:hunger",
];

static KEYS: OnceLock<std::collections::HashMap<&'static str, EffectKey>> = OnceLock::new();

pub fn register(registrar: &mut dyn Registrar) {
    let _ = KEYS.set(
        USED.iter()
            .map(|id| (*id, registrar.effect_key(id)))
            .collect(),
    );
}

/// The key of an effect in [`USED`].
pub fn key(id: &str) -> Option<EffectKey> {
    KEYS.get()?.get(id).copied()
}

/// `NourishmentEffect#applyEffectTick`: the exhaustion to remove this tick (0 for none).
pub fn nourishment_reduction(exhaustion: f32, hurt: bool, food: i32, natural_regen: bool) -> f32 {
    let healing_with_hunger = natural_regen && hurt && food >= 18;
    if healing_with_hunger || exhaustion <= 0.0 {
        0.0
    } else {
        exhaustion.min(4.0)
    }
}

/// `ComfortEffect#applyEffectTick`: whether this tick heals 1.0.
pub fn comfort_heals(
    has_regeneration: bool,
    saturation: Option<f32>,
    health: f32,
    max: f32,
) -> bool {
    !has_regeneration && saturation.is_none_or(|s| s <= 0.0) && health < max
}

pub fn should_apply(_: &dyn Host, event: &mut ShouldApplyEffectTickThisTickEvent) {
    if Some(event.effect) == key(NOURISHMENT) {
        event.apply = true;
    } else if Some(event.effect) == key(COMFORT) {
        event.apply = event.tick_count % 80 == 0;
    }
}

pub fn apply_tick(host: &dyn Host, event: &mut ApplyEffectTickEvent<'_>) {
    if Some(event.effect) == key(NOURISHMENT) {
        let Some(food) = event.food else {
            return;
        };
        let hurt = event.health < event.max_health;
        let cut = nourishment_reduction(food.exhaustion, hurt, food.level, true);
        if cut > 0.0 {
            event.add_exhaustion(-cut);
        }
        event.keep = true;
    } else if Some(event.effect) == key(COMFORT) {
        let regen =
            key("minecraft:regeneration").is_some_and(|r| event.holder.effect(host, r).is_some());
        let saturation = event.food.map(|f| f.saturation);
        if comfort_heals(regen, saturation, event.health, event.max_health) {
            event.heal(1.0);
        }
        event.keep = true;
    }
}

/// The consume effect types FD registers (ModConsumeEffectTypes), for the acting player.
pub fn consume_effect(host: &mut dyn ServerHost, event: &mut ConsumeEffectEvent<'_>) {
    let Some(player) = host.players().acting_player() else {
        return;
    };
    let Some(living) = host.living(player.entity()) else {
        return;
    };
    match event.kind {
        HEAL => {
            let amount = match event.data {
                Tag::Compound(c) => c.get(&"amount".into()).and_then(number).unwrap_or(0.0),
                _ => 0.0,
            };
            let _ = living.heal(host, amount);
        }
        EXTINGUISH => {
            let _ = player.entity().extinguish_fire(host);
        }
        REMOVE_RANDOM_EFFECTS => {
            let harmful_only = matches!(
                event.data,
                Tag::Compound(c) if matches!(c.get(&"harmful_only".into()), Some(Tag::Byte(1)))
            );
            // The `excluded` tags (#farmersdelight:ignored/milk_bottle, hot_cocoa) are empty in FD
            // (Q13); mob-effect tags cannot be queried at the build pin, so none is excluded.
            let candidates: Vec<EffectKey> = living
                .effects(&*host)
                .unwrap_or_default()
                .into_iter()
                .filter(|e| !harmful_only || e.id.category(&*host) == Some(EffectCategory::Harmful))
                .map(|e| e.id)
                .collect();
            if candidates.is_empty() {
                return;
            }
            let roll = player
                .level(&*host)
                .and_then(|l| l.random(&*host))
                .and_then(|mut r| r.next_int_bound(candidates.len() as i32).ok())
                .unwrap_or(0);
            let _ = living.remove_effect(host, candidates[roll.max(0) as usize % candidates.len()]);
        }
        _ => {}
    }
}

/// A numeric tag as f32: data-driven payloads keep JSON numbers in the narrowest NBT type
/// (`2.0` arrives as a byte).
fn number(tag: &Tag) -> Option<f32> {
    Some(match tag {
        Tag::Byte(v) => *v as f32,
        Tag::Short(v) => *v as f32,
        Tag::Int(v) => *v as f32,
        Tag::Long(v) => *v as f32,
        Tag::Float(bits) => f32::from_bits(*bits),
        Tag::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_of_any_width() {
        assert_eq!(number(&Tag::Byte(2)), Some(2.0));
        assert_eq!(number(&Tag::Float(2.5f32.to_bits())), Some(2.5));
        assert_eq!(number(&Tag::Double(0.5f64.to_bits())), Some(0.5));
        assert_eq!(number(&Tag::String("2".into())), None);
    }

    #[test]
    fn nourishment_cuts_up_to_four() {
        assert_eq!(nourishment_reduction(2.5, false, 20, true), 2.5);
        assert_eq!(nourishment_reduction(6.0, false, 20, true), 4.0);
        assert_eq!(nourishment_reduction(0.0, false, 20, true), 0.0);
        // healing from a full bar: exhaustion is left to drain into regeneration
        assert_eq!(nourishment_reduction(3.0, true, 18, true), 0.0);
        assert_eq!(nourishment_reduction(3.0, true, 17, true), 3.0);
        assert_eq!(nourishment_reduction(3.0, true, 20, false), 3.0);
    }

    #[test]
    fn comfort_heals_only_without_regen_or_saturation() {
        assert!(comfort_heals(false, Some(0.0), 10.0, 20.0));
        assert!(!comfort_heals(true, Some(0.0), 10.0, 20.0));
        assert!(!comfort_heals(false, Some(0.5), 10.0, 20.0));
        assert!(!comfort_heals(false, None, 20.0, 20.0));
        assert!(comfort_heals(false, None, 5.0, 8.0)); // a mob has no saturation
    }
}
