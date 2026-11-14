//! `farmersdelight:cutting` (J/common/crafting/CuttingBoardRecipe.java), parsed from raw JSON.
use super::ingredient::tool_alternatives;
use mod_api::*;
use serde_json::Value;

pub const TYPE: &str = "farmersdelight:cutting";
/// `CuttingBoardRecipe.MAX_RESULTS`.
pub const MAX_RESULTS: usize = 4;

/// The parts of one recipe file that need no host to validate.
#[derive(Clone, Debug, PartialEq)]
pub struct CuttingShape {
    pub input: Value,
    pub tools: Vec<Value>,
    /// `(item JSON, chance)`; chance defaults to 1.0.
    pub results: Vec<(Value, f32)>,
    pub sound: Option<String>,
}

/// Validate a cutting recipe's JSON as the reference codec does.
pub fn shape(json: &Value) -> Result<CuttingShape, String> {
    let ingredients = json["ingredients"]
        .as_array()
        .ok_or("missing ingredients")?;
    match ingredients.len() {
        0 => return Err("No ingredients for cutting recipe".into()),
        1 => {}
        _ => return Err("Too many ingredients for cutting recipe".into()),
    }
    let tools = tool_alternatives(json.get("tool").ok_or("missing tool")?)?;
    let raw = json["result"].as_array().ok_or("missing result list")?;
    if raw.is_empty() || raw.len() > MAX_RESULTS {
        return Err(format!("result must list 1 to {MAX_RESULTS} entries"));
    }
    let results = raw
        .iter()
        .map(|r| {
            let item = r.get("item").ok_or("result without item")?.clone();
            let chance = match r.get("chance") {
                None => 1.0,
                Some(c) => c.as_f64().ok_or("chance is not a number")? as f32,
            };
            Ok((item, chance))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let sound = match json.get("sound") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        // SoundEvent.CODEC also accepts an inline `{sound_id, range}` object.
        Some(Value::Object(o)) => o.get("sound_id").and_then(Value::as_str).map(str::to_owned),
        Some(_) => return Err("invalid sound".into()),
    };
    Ok(CuttingShape {
        input: ingredients[0].clone(),
        tools,
        results,
        sound,
    })
}

#[derive(Clone, Debug)]
pub struct ChanceResult {
    pub stack: ItemStack,
    pub chance: f32,
}

#[derive(Clone, Debug)]
pub struct CuttingRecipe {
    pub id: String,
    pub input: Ingredient,
    /// Any one accepting the tool is enough.
    pub tools: Vec<Ingredient>,
    pub results: Vec<ChanceResult>,
    pub sound: Option<String>,
}

impl CuttingRecipe {
    pub fn tool_matches(&self, mut test: impl FnMut(&Ingredient) -> bool) -> bool {
        self.tools.iter().any(&mut test)
    }
}

/// Build the table from the host's recipes, in file order. Invalid entries are skipped and
/// returned with their reason (one per id per load).
pub fn build(host: &dyn Host) -> (Vec<CuttingRecipe>, Vec<(String, String)>) {
    let mut table = Vec::new();
    let mut skipped = Vec::new();
    for recipe in host.recipes_of_type(TYPE).iter() {
        match resolve(host, &recipe.json) {
            Ok(r) => table.push(CuttingRecipe {
                id: recipe.id.to_string(),
                ..r
            }),
            Err(e) => skipped.push((recipe.id.to_string(), e)),
        }
    }
    (table, skipped)
}

fn ingredient(host: &dyn Host, json: &Value) -> Option<Ingredient> {
    host.recipe_ingredient(&json.to_string()).ok().flatten()
}

fn resolve(host: &dyn Host, json: &Value) -> Result<CuttingRecipe, String> {
    let shape = shape(json)?;
    let input = ingredient(host, &shape.input).ok_or("input resolves to no items")?;
    let tools: Vec<_> = shape
        .tools
        .iter()
        .filter_map(|t| ingredient(host, t))
        .collect();
    if tools.is_empty() {
        return Err("tool resolves to no items".into());
    }
    let results = shape
        .results
        .iter()
        .map(|(item, chance)| {
            let stack = host
                .recipe_result(&item.to_string())
                .ok()
                .flatten()
                .ok_or_else(|| format!("unknown result {item}"))?;
            Ok(ChanceResult {
                stack,
                chance: *chance,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CuttingRecipe {
        id: String::new(),
        input,
        tools,
        results,
        sound: shape.sound,
    })
}

/// `ChanceResult.rollOutput`: each unit of the stack is lost when `next_float() > chance +
/// fortune_bonus * fortune`. Returns how many units remain.
pub fn roll_count(
    count: u32,
    chance: f32,
    fortune_bonus: f64,
    fortune: u32,
    mut next_float: impl FnMut() -> f32,
) -> u32 {
    let threshold = chance as f64 + fortune_bonus * fortune as f64;
    (0..count)
        .filter(|_| (next_float() as f64) <= threshold)
        .count() as u32
}

/// Outcome of looking up a recipe for a stored item and a tool.
#[derive(Debug, PartialEq)]
pub enum Lookup<T> {
    Found(T),
    /// Some recipe takes the item, but not with this tool.
    InvalidTool,
    /// No recipe takes the item.
    InvalidItem,
}

/// First recipe, in file order, whose input and tool both match.
pub fn find(
    table: &[CuttingRecipe],
    mut input: impl FnMut(&Ingredient) -> bool,
    mut tool: impl FnMut(&Ingredient) -> bool,
) -> Lookup<&CuttingRecipe> {
    let mut input_known = false;
    for recipe in table {
        if input(&recipe.input) {
            input_known = true;
            if recipe.tool_matches(&mut tool) {
                return Lookup::Found(recipe);
            }
        }
    }
    if input_known {
        Lookup::InvalidTool
    } else {
        Lookup::InvalidItem
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn fixture() -> Vec<(String, Value)> {
        let text = include_str!("../../tests/fixtures/cutting_recipes.json");
        let doc: Value = serde_json::from_str(text).unwrap();
        doc["cutting_recipes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["id"].as_str().unwrap().to_owned(), r["json"].clone()))
            .collect()
    }

    #[test]
    fn every_reference_recipe_has_a_valid_shape() {
        let all = fixture();
        assert_eq!(all.len(), 117);
        for (id, json) in &all {
            shape(json).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }

    #[test]
    fn tool_form_counts() {
        // inv§2: 51 knife tag, 25 axe_strip, 15 axe_dig|axes, 15 hoe_dig|hoes, 6 pickaxe_dig,
        // 3 shears_dig|shear tag, 2 shovel_dig.
        let mut forms: BTreeMap<String, usize> = BTreeMap::new();
        for (_, json) in fixture() {
            let alternatives = tool_alternatives(&json["tool"]).unwrap();
            let key = alternatives
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<Vec<_>>()
                .join("|");
            *forms.entry(key).or_default() += 1;
        }
        let expected: BTreeMap<String, usize> = [
            ("#c:tools/knife", 51),
            ("#minecraft:axes", 25),
            ("#minecraft:axes|#minecraft:axes", 15),
            ("#minecraft:hoes|#minecraft:hoes", 15),
            ("#minecraft:pickaxes", 6),
            ("#c:tools/shear|#c:tools/shear", 3),
            ("#minecraft:shovels", 2),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
        assert_eq!(forms, expected);
    }

    #[test]
    fn beef_cuts_into_two_minced_beef() {
        let beef = fixture()
            .into_iter()
            .find(|(id, _)| id == "farmersdelight:cutting/beef")
            .unwrap()
            .1;
        let s = shape(&beef).unwrap();
        assert_eq!(s.input, json!("minecraft:beef"));
        assert_eq!(s.tools, vec![json!("#c:tools/knife")]);
        assert_eq!(
            s.results,
            vec![(json!({"count": 2, "id": "farmersdelight:minced_beef"}), 1.0)]
        );
        assert_eq!(s.sound, None);
    }

    #[test]
    fn codec_limits() {
        let base = json!({"ingredients": ["a:b"], "tool": "#c:tools/knife", "result": [{"item": {"id": "a:c"}}]});
        assert!(shape(&base).is_ok());
        let mut two = base.clone();
        two["ingredients"] = json!(["a:b", "a:c"]);
        assert!(shape(&two).is_err());
        let mut none = base.clone();
        none["ingredients"] = json!([]);
        assert!(shape(&none).is_err());
        let mut five = base.clone();
        five["result"] = json!([{"item":{"id":"a:c"}},{"item":{"id":"a:c"}},{"item":{"id":"a:c"}},{"item":{"id":"a:c"}},{"item":{"id":"a:c"}}]);
        assert!(shape(&five).is_err());
        let mut chance = base.clone();
        chance["result"] = json!([{"item": {"id": "a:c"}, "chance": 0.5}]);
        assert_eq!(shape(&chance).unwrap().results[0].1, 0.5);
    }

    #[test]
    fn rolls_follow_chance_result() {
        assert_eq!(roll_count(2, 1.0, 0.1, 0, || 0.99), 2);
        assert_eq!(roll_count(3, 0.5, 0.1, 0, || 0.6), 0);
        // fortune 1 raises the threshold to 0.6 (Java: float roll promoted to double)
        assert_eq!(roll_count(3, 0.5, 0.1, 1, || 0.55), 3);
        assert_eq!(roll_count(3, 0.5, 0.1, 1, || 0.6), 0);
        let mut seq = [0.1, 0.9].into_iter().cycle();
        assert_eq!(roll_count(2, 0.5, 0.1, 0, || seq.next().unwrap()), 1);
    }

    fn ing(items: &[&str]) -> Ingredient {
        Ingredient::of(items.iter().map(|s| s.to_string()).collect())
    }

    fn recipe(id: &str, input: &str, tool: &str) -> CuttingRecipe {
        CuttingRecipe {
            id: id.into(),
            input: ing(&[input]),
            tools: vec![ing(&[tool])],
            results: vec![],
            sound: None,
        }
    }

    #[test]
    fn lookup_first_match_and_failure_kinds() {
        let table = vec![
            recipe("a", "beef", "knife"),
            recipe("b", "beef", "axe"),
            recipe("c", "beef", "knife"),
        ];
        let has = |s: &'static str| move |i: &Ingredient| i.items.contains(s);
        match find(&table, has("beef"), has("knife")) {
            Lookup::Found(r) => assert_eq!(r.id, "a"),
            other => panic!("{other:?}"),
        }
        match find(&table, has("beef"), has("axe")) {
            Lookup::Found(r) => assert_eq!(r.id, "b"),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            find(&table, has("beef"), has("stick")),
            Lookup::InvalidTool
        ));
        assert!(matches!(
            find(&table, has("dirt"), has("knife")),
            Lookup::InvalidItem
        ));
    }

    /// A recipes-only host (FakeHost has no recipe backend), as the machine sample's tests use.
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
                // "#empty" stands in for a tag with no items.
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
            let ids = ["farmersdelight:minced_beef", "minecraft:bone_meal"];
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
    fn build_keeps_file_order_and_skips_invalid_entries() {
        let beef = fixture()
            .into_iter()
            .find(|(id, _)| id == "farmersdelight:cutting/beef")
            .unwrap()
            .1;
        let mut empty_tool = beef.clone();
        empty_tool["tool"] = json!("#empty");
        let mut unknown_result = beef.clone();
        unknown_result["result"] = json!([{"item": {"id": "farmersdelight:nothing"}}]);
        let host = host::data(&[
            ("farmersdelight:cutting/beef", beef.clone()),
            ("test:empty_tool", empty_tool),
            ("test:unknown_result", unknown_result),
            (
                "test:two_inputs",
                json!({"ingredients": ["a:b", "a:c"], "tool": "#c:tools/knife", "result": [{"item": {"id": "minecraft:bone_meal"}}]}),
            ),
        ]);
        let (table, skipped) = build(&host);
        assert_eq!(table.len(), 1);
        let r = &table[0];
        assert_eq!(r.id, "farmersdelight:cutting/beef");
        assert!(r.input.items.contains("minecraft:beef"));
        assert!(r.tools[0].items.contains("#c:tools/knife"));
        assert_eq!(r.results[0].stack.count(), 2);
        assert_eq!(
            r.results[0].stack.item(),
            host::key("farmersdelight:minced_beef").unwrap()
        );
        let ids: Vec<_> = skipped.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            ids,
            ["test:empty_tool", "test:unknown_result", "test:two_inputs"]
        );
    }

    #[test]
    fn rebuild_replaces_overrides_and_removals() {
        let tables = crate::recipes::RecipeTables::default();
        let beef = fixture()
            .into_iter()
            .find(|(id, _)| id == "farmersdelight:cutting/beef")
            .unwrap()
            .1;
        tables.rebuild(&host::data(&[(
            "farmersdelight:cutting/beef",
            beef.clone(),
        )]));
        assert_eq!(tables.cutting.snapshot()[0].results[0].stack.count(), 2);
        // A data pack overriding the file: three minced beef.
        let mut three = beef;
        three["result"][0]["item"]["count"] = json!(3);
        tables.rebuild(&host::data(&[("farmersdelight:cutting/beef", three)]));
        assert_eq!(tables.cutting.snapshot()[0].results[0].stack.count(), 3);
        // ...and removing it.
        tables.rebuild(&host::data(&[]));
        assert!(tables.cutting.snapshot().is_empty());
    }
}
