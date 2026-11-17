//! Backstabbing (J/common/item/enchantment/BackstabbingEnchantment.java, applied from
//! `LivingEntityMixin#handleBackstabbingDamage`). The enchantment itself is carried data
//! (`data/farmersdelight/enchantment/backstabbing.json`); its `farmersdelight:backstabbing`
//! effect is evaluated here with the shipped values (A41).
use mod_api::*;
use std::sync::{Arc, Mutex};

pub const BACKSTABBING: &str = "farmersdelight:backstabbing";
const CRIT: &str = "minecraft:entity.player.attack.crit";

/// Crit sounds owed at the end of the tick: (attacker id, where).
pub type Pending = Arc<Mutex<Vec<(i32, [f64; 3])>>>;

/// `isLookingBehindTarget`: the attacker stands in the cone behind where the target looks.
pub fn behind(target: [f64; 3], view: [f64; 3], attacker: [f64; 3]) -> bool {
    let d = [
        attacker[0] - target[0],
        attacker[1] - target[1],
        attacker[2] - target[2],
    ];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    // Vec3.normalize: a vector shorter than 1e-5 becomes zero.
    let n = if len < 1.0e-5 {
        [0.0; 3]
    } else {
        [d[0] / len, d[1] / len, d[2] / len]
    };
    let flat = [n[0], 0.0, n[2]];
    flat[0] * view[0] + flat[1] * view[1] + flat[2] * view[2] < -0.5
}

/// The JSON's `multiply` by `linear(base 1.4, per_level_above_first 0.2)`.
pub fn multiplier(level: u32) -> f32 {
    1.4 + 0.2 * (level as f32 - 1.0)
}

pub fn on_incoming(pending: &Pending, event: &mut LivingIncomingDamageEvent<'_>) {
    let amount = event.container.new_damage();
    if amount <= 0.0 || event.source.causing == 0 {
        return;
    }
    let Some(from) = event.source.position else {
        return;
    };
    let Some(weapon) = event.weapon else {
        return;
    };
    let level = crate::blocks::cutting_board::enchantment_level(weapon, BACKSTABBING);
    if level == 0 || !behind(event.victim.position, event.victim.view, from) {
        return;
    }
    let dealt = amount * multiplier(level);
    if dealt != amount {
        event.container.set_new_damage(dealt);
        pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((event.source.causing, from));
    }
}

/// The crit sound, at the attacker's position as the hit saw it, in the attacker's level.
pub fn drain(pending: &Pending, host: &mut dyn ServerHost) {
    let owed: Vec<_> = std::mem::take(&mut *pending.lock().unwrap_or_else(|e| e.into_inner()));
    for (attacker, at) in owed {
        let Some(level) = host.entity(attacker as u32).and_then(|e| e.level(&*host)) else {
            continue;
        };
        level.play_sound(host, CRIT, at, SoundSource::Blocks, 1.0, 1.0, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behind_means_the_back_cone() {
        let view = [0.0, 0.0, 1.0]; // looking south
        assert!(behind([0.0; 3], view, [0.0, 1.6, -2.0]));
        assert!(!behind([0.0; 3], view, [0.0, 0.0, 2.0]));
        assert!(!behind([0.0; 3], view, [2.0, 0.0, 0.0]));
        assert!(behind([0.0; 3], view, [-1.0, 0.0, -2.0]));
    }

    #[test]
    fn multipliers_per_level() {
        assert!((multiplier(1) - 1.4).abs() < 1e-6);
        assert!((multiplier(2) - 1.6).abs() < 1e-6);
        assert!((multiplier(3) - 1.8).abs() < 1e-6);
    }
}
