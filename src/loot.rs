//! Loot (J/refabricated/LootModificationEvents.java, J/common/loot/**). Fabric's
//! `LootTableEvents.MODIFY` builders become JSON pools added in `loot_table_load`; FD's
//! `MODIFY_DROPS` additions (leather from knife kills, cake slices from candle cakes) become
//! tag-gated pools on every entity / block table. `smoker_cook` and
//! `can_item_perform_ability` are evaluated here.
use mod_api::serde_json::{Value, json};
use mod_api::*;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

const KNIVES: &str = "#farmersdelight:tools/knives";
const STRAW_HARVESTERS: &str = "#farmersdelight:straw_harvesters";

/// `Configuration.GENERATE_FD_CHEST_LOOT` (`world.generateFDChestLoot`, default on).
pub fn chest_loot_enabled() -> bool {
    crate::config::chest_loot_enabled()
}

/// Vanilla chest table → FD's nested table.
const CHESTS: [(&str, &str); 14] = [
    (
        "minecraft:chests/abandoned_mineshaft",
        "farmersdelight:chests/fd_abandoned_mineshaft",
    ),
    (
        "minecraft:chests/bastion_hoglin_stable",
        "farmersdelight:chests/fd_bastion_hoglin_stable",
    ),
    (
        "minecraft:chests/bastion_treasure",
        "farmersdelight:chests/fd_bastion_treasure",
    ),
    (
        "minecraft:chests/end_city_treasure",
        "farmersdelight:chests/fd_end_city_treasure",
    ),
    (
        "minecraft:chests/pillager_outpost",
        "farmersdelight:chests/fd_pillager_outpost",
    ),
    (
        "minecraft:chests/ruined_portal",
        "farmersdelight:chests/fd_ruined_portal",
    ),
    (
        "minecraft:chests/shipwreck_supply",
        "farmersdelight:chests/fd_shipwreck_supply",
    ),
    (
        "minecraft:chests/simple_dungeon",
        "farmersdelight:chests/fd_simple_dungeon",
    ),
    (
        "minecraft:chests/village/village_butcher",
        "farmersdelight:chests/fd_village_butcher",
    ),
    (
        "minecraft:chests/village/village_desert_house",
        "farmersdelight:chests/fd_village_desert_house",
    ),
    (
        "minecraft:chests/village/village_plains_house",
        "farmersdelight:chests/fd_village_plains_house",
    ),
    (
        "minecraft:chests/village/village_savanna_house",
        "farmersdelight:chests/fd_village_savanna_house",
    ),
    (
        "minecraft:chests/village/village_snowy_house",
        "farmersdelight:chests/fd_village_snowy_house",
    ),
    (
        "minecraft:chests/village/village_taiga_house",
        "farmersdelight:chests/fd_village_taiga_house",
    ),
];

fn knife_tool() -> Value {
    json!({"type": "minecraft:match_tool", "predicate": {"items": KNIVES}})
}

fn not(term: Value) -> Value {
    json!({"type": "minecraft:inverted", "term": term})
}

fn silk_touch_tool() -> Value {
    json!({"type": "minecraft:match_tool", "predicate": {
        "predicates": {"minecraft:enchantments": [{"enchantments": "minecraft:silk_touch"}]}
    }})
}

fn attacker_knife() -> Value {
    json!({"type": "minecraft:entity_properties", "entity": "attacker",
           "predicate": {"minecraft:equipment": {"mainhand": {"items": KNIVES}}}})
}

fn on_fire(fire: bool) -> Value {
    json!({"type": "minecraft:entity_properties", "entity": "this",
           "predicate": {"minecraft:flags": {"is_on_fire": fire}}})
}

/// `randomChanceAndLootingBoost(0.5, 0.1)`.
fn looting_chance() -> Value {
    json!({"type": "minecraft:random_chance_with_enchanted_bonus",
           "enchantment": "minecraft:looting", "unenchanted_chance": 0.5,
           "enchanted_chance": {"type": "minecraft:linear", "base": 0.6, "per_level_above_first": 0.1}})
}

fn item(name: &str, conditions: Vec<Value>, count: Option<i32>) -> Value {
    let mut entry = json!({"type": "minecraft:item", "name": name});
    if let Some(c) = all_of(conditions) {
        entry["condition"] = c;
    }
    if let Some(n) = count {
        entry["modifier"] = json!({"type": "minecraft:set_count", "count": n});
    }
    entry
}

/// A one-roll pool of one entry (`LootPool.lootPool().add(entry)`), with pool conditions.
fn pool(entry: Value, conditions: Vec<Value>) -> Value {
    let mut p = json!({"rolls": 1, "entries": [entry]});
    if let Some(c) = all_of(conditions) {
        p["condition"] = c;
    }
    p
}

/// 26.3 loot JSON holds one `condition`; several are an `all_of`.
fn all_of(mut conditions: Vec<Value>) -> Option<Value> {
    match conditions.len() {
        0 => None,
        1 => conditions.pop(),
        _ => Some(json!({"type": "minecraft:all_of", "terms": conditions})),
    }
}

fn block_state(block: &str, property: &str, value: u32) -> Value {
    json!({"type": "minecraft:match_block", "blocks": block,
           "state": {property: value.to_string()}})
}

/// `tableBuilder.modifyPools(b -> b.when(condition))`.
fn guard_pools(table: &mut Value, condition: &Value) {
    if let Some(pools) = table.get_mut("pools").and_then(Value::as_array_mut) {
        for p in pools {
            let joined = match p.get("condition").cloned() {
                Some(old) => json!({"type": "minecraft:all_of", "terms": [old, condition.clone()]}),
                None => condition.clone(),
            };
            p["condition"] = joined;
        }
    }
}

fn add_pool(table: &mut Value, p: Value) {
    if !table.get("pools").is_some_and(Value::is_array) {
        table["pools"] = json!([]);
    }
    if let Some(pools) = table["pools"].as_array_mut() {
        pools.push(p);
    }
}

/// `pastrySlicing`: without a knife the old pools; with one, the slices left (`max − bites`).
fn pastry(table: &mut Value, block: &str, slice: &str, max: u32) {
    guard_pools(table, &not(knife_tool()));
    for bites in 0..max {
        add_pool(
            table,
            pool(
                item(slice, vec![], Some((max - bites) as i32)),
                vec![json!({"type": "minecraft:all_of",
                            "terms": [knife_tool(), block_state(block, "bites", bites)]})],
            ),
        );
    }
}

fn straw_chance(table: &mut Value) {
    let when = json!({"type": "minecraft:all_of", "terms": [
        {"type": "minecraft:random_chance", "chance": 0.3},
        {"type": "minecraft:match_tool", "predicate": {"items": STRAW_HARVESTERS}}
    ]});
    add_pool(
        table,
        pool(item("farmersdelight:straw", vec![when], None), vec![]),
    );
}

fn straw_at_age(table: &mut Value, block: &str, age: u32) {
    let when = json!({"type": "minecraft:all_of", "terms": [
        {"type": "minecraft:match_tool", "predicate": {"items": STRAW_HARVESTERS}},
        block_state(block, "age", age)
    ]});
    add_pool(
        table,
        pool(item("farmersdelight:straw", vec![when], None), vec![]),
    );
}

fn scavenge(table: &mut Value, name: &str, extra: Vec<Value>) {
    let mut terms = vec![attacker_knife()];
    terms.extend(extra);
    let when = json!({"type": "minecraft:all_of", "terms": terms});
    add_pool(table, pool(item(name, vec![when], None), vec![]));
}

/// `LootModificationEvents#modifyTable` (+ the MODIFY_DROPS rows as pools).
pub fn on_table_load(id: &str, table: &mut Value) {
    if chest_loot_enabled()
        && let Some((_, nested)) = CHESTS.iter().find(|(k, _)| *k == id)
    {
        add_pool(
            table,
            pool(
                json!({"type": "minecraft:loot_table", "value": nested}),
                vec![],
            ),
        );
    }
    match id {
        "minecraft:entities/chicken" => scavenge(table, "minecraft:feather", vec![]),
        "minecraft:entities/hoglin" => {
            scavenge(table, "farmersdelight:ham", vec![on_fire(false)]);
            scavenge(table, "farmersdelight:smoked_ham", vec![on_fire(true)]);
        }
        "minecraft:entities/pig" => {
            scavenge(
                table,
                "farmersdelight:ham",
                vec![on_fire(false), looting_chance()],
            );
            scavenge(
                table,
                "farmersdelight:smoked_ham",
                vec![on_fire(true), looting_chance()],
            );
        }
        "minecraft:entities/rabbit" => scavenge(table, "minecraft:rabbit_hide", vec![]),
        "minecraft:entities/shulker" => scavenge(table, "minecraft:shulker_shell", vec![]),
        "minecraft:entities/spider" | "minecraft:entities/cave_spider" => {
            scavenge(table, "minecraft:string", vec![])
        }
        "minecraft:blocks/pumpkin" => {
            guard_pools(
                table,
                &json!({"type": "minecraft:any_of",
                        "terms": [not(knife_tool()), silk_touch_tool()]}),
            );
            add_pool(
                table,
                pool(
                    item("farmersdelight:pumpkin_slice", vec![], Some(4)),
                    vec![json!({"type": "minecraft:all_of",
                                "terms": [knife_tool(), not(silk_touch_tool())]})],
                ),
            );
        }
        "farmersdelight:blocks/apple_pie" => pastry(
            table,
            "farmersdelight:apple_pie",
            "farmersdelight:apple_pie_slice",
            4,
        ),
        "minecraft:blocks/cake" => pastry(table, "minecraft:cake", "farmersdelight:cake_slice", 7),
        "farmersdelight:blocks/chocolate_pie" => pastry(
            table,
            "farmersdelight:chocolate_pie",
            "farmersdelight:chocolate_pie_slice",
            4,
        ),
        "farmersdelight:blocks/pumpkin_pie" => pastry(
            table,
            "farmersdelight:pumpkin_pie",
            "farmersdelight:pumpkin_pie_slice",
            4,
        ),
        "farmersdelight:blocks/sweet_berry_cheesecake" => pastry(
            table,
            "farmersdelight:sweet_berry_cheesecake",
            "farmersdelight:sweet_berry_cheesecake_slice",
            4,
        ),
        "minecraft:blocks/short_grass" | "minecraft:blocks/tall_grass" => straw_chance(table),
        "farmersdelight:blocks/sandy_shrub" => straw_chance(table),
        "farmersdelight:blocks/rice_panicles" => {
            straw_at_age(table, "farmersdelight:rice_panicles", 3)
        }
        "minecraft:blocks/wheat" => straw_at_age(table, "minecraft:wheat", 7),
        _ => {}
    }
    // MODIFY_DROPS: one leather from a knife kill of a `#farmersdelight:drops_leather` mob, on
    // the table it rolls (its own), so every entity table carries the tag-gated pool.
    if id
        .split_once(':')
        .is_some_and(|(_, path)| path.starts_with("entities/"))
    {
        let leather_mob = json!({"type": "minecraft:entity_properties", "entity": "this",
            "predicate": {"minecraft:entity_type": "#farmersdelight:drops_leather"}});
        add_pool(
            table,
            pool(
                item(
                    "minecraft:leather",
                    vec![attacker_knife(), leather_mob],
                    None,
                ),
                vec![],
            ),
        );
    }
    // MODIFY_DROPS: 7 cake slices from a `#farmersdelight:drops_cake_slice` block broken with a
    // knife, on its own table.
    if id
        .split_once(':')
        .is_some_and(|(_, path)| path.starts_with("blocks/"))
    {
        let candle_cake = json!({"type": "minecraft:match_block",
            "blocks": "#farmersdelight:drops_cake_slice"});
        add_pool(
            table,
            pool(
                item(
                    "farmersdelight:cake_slice",
                    vec![knife_tool(), candle_cake],
                    Some(7),
                ),
                vec![],
            ),
        );
    }
}

// ---------------------------------------------------------------- custom kinds

/// `CanItemPerformAbility`: the context's tool can perform `ability` (each ability is a tag).
pub fn on_condition(event: &mut LootConditionEvent<'_>) {
    if event.kind != "farmersdelight:can_item_perform_ability" {
        return;
    }
    let Some(tool) = &event.ctx.tool else {
        event.holds = false;
        return;
    };
    let tag = event
        .data
        .get("ability")
        .and_then(Value::as_str)
        .and_then(crate::recipes::ingredient::ability_tag);
    event.holds = tag
        .and_then(|t| Identifier::parse(t.trim_start_matches('#')).ok())
        .and_then(|id| event.tags.key(&id))
        .and_then(|key| event.tags.is_in_tag(tool.item, key))
        == Some(true);
}

/// Smoking recipes by input item (`RecipeType.SMOKING`), refreshed when the server's data loads.
static SMOKING: LazyLock<RwLock<HashMap<ItemKey, ItemStack>>> = LazyLock::new(Default::default);

pub fn refresh_smoking(host: &dyn Host) {
    let mut map = HashMap::new();
    for recipe in host.recipes_of_type("minecraft:smoking").iter() {
        let (Some(input), Some(result)) = (recipe.ingredients.first(), recipe.results.first())
        else {
            continue;
        };
        if let Ok(keys) = input.item_keys(host) {
            for key in keys.iter() {
                // getRecipeFor: the first matching recipe wins.
                map.entry(*key).or_insert_with(|| result.clone());
            }
        }
    }
    *SMOKING.write().unwrap_or_else(|e| e.into_inner()) = map;
}

/// `SmokerCookFunction`: the stack's smoking result, count × the stack's count.
pub fn on_function(event: &mut LootFunctionEvent<'_>) {
    if event.kind != "farmersdelight:smoker_cook" || event.stack.count <= 0 {
        return;
    }
    let map = SMOKING.read().unwrap_or_else(|e| e.into_inner());
    if let Some(result) = map.get(&event.stack.item) {
        event.stack.item = result.item();
        event.stack.count *= result.count() as i32;
        event.stack.damage = 0;
        event.stack.enchantments.clear();
        event.stack.potion = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chest_tables_get_one_nested_pool() {
        let mut t = json!({"pools": [{"rolls": 1, "entries": []}]});
        on_table_load("minecraft:chests/simple_dungeon", &mut t);
        assert_eq!(t["pools"].as_array().unwrap().len(), 2);
        assert_eq!(
            t["pools"][1]["entries"][0]["value"],
            "farmersdelight:chests/fd_simple_dungeon"
        );
    }

    #[test]
    fn cake_gets_seven_slice_pools_and_a_knife_guard() {
        let mut t = json!({"pools": [{"rolls": 1, "entries": [], "condition": {"type": "minecraft:survives_explosion"}}]});
        on_table_load("minecraft:blocks/cake", &mut t);
        let pools = t["pools"].as_array().unwrap();
        assert_eq!(pools.len(), 9); // + the tag-gated candle cake pool
        assert_eq!(pools[0]["condition"]["terms"].as_array().unwrap().len(), 2);
        assert_eq!(pools[1]["entries"][0]["modifier"]["count"], 7);
        assert_eq!(pools[7]["entries"][0]["modifier"]["count"], 1);
    }

    #[test]
    fn leather_only_for_the_tag_members() {
        let mut cow = json!({"pools": []});
        on_table_load("minecraft:entities/cow", &mut cow);
        let pool = &cow["pools"][0]["entries"][0];
        assert_eq!(pool["name"], "minecraft:leather");
        assert_eq!(
            pool["condition"]["terms"][1]["predicate"]["minecraft:entity_type"],
            "#farmersdelight:drops_leather"
        );
        let mut chest = json!({"pools": []});
        on_table_load("minecraft:chests/igloo_chest", &mut chest);
        assert_eq!(chest["pools"].as_array().unwrap().len(), 0);
    }
}
