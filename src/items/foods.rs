//! Food-related behaviour beyond the declarations (M4-05, M4-06).
//!
//! - Vanilla overrides (J/common/event/CommonModBusEvents.java#onModifyDefaultComponents): soups in
//!   the stackable list stack to 16; Rabbit Stew gets 14/0.75 and Jump Boost II (200); vanilla
//!   soups grant Nourishment (mushroom/beetroot 3600, rabbit 6000); the straw bed is renamed
//!   "Hay Bed" (Q14). Read once at startup, like the reference's default components.
//! - Dog food and horse feed (DogFoodItem, HorseFeedItem): used on a tamed (or untamable) entity
//!   of `#farmersdelight:dog_food_users` / `horse_feed_users`: full health, the effects, an eat
//!   sound and five `star` particles; dog food leaves a bowl.
use crate::config;
use mod_api::serde_json::{Value, json};
use mod_api::*;
use std::sync::OnceLock;

pub const DOG_FOOD: &str = "farmersdelight:dog_food";
pub const HORSE_FEED: &str = "farmersdelight:horse_feed";
const DOG_FOOD_USERS: &str = "farmersdelight:dog_food_users";
const HORSE_FEED_USERS: &str = "farmersdelight:horse_feed_users";

/// `farmersdelight:star`, registered with the cooking pot's particle types.
pub static STAR: OnceLock<ParticleKey> = OnceLock::new();

fn nourishment(duration: i32) -> Value {
    json!({"id": crate::effects::NOURISHMENT, "duration": duration, "amplifier": 0, "show_particles": false})
}

/// `FoodValues.ConsumableValues.VANILLA_SOUP_EFFECTS`.
pub fn soup_nourishment(item: &str) -> Option<i32> {
    match item {
        "minecraft:mushroom_stew" | "minecraft:beetroot_soup" => Some(3600),
        "minecraft:rabbit_stew" => Some(6000),
        _ => None,
    }
}

/// `onModifyDefaultComponents` for one item's default components.
pub fn modify_default_components(
    item: &str,
    components: &mut mod_api::serde_json::Map<String, Value>,
) {
    // Each entry is `Identifier.parse`d, so a bare path names a `minecraft:` item; any item in the list is
    // affected (the BowlFoodItem restriction in the comment is not enforced).
    if config::stackable_soup_items_enabled()
        && config::soup_item_list().iter().any(|key| {
            if key.contains(':') {
                key == item
            } else {
                item.strip_prefix("minecraft:") == Some(key.as_str())
            }
        })
    {
        components.insert("minecraft:max_stack_size".into(), json!(16));
    }
    let buff = config::rabbit_stew_buff_enabled() && item == "minecraft:rabbit_stew";
    if buff {
        // FoodValues.RABBIT_STEW_BUFF: 14 / 0.75 → saturation 14·0.75·2.
        components.insert(
            "minecraft:food".into(),
            json!({"nutrition": 14, "saturation": 21.0}),
        );
    }
    let extra = config::vanilla_soup_extra_effects_enabled();
    if let Some(duration) = soup_nourishment(item)
        && (extra || config::rabbit_stew_buff_enabled())
    {
        let consumable = components
            .entry("minecraft:consumable")
            .or_insert_with(|| json!({}));
        if let Some(obj) = consumable.as_object_mut() {
            let list = obj.entry("on_consume_effects").or_insert_with(|| json!([]));
            if let Some(list) = list.as_array_mut() {
                if buff {
                    list.push(json!({"type": "minecraft:apply_effects", "effects": [
                        {"id": "minecraft:jump_boost", "duration": 200, "amplifier": 1}]}));
                }
                if extra {
                    list.push(json!({"type": "minecraft:apply_effects", "effects": [nourishment(duration)]}));
                }
            }
        }
    }
    if config::vanilla_straw_bed_rename_enabled() && item == "minecraft:straw_bed" {
        components.insert(
            "minecraft:item_name".into(),
            json!({"translate": "block.farmersdelight.hay_bed"}),
        );
    }
}

/// What feeding a pet gives: users tag, effects (id, duration, amplifier), sound, leaves a bowl.
pub type PetFood = (
    &'static str,
    &'static [(&'static str, i32, i32)],
    &'static str,
    bool,
);

pub fn pet_food(item: &str) -> Option<PetFood> {
    match item {
        DOG_FOOD => Some((
            DOG_FOOD_USERS,
            &[
                ("minecraft:speed", 6000, 0),
                ("minecraft:strength", 6000, 0),
                ("minecraft:resistance", 6000, 0),
            ],
            "minecraft:entity.generic.eat",
            true,
        )),
        HORSE_FEED => Some((
            HORSE_FEED_USERS,
            &[
                ("minecraft:speed", 6000, 1),
                ("minecraft:jump_boost", 6000, 0),
            ],
            "minecraft:entity.horse.eat",
            false,
        )),
        _ => None,
    }
}

/// `DogFoodEvent#onDogFoodApplied` / `HorseFeedEvent#onHorseFeedApplied`.
pub fn entity_interact(host: &mut dyn ServerHost, event: &mut EntityInteractEvent<'_>) -> Flow {
    let Some(stack) = event.stack else {
        return Flow::Continue;
    };
    let Some((users, effects, sound, bowl)) = pet_food(&stack.item().id(&*host)) else {
        return Flow::Continue;
    };
    let Some(player) = host.players().acting_player() else {
        return Flow::Continue;
    };
    if player.entity().spectator(&*host).unwrap_or(false)
        || !host.entity_type_in_tag(event.kind, users)
    {
        return Flow::Continue;
    }
    let Some(target) = host.entity(event.entity as u32) else {
        return Flow::Continue;
    };
    let Some(living) = host.living(target) else {
        return Flow::Continue;
    };
    // Untamable users are fed too; tamable ones only once tamed.
    let alive = living.health(&*host).is_some_and(|h| h > 0.0);
    if !alive || living.is_tamed(&*host) == Some(false) {
        return Flow::Continue;
    }
    let max = living.max_health(&*host).unwrap_or(0.0);
    let _ = living.set_health(host, max);
    for (id, duration, amplifier) in effects {
        if let Some(key) = crate::effects::key(id) {
            let _ = living.add_effect(
                host,
                &ActiveEffect::new(key, *duration, *amplifier, false, true, true),
            );
        }
    }
    let at = target.position(&*host).unwrap_or([0.0; 3]);
    if let Some(level) = player.level(&*host) {
        let block = [
            at[0].floor() + 0.5,
            at[1].floor() + 0.5,
            at[2].floor() + 0.5,
        ];
        level.play_sound(host, sound, block, SoundSource::Players, 0.8, 0.8, None);
        if let Some(star) = STAR.get() {
            let [w, h] = target.dimensions(&*host).unwrap_or([0.6, 0.85]);
            let center = [at[0], at[1] + h * 0.5 + 0.5, at[2]];
            let _ = level.send_particles(
                host,
                &ParticleOptions::simple(*star),
                None,
                center,
                [w * 0.5, h * 0.5, w * 0.5],
                0.02,
                5,
            );
        }
    }
    if !player.is_creative(&*host).unwrap_or(false) {
        if let Some(holder) = host.living(player.entity()) {
            let _ = holder.set_hand_item(host, event.hand, stack.clone().shrink(1));
        }
        if bowl && let Some(key) = host.item_key(&Identifier::parse("minecraft:bowl").expect("id"))
        {
            let _ = host.transaction(|host, tx| {
                crate::inventory::give_or_drop(
                    host,
                    tx,
                    player,
                    ItemStack::new(key, 1).expect("bowl"),
                );
                Ok::<_, ()>(())
            });
        }
    }
    Flow::Cancel
}

#[cfg(test)]
mod tests {
    use super::*;

    fn components(item: &str) -> mod_api::serde_json::Map<String, Value> {
        let mut c = mod_api::serde_json::Map::new();
        modify_default_components(item, &mut c);
        c
    }

    #[test]
    fn vanilla_soups_stack_and_nourish() {
        let stew = components("minecraft:mushroom_stew");
        assert_eq!(stew["minecraft:max_stack_size"], json!(16));
        let effects = &stew["minecraft:consumable"]["on_consume_effects"];
        assert_eq!(effects[0]["effects"][0]["duration"], json!(3600));
        let rabbit = components("minecraft:rabbit_stew");
        assert_eq!(rabbit["minecraft:food"]["nutrition"], json!(14));
        let effects = rabbit["minecraft:consumable"]["on_consume_effects"]
            .as_array()
            .unwrap();
        assert_eq!(
            effects[0]["effects"][0]["id"],
            json!("minecraft:jump_boost")
        );
        assert_eq!(effects[0]["effects"][0]["amplifier"], json!(1));
        assert_eq!(effects[1]["effects"][0]["duration"], json!(6000));
        assert!(components("minecraft:bread").is_empty());
        assert_eq!(
            components("minecraft:straw_bed")["minecraft:item_name"]["translate"],
            json!("block.farmersdelight.hay_bed")
        );
    }

    #[test]
    fn pet_foods() {
        let (tag, effects, _, bowl) = pet_food(DOG_FOOD).unwrap();
        assert_eq!(tag, DOG_FOOD_USERS);
        assert_eq!(effects.len(), 3);
        assert!(bowl);
        let (_, effects, _, bowl) = pet_food(HORSE_FEED).unwrap();
        assert_eq!(effects[0], ("minecraft:speed", 6000, 1));
        assert!(!bowl);
        assert!(pet_food("minecraft:bone").is_none());
    }
}
