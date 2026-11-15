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

static KEYS: OnceLock<[ItemKey; 6]> = OnceLock::new();

pub fn register(registrar: &mut dyn Registrar) {
    let keys = KNIVES.map(|id| registrar.item_key(id));
    let _ = KEYS.set(keys);
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
    fn knockback_loses_a_float_tenth() {
        assert_eq!(knocked(0.4), 0.4 - 0.10000000149011612);
    }
}
