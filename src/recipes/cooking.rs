//! `farmersdelight:cooking` (J/common/crafting/CookingPotRecipe.java), parsed from raw JSON.
//!
//! Matching follows `CookingPotRecipe#matches` with Refabricated's `RecipeWrapper`: the number of
//! non-empty input slots must equal the number of ingredients, and `StackedItemContents` is fed
//! each slot with a cap of **one** unit (`accountStack(stack, 1)`), so a recipe matches exactly
//! when every ingredient can be given its own distinct non-empty slot (a perfect bipartite
//! matching). Stack sizes never matter; two identical ingredients need two slots.
use super::ingredient::tool_alternatives;
use mod_api::*;
use serde_json::Value;

pub const TYPE: &str = "farmersdelight:cooking";
/// `CookingPotRecipe.INPUT_SLOTS`, also the codec's `listOf(1, 6)` upper bound.
pub const INPUT_SLOTS: usize = 6;
/// Codec defaults.
pub const DEFAULT_COOKING_TIME: i32 = 200;

/// `CookingPotBookCategory`; the codec's default is `MISC`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    Meals,
    Drinks,
    #[default]
    Misc,
}

impl Tab {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "meals" => Self::Meals,
            "drinks" => Self::Drinks,
            "misc" => Self::Misc,
            _ => return None,
        })
    }
}

/// The parts of one recipe file that need no host to validate.
#[derive(Clone, Debug, PartialEq)]
pub struct CookingShape {
    pub group: String,
    pub tab: Tab,
    /// Per ingredient, plain ingredient JSON values any one of which accepts a stack
    /// (`fabric:any` is flattened to its children).
    pub ingredients: Vec<Vec<Value>>,
    pub result: Value,
    pub container: Option<Value>,
    pub experience: f32,
    pub cooking_time: i32,
}

/// Validate a cooking recipe's JSON as `CookingPotRecipe.Serializer.CODEC` does.
pub fn shape(json: &Value) -> Result<CookingShape, String> {
    let group = match json.get("group") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => return Err("group is not a string".into()),
    };
    let tab = match json.get("recipe_book_tab") {
        None => Tab::default(),
        Some(v) => v
            .as_str()
            .and_then(Tab::parse)
            .ok_or_else(|| format!("unknown recipe_book_tab {v}"))?,
    };
    let raw = json["ingredients"]
        .as_array()
        .ok_or("missing ingredients")?;
    if raw.is_empty() || raw.len() > INPUT_SLOTS {
        return Err(format!("ingredients must list 1 to {INPUT_SLOTS} entries"));
    }
    let ingredients = raw
        .iter()
        .map(tool_alternatives)
        .collect::<Result<Vec<_>, String>>()?;
    let result = json.get("result").ok_or("missing result")?.clone();
    let container = json.get("container").cloned();
    let experience = match json.get("experience") {
        None => 0.0,
        Some(v) => v.as_f64().ok_or("experience is not a number")? as f32,
    };
    let cooking_time = match json.get("cookingtime") {
        None => DEFAULT_COOKING_TIME,
        Some(v) => v
            .as_i64()
            .and_then(|t| i32::try_from(t).ok())
            .ok_or("cookingtime is not an int")?,
    };
    Ok(CookingShape {
        group,
        tab,
        ingredients,
        result,
        container,
        experience,
        cooking_time,
    })
}

#[derive(Clone, Debug)]
pub struct CookingRecipe {
    pub id: String,
    pub group: String,
    pub tab: Tab,
    /// Per ingredient, the alternatives (any one accepting is enough).
    pub ingredients: Vec<Vec<Ingredient>>,
    pub result: ItemStack,
    /// The explicit `container`; the derived default (crafting remainder, override map) is
    /// the block entity's business (M3-03).
    pub container: Option<ItemStack>,
    pub experience: f32,
    pub cooking_time: i32,
}

impl CookingRecipe {
    /// Whether the input slots match (see the module doc). `accepts(ingredient, stack)`
    /// tests one alternative against one stack.
    pub fn matches<T>(
        &self,
        slots: &[Option<T>],
        mut accepts: impl FnMut(&Ingredient, &T) -> bool,
    ) -> bool {
        let filled: Vec<&T> = slots.iter().flatten().collect();
        perfect_matching(self.ingredients.len(), filled.len(), |i, s| {
            self.ingredients[i]
                .iter()
                .any(|alt| accepts(alt, filled[s]))
        })
    }
}

/// Every ingredient gets its own slot and every slot is used (counts equal, Kuhn's algorithm;
/// at most 6×6, so the edge test is cached once).
pub fn perfect_matching(
    ingredients: usize,
    slots: usize,
    mut edge: impl FnMut(usize, usize) -> bool,
) -> bool {
    if ingredients != slots {
        return false;
    }
    let adj: Vec<Vec<usize>> = (0..ingredients)
        .map(|i| (0..slots).filter(|&s| edge(i, s)).collect())
        .collect();
    let mut owner: Vec<Option<usize>> = vec![None; slots];
    fn augment(
        i: usize,
        adj: &[Vec<usize>],
        seen: &mut [bool],
        owner: &mut [Option<usize>],
    ) -> bool {
        for &s in &adj[i] {
            if seen[s] {
                continue;
            }
            seen[s] = true;
            if owner[s].is_none_or(|j| augment(j, adj, seen, owner)) {
                owner[s] = Some(i);
                return true;
            }
        }
        false
    }
    (0..ingredients).all(|i| augment(i, &adj, &mut vec![false; slots], &mut owner))
}

/// `RecipeManager.CachedCheck#getRecipeFor`: the recipe that matched last time is tried
/// first, then every recipe in file order.
pub fn find<'a, T>(
    table: &'a [CookingRecipe],
    last: Option<&str>,
    slots: &[Option<T>],
    mut accepts: impl FnMut(&Ingredient, &T) -> bool,
) -> Option<&'a CookingRecipe> {
    if slots.iter().all(Option::is_none) {
        return None; // `hasInput()`
    }
    let cached = last.and_then(|id| table.iter().find(|r| r.id == id));
    cached
        .into_iter()
        .chain(table.iter())
        .find(|r| r.matches(slots, &mut accepts))
}

/// Build the table from the host's recipes, in file order. Invalid entries are skipped and
/// returned with their reason (one per id per load).
pub fn build(host: &dyn Host) -> (Vec<CookingRecipe>, Vec<(String, String)>) {
    let mut table = Vec::new();
    let mut skipped = Vec::new();
    for recipe in host.recipes_of_type(TYPE).iter() {
        match resolve(host, &recipe.json) {
            Ok(r) => table.push(CookingRecipe {
                id: recipe.id.to_string(),
                ..r
            }),
            Err(e) => skipped.push((recipe.id.to_string(), e)),
        }
    }
    (table, skipped)
}

fn stack(host: &dyn Host, json: &Value) -> Result<ItemStack, String> {
    host.recipe_result(&json.to_string())
        .ok()
        .flatten()
        .ok_or_else(|| format!("unknown item {json}"))
}

fn resolve(host: &dyn Host, json: &Value) -> Result<CookingRecipe, String> {
    let shape = shape(json)?;
    let ingredients = shape
        .ingredients
        .iter()
        .enumerate()
        .map(|(n, alternatives)| {
            let resolved: Vec<_> = alternatives
                .iter()
                .filter_map(|a| host.recipe_ingredient(&a.to_string()).ok().flatten())
                .collect();
            if resolved.is_empty() {
                Err(format!("ingredient {n} resolves to no items"))
            } else {
                Ok(resolved)
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CookingRecipe {
        id: String::new(),
        group: shape.group,
        tab: shape.tab,
        ingredients,
        result: stack(host, &shape.result)?,
        container: shape
            .container
            .as_ref()
            .map(|c| stack(host, c))
            .transpose()?,
        experience: shape.experience,
        cooking_time: shape.cooking_time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn fixture() -> Vec<(String, Value)> {
        let text = include_str!("../../tests/fixtures/cooking_recipes.json");
        let doc: Value = serde_json::from_str(text).unwrap();
        doc["cooking_recipes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["id"].as_str().unwrap().to_owned(), r["json"].clone()))
            .collect()
    }

    fn get(id: &str) -> Value {
        fixture()
            .into_iter()
            .find(|(i, _)| i == id)
            .unwrap_or_else(|| panic!("{id} not in fixture"))
            .1
    }

    #[test]
    fn every_reference_recipe_has_a_valid_shape() {
        let all = fixture();
        assert_eq!(all.len(), 28);
        let mut sizes: BTreeMap<usize, usize> = BTreeMap::new();
        let mut tabs: BTreeMap<String, usize> = BTreeMap::new();
        for (id, json) in &all {
            let s = shape(json).unwrap_or_else(|e| panic!("{id}: {e}"));
            *sizes.entry(s.ingredients.len()).or_default() += 1;
            *tabs.entry(format!("{:?}", s.tab)).or_default() += 1;
        }
        let sizes: Vec<_> = sizes.into_iter().collect();
        assert_eq!(sizes, [(1, 1), (2, 4), (3, 6), (4, 15), (5, 1), (6, 1)]);
        let tabs: Vec<_> = tabs.into_iter().collect();
        assert_eq!(
            tabs,
            [
                ("Drinks".into(), 2),
                ("Meals".into(), 20),
                ("Misc".into(), 6)
            ]
        );
    }

    #[test]
    fn the_three_fabric_any_files_flatten() {
        for (id, n) in [
            ("farmersdelight:cooking/bone_broth", 1),
            ("farmersdelight:cooking/cabbage_rolls", 1),
            ("farmersdelight:cooking/dumplings", 3),
        ] {
            let s = shape(&get(id)).unwrap();
            assert_eq!(s.ingredients[n].len(), 4, "{id}");
        }
        let rolls = shape(&get("farmersdelight:cooking/cabbage_rolls")).unwrap();
        assert_eq!(
            rolls.ingredients[1],
            vec![
                json!("#c:foods/raw_meat"),
                json!("#c:foods/safe_raw_fish"),
                json!("#c:foods/vegetable"),
                json!("#c:mushrooms")
            ]
        );
    }

    #[test]
    fn defaults_and_explicit_fields() {
        let cider = shape(&get("farmersdelight:cooking/apple_cider")).unwrap();
        assert_eq!(cider.tab, Tab::Drinks);
        assert_eq!(cider.experience, 1.0);
        assert_eq!(cider.group, "");
        assert_eq!(cider.container, None);
        let minimal = shape(&json!({"ingredients": ["a:b"], "result": {"id": "a:c"}})).unwrap();
        assert_eq!(minimal.tab, Tab::Misc);
        assert_eq!(minimal.experience, 0.0);
        assert_eq!(minimal.cooking_time, DEFAULT_COOKING_TIME);
        let stew = shape(&get("farmersdelight:cooking/beetroot_soup")).unwrap();
        assert!(stew.container.is_some());
    }

    #[test]
    fn codec_limits() {
        let base = json!({"ingredients": ["a:b"], "result": {"id": "a:c"}});
        let with = |k: &str, v: Value| {
            let mut j = base.clone();
            j[k] = v;
            shape(&j)
        };
        assert!(with("ingredients", json!([])).is_err());
        assert!(with("ingredients", Value::from(vec!["a:b"; 7])).is_err());
        assert!(with("ingredients", Value::from(vec!["a:b"; 6])).is_ok());
        assert!(with("recipe_book_tab", json!("soups")).is_err());
        assert!(with("cookingtime", json!(1.5)).is_err());
        assert!(with("experience", json!("x")).is_err());
        assert!(shape(&json!({"ingredients": ["a:b"]})).is_err());
    }

    fn ing(items: &[&str]) -> Ingredient {
        Ingredient::of(items.iter().map(|s| s.to_string()).collect())
    }

    fn recipe(id: &str, ingredients: &[&[&str]]) -> CookingRecipe {
        CookingRecipe {
            id: id.into(),
            group: String::new(),
            tab: Tab::Misc,
            ingredients: ingredients.iter().map(|alts| vec![ing(alts)]).collect(),
            result: ItemStack::new(host::key("minecraft:bone_meal").unwrap(), 1).unwrap(),
            container: None,
            experience: 0.0,
            cooking_time: 200,
        }
    }

    fn slots(items: &[&'static str]) -> Vec<Option<&'static str>> {
        items
            .iter()
            .map(|s| (!s.is_empty()).then_some(*s))
            .collect()
    }

    fn has(i: &Ingredient, s: &&str) -> bool {
        i.items.contains(*s)
    }

    #[test]
    fn exact_slot_count_rule() {
        let cider = recipe("cider", &[&["apple"], &["apple"], &["sugar"]]);
        assert!(cider.matches(&slots(&["apple", "", "apple", "sugar", "", ""]), has));
        // one stack of apples is one unit: two apple ingredients need two slots
        assert!(!cider.matches(&slots(&["apple", "sugar"]), has));
        // an extra filled slot (even a matching one) breaks the count
        assert!(!cider.matches(&slots(&["apple", "apple", "sugar", "apple"]), has));
        assert!(!cider.matches(&slots(&["apple", "apple", "sugar", "dirt"]), has));
        assert!(!cider.matches(&slots(&["apple", "dirt", "sugar"]), has));
    }

    #[test]
    fn overlapping_ingredients_need_an_augmenting_path() {
        // ingredient 0 takes {a, b}, ingredient 1 only {a}: the greedy pick a→0 fails, the
        // matcher must move 0 to b.
        let r = recipe("r", &[&["a", "b"], &["a"]]);
        assert!(r.matches(&slots(&["a", "b"]), has));
        assert!(r.matches(&slots(&["b", "a"]), has));
        assert!(!r.matches(&slots(&["b", "b"]), has));
        // tag-like alternatives across six slots
        let six = recipe(
            "six",
            &[
                &["m1", "m2"],
                &["m2"],
                &["v"],
                &["v", "m1"],
                &["x"],
                &["x", "v"],
            ],
        );
        assert!(six.matches(&slots(&["x", "v", "m1", "m2", "v", "x"]), has));
        assert!(!six.matches(&slots(&["x", "v", "m2", "v", "v", "v"]), has));
    }

    #[test]
    fn fabric_any_alternatives_match_any_child() {
        let mut broth = recipe("broth", &[&["bone"]]);
        broth.ingredients.push(vec![
            ing(&["glow_berries"]),
            ing(&["red_mushroom", "brown_mushroom"]),
        ]);
        assert!(broth.matches(&slots(&["brown_mushroom", "bone"]), has));
        assert!(broth.matches(&slots(&["glow_berries", "bone"]), has));
        assert!(!broth.matches(&slots(&["dirt", "bone"]), has));
    }

    #[test]
    fn find_prefers_the_last_match_then_file_order() {
        let table = vec![
            recipe("a", &[&["x", "y"]]),
            recipe("b", &[&["x"]]),
            recipe("c", &[&["x"], &["x"]]),
        ];
        let id = |r: Option<&CookingRecipe>| r.map(|r| r.id.clone());
        assert_eq!(
            id(find(&table, None, &slots(&["x"]), has)),
            Some("a".into())
        );
        assert_eq!(
            id(find(&table, Some("b"), &slots(&["x"]), has)),
            Some("b".into())
        );
        // a stale cached id falls back to file order
        assert_eq!(
            id(find(&table, Some("c"), &slots(&["x"]), has)),
            Some("a".into())
        );
        assert_eq!(
            id(find(&table, None, &slots(&["x", "", "x"]), has)),
            Some("c".into())
        );
        assert_eq!(id(find(&table, None, &slots(&["", ""]), has)), None);
        assert_eq!(id(find(&table, None, &slots(&["z"]), has)), None);
    }

    /// A recipes-only host, as in `cutting.rs`.
    mod host {
        use super::*;
        use mod_api::prelude::ItemsHost;
        use std::sync::Arc;

        pub struct Data {
            pub recipes: Arc<[Recipe]>,
        }
        impl Host for Data {}
        impl mod_api::prelude::RecipesHost for Data {
            fn recipes_of_type(&self, kind: &str) -> Arc<[Recipe]> {
                if kind == TYPE {
                    self.recipes.clone()
                } else {
                    Arc::from(Vec::new())
                }
            }
            fn recipe_ingredient(&self, json: &str) -> Result<Option<Ingredient>, RecipeJsonError> {
                let v: Value =
                    serde_json::from_str(json).map_err(|_| RecipeJsonError::InvalidJson)?;
                Ok(v.as_str()
                    .filter(|s| *s != "#empty")
                    .map(|s| Ingredient::of(vec![s.to_owned()])))
            }
            fn recipe_result(&self, json: &str) -> Result<Option<ItemStack>, RecipeJsonError> {
                let v: Value =
                    serde_json::from_str(json).map_err(|_| RecipeJsonError::InvalidJson)?;
                let count = v["count"].as_u64().unwrap_or(1) as u32;
                Ok(v["id"]
                    .as_str()
                    .and_then(key)
                    .and_then(|k| ItemStack::new(k, count)))
            }
        }
        mod_api::impl_host_areas!(Data, except(recipes));

        pub fn key(id: &str) -> Option<ItemKey> {
            let mut fake = mod_api::testing::FakeHost::new();
            let ids = [
                "farmersdelight:apple_cider",
                "farmersdelight:bone_broth",
                "minecraft:bone_meal",
                "minecraft:bowl",
                "minecraft:glass_bottle",
                "testaddon:mash",
            ];
            fake.registries
                .resolve_items(ids.iter().map(|i| Identifier::parse(i).unwrap()))
                .unwrap();
            fake.item_key(&Identifier::parse(id).ok()?)
        }

        pub fn data(entries: &[(&str, Value)]) -> Data {
            Data {
                recipes: entries
                    .iter()
                    .map(|(id, json)| Recipe::new(*id, TYPE, Arc::new(json.clone())))
                    .collect::<Vec<_>>()
                    .into(),
            }
        }
    }

    #[test]
    fn build_resolves_and_skips_invalid_entries() {
        let cider = get("farmersdelight:cooking/apple_cider");
        let broth = get("farmersdelight:cooking/bone_broth");
        let mut empty = cider.clone();
        empty["ingredients"][2] = json!("#empty");
        let mut container = cider.clone();
        container["container"] = json!({"id": "minecraft:glass_bottle"});
        let mut unknown = cider.clone();
        unknown["result"] = json!({"id": "farmersdelight:nothing"});
        let host = host::data(&[
            ("farmersdelight:cooking/apple_cider", cider),
            ("farmersdelight:cooking/bone_broth", broth),
            ("test:empty", empty),
            ("test:container", container),
            ("test:unknown", unknown),
        ]);
        let (table, skipped) = build(&host);
        let ids: Vec<_> = table.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "farmersdelight:cooking/apple_cider",
                "farmersdelight:cooking/bone_broth",
                "test:container"
            ]
        );
        let cider = &table[0];
        assert_eq!(cider.ingredients.len(), 3);
        assert_eq!(cider.cooking_time, DEFAULT_COOKING_TIME);
        assert_eq!(
            cider.result.item(),
            host::key("farmersdelight:apple_cider").unwrap()
        );
        assert_eq!(table[1].ingredients[1].len(), 4);
        assert_eq!(
            table[2].container.as_ref().unwrap().item(),
            host::key("minecraft:glass_bottle").unwrap()
        );
        let skipped: Vec<_> = skipped.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(skipped, ["test:empty", "test:unknown"]);
    }

    /// BX-10: an add-on's `farmersdelight:cooking` recipe (own namespace, `fabric:any` ingredient,
    /// foreign result) is read like FD's own; the `fabric:any` child naming an empty tag drops out.
    #[test]
    fn addon_namespace_recipe_builds() {
        let host = host::data(&[(
            "testaddon:cooking/mash",
            json!({"type": "farmersdelight:cooking", "ingredients": [
                    {"fabric:type": "fabric:any", "ingredients": ["minecraft:bone_meal", "#empty"]},
                    "minecraft:bowl"],
                "result": {"id": "testaddon:mash"}, "container": {"id": "minecraft:bowl"}}),
        )]);
        let (table, skipped) = build(&host);
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(table[0].id, "testaddon:cooking/mash");
        assert_eq!(table[0].ingredients[0].len(), 1);
        assert_eq!(table[0].result.item(), host::key("testaddon:mash").unwrap());
    }
}
