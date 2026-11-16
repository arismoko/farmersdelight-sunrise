//! FD's two config files (J/common/Configuration.java, mlconfigs' Fabric holder): 19 common keys and 3 client
//! keys, kept as the reference keeps them on Fabric — a pretty-printed JSON object per file, one sub-object per
//! category — and loaded through the engine's config hooks.
//!
//! Load rules follow `FabricConfigHolder#forceLoad` and the `*ConfigValue#loadFromJson` bodies: a missing
//! category or key keeps the current value; a value that cannot be read keeps it too; a double outside its range
//! falls back to the default; a list that is not an array keeps its value; a list whose element cannot be read
//! as a string keeps the elements read before it. After each load the file is rewritten in the reference's
//! normalised form (`saveConfig`) when its text differs, so missing keys appear with their values.
//!
//! The common config is never synced to clients (quirk Q6): each side reads its own file.
//!
//! Startup-only keys (read once, when the game builds item defaults): `overrides.*` except
//! `enablePumpkinPieSneakToPlace` and `enableCuttingBoardDispenserBehavior`, and `overrides.stack_size.*`.
//! `crafting.enableVanillaCropCrates` and `world.generateFDChestLoot` act on the next data load (`/reload`).
//! Every other key is read where it is used, so an edit applies as soon as the game rereads the file.

use mod_api::serde_json::{self, Map, Value};
use std::sync::{LazyLock, RwLock};

/// The engine names a mod's files after its id (`ConfigPath::owned_by`), so FD's `farmersdelight-common.json`
/// becomes `ported_farmersdelight-common.json` (delta A45).
pub const COMMON_FILE: &str = "ported_farmersdelight-common.json";
pub const CLIENT_FILE: &str = "ported_farmersdelight-client.json";

/// `FabricConfigHolder#saveConfig`'s first property.
const README: &str = "This config file does not support comments. To see them configure it in-game using YACL or \
                      Cloth Config (or just use Forge)";

/// One value of a key.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Bool(bool),
    Double(f64),
    Str(String),
    List(Vec<String>),
}

/// A key: its category path below the file's root, its name, its default and (doubles) its range.
pub struct Key {
    pub path: &'static [&'static str],
    pub name: &'static str,
    default: fn() -> Val,
    range: (f64, f64),
}

const fn key(path: &'static [&'static str], name: &'static str, default: fn() -> Val) -> Key {
    Key {
        path,
        name,
        default,
        range: (0.0, 0.0),
    }
}

const fn ranged(path: &'static [&'static str], name: &'static str, default: fn() -> Val) -> Key {
    Key {
        path,
        name,
        default,
        range: (0.0, 1.0),
    }
}

fn strings(list: &[&str]) -> Val {
    Val::List(list.iter().map(|s| s.to_string()).collect())
}

const SETTINGS: &[&str] = &["settings"];
const FARMING: &[&str] = &["farming"];
const CRAFTING: &[&str] = &["crafting"];
const OVERRIDES: &[&str] = &["overrides"];
const STACK_SIZE: &[&str] = &["overrides", "stack_size"];
const WORLD: &[&str] = &["world"];
const DEBUG: &[&str] = &["debug"];
const CLIENT: &[&str] = &["client"];

/// The common keys in the reference's declaration order (which is also the file's order).
pub static COMMON_KEYS: [Key; 19] = [
    key(SETTINGS, "enableRopeReeling", || Val::Bool(true)),
    key(SETTINGS, "canvasSignDarkBackgroundList", || {
        strings(&["gray", "purple", "blue", "brown", "green", "red", "black"])
    }),
    ranged(FARMING, "richSoilBoostChance", || Val::Double(0.2)),
    key(FARMING, "enableTomatoVineClimbingTaggedRopes", || {
        Val::Bool(true)
    }),
    key(FARMING, "defaultTomatoVineRope", || {
        Val::Str("farmersdelight:rope".into())
    }),
    key(CRAFTING, "enableCookingPotRecipeBook", || Val::Bool(true)),
    key(CRAFTING, "enableVanillaCropCrates", || Val::Bool(true)),
    ranged(CRAFTING, "cuttingBoardFortuneBonus", || Val::Double(0.1)),
    key(OVERRIDES, "enableVanillaStrawBedRename", || Val::Bool(true)),
    key(OVERRIDES, "enableVanillaSoupExtraEffects", || {
        Val::Bool(true)
    }),
    key(OVERRIDES, "enableRabbitStewBuff", || Val::Bool(true)),
    key(OVERRIDES, "enablePumpkinPieSneakToPlace", || {
        Val::Bool(false)
    }),
    key(OVERRIDES, "enableCuttingBoardDispenserBehavior", || {
        Val::Bool(true)
    }),
    key(STACK_SIZE, "enableStackableSoupItems", || Val::Bool(true)),
    key(STACK_SIZE, "soupItemList", || {
        strings(&[
            "minecraft:mushroom_stew",
            "minecraft:beetroot_soup",
            "minecraft:rabbit_stew",
        ])
    }),
    key(WORLD, "generateFDChestLoot", || Val::Bool(true)),
    key(WORLD, "generateVillageCompostHeaps", || Val::Bool(true)),
    key(WORLD, "generateFDCropsOnVillageFarms", || Val::Bool(true)),
    key(DEBUG, "enableTomatoRopePermanence", || Val::Bool(true)),
];

pub static CLIENT_KEYS: [Key; 3] = [
    key(CLIENT, "enableNourishmentHungerOverlay", || Val::Bool(true)),
    key(CLIENT, "enableComfortHealthOverlay", || Val::Bool(true)),
    key(CLIENT, "enableFoodEffectTooltip", || Val::Bool(true)),
];

/// The current values of one file's keys, by index into its key table.
pub struct Values {
    keys: &'static [Key],
    values: RwLock<Vec<Val>>,
}

impl Values {
    fn new(keys: &'static [Key]) -> Self {
        Self {
            keys,
            values: RwLock::new(keys.iter().map(|k| (k.default)()).collect()),
        }
    }

    fn get(&self, index: usize) -> Val {
        self.values
            .read()
            .map(|v| v[index].clone())
            .unwrap_or_else(|_| (self.keys[index].default)())
    }

    fn bool(&self, index: usize) -> bool {
        match self.get(index) {
            Val::Bool(b) => b,
            _ => false,
        }
    }

    fn double(&self, index: usize) -> f64 {
        match self.get(index) {
            Val::Double(d) => d,
            _ => 0.0,
        }
    }

    fn list(&self, index: usize) -> Vec<String> {
        match self.get(index) {
            Val::List(l) => l,
            _ => Vec::new(),
        }
    }

    fn string(&self, index: usize) -> String {
        match self.get(index) {
            Val::Str(s) => s,
            _ => String::new(),
        }
    }

    /// The boolean key `name` (its last path segment, e.g. `generateFDChestLoot`), if this file has one.
    pub fn bool_named(&self, name: &str) -> Option<bool> {
        let index = self.keys.iter().position(|k| k.name == name)?;
        match self.get(index) {
            Val::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// Reads `text` over the current values and returns the normalised file text.
    pub fn load(&self, text: &str) -> String {
        let mut values = match self.values.write() {
            Ok(v) => v,
            Err(poisoned) => poisoned.into_inner(),
        };
        // FD's Gson read: a file that is not a JSON object changes nothing (and is then rewritten).
        if let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text) {
            load_into(self.keys, &mut values, &root);
        }
        render(self.keys, &values)
    }

    #[cfg(test)]
    fn reset(&self) {
        let mut values = self.values.write().unwrap();
        *values = self.keys.iter().map(|k| (k.default)()).collect();
    }
}

pub static COMMON: LazyLock<Values> = LazyLock::new(|| Values::new(&COMMON_KEYS));
pub static CLIENT_VALUES: LazyLock<Values> = LazyLock::new(|| Values::new(&CLIENT_KEYS));

/// The default text of a file: what the reference writes when the file is missing.
pub fn default_text(keys: &'static [Key]) -> String {
    render(
        keys,
        &keys.iter().map(|k| (k.default)()).collect::<Vec<_>>(),
    )
}

fn load_into(keys: &[Key], values: &mut [Val], root: &Map<String, Value>) {
    for (key, value) in keys.iter().zip(values.iter_mut()) {
        // ConfigSubCategory#loadFromJson: a missing or non-object category leaves its keys alone.
        let mut object = Some(root);
        for part in key.path {
            object = object.and_then(|o| o.get(*part)).and_then(Value::as_object);
        }
        let Some(entry) = object.and_then(|o| o.get(key.name)) else {
            continue;
        };
        let default = (key.default)();
        match default {
            Val::Bool(_) => {
                if let Some(b) = as_boolean(entry) {
                    *value = Val::Bool(b);
                }
            }
            Val::Double(d) => {
                if let Some(read) = as_double(entry) {
                    let (min, max) = key.range;
                    *value = Val::Double(if read >= min && read <= max { read } else { d });
                }
            }
            Val::Str(_) => {
                if let Some(s) = as_string(entry) {
                    *value = Val::Str(s);
                }
            }
            Val::List(_) => {
                // ListStringConfigValue: only an array replaces the value; an unreadable element stops the
                // read with the elements before it kept.
                if let Value::Array(items) = entry {
                    let mut list = Vec::new();
                    for item in items {
                        match as_string(item) {
                            Some(s) => list.push(s),
                            None => break,
                        }
                    }
                    *value = Val::List(list);
                }
            }
        }
    }
}

/// Gson's `JsonElement#getAsBoolean`: a one-element array reads its element; a string or number reads as
/// `Boolean.parseBoolean` of its text; null and objects cannot be read.
fn as_boolean(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(*b),
        Value::String(s) => Some(s.eq_ignore_ascii_case("true")),
        Value::Number(_) => Some(false),
        Value::Array(a) if a.len() == 1 => as_boolean(&a[0]),
        _ => None,
    }
}

/// Gson's `getAsDouble`: a number, or a string `Double.parseDouble` accepts.
fn as_double(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => parse_java_double(s),
        Value::Array(a) if a.len() == 1 => as_double(&a[0]),
        _ => None,
    }
}

/// `Double.parseDouble`: surrounding whitespace ignored, an optional `d`/`f` suffix, `NaN` and `Infinity`.
fn parse_java_double(text: &str) -> Option<f64> {
    let t = text.trim_matches(|c: char| c <= ' ');
    let t = t.strip_suffix(['d', 'D', 'f', 'F']).unwrap_or(t);
    match t.trim_start_matches(['+', '-']) {
        "NaN" => Some(f64::NAN),
        "Infinity" => Some(if t.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }),
        body if body
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-')) =>
        {
            t.parse().ok()
        }
        _ => None,
    }
}

/// Gson's `getAsString`: a string, a number's text or a boolean's.
fn as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Array(a) if a.len() == 1 => as_string(&a[0]),
        _ => None,
    }
}

/// `saveConfig`: Gson's pretty printing (two-space indent) of `#README` and the categories in declaration order.
fn render(keys: &[Key], values: &[Val]) -> String {
    // Build the category tree in first-seen order.
    struct Node {
        name: &'static str,
        entries: Vec<(&'static str, usize)>,
        children: Vec<Node>,
        order: Vec<bool>, // true = child node, false = entry, in insertion order
    }
    fn node<'a>(
        children: &'a mut Vec<Node>,
        order: &mut Vec<bool>,
        name: &'static str,
    ) -> &'a mut Node {
        if let Some(i) = children.iter().position(|n| n.name == name) {
            return &mut children[i];
        }
        children.push(Node {
            name,
            entries: Vec::new(),
            children: Vec::new(),
            order: Vec::new(),
        });
        order.push(true);
        children.last_mut().expect("just pushed")
    }
    let mut roots: Vec<Node> = Vec::new();
    let mut root_order = Vec::new();
    for (i, key) in keys.iter().enumerate() {
        let mut current = node(&mut roots, &mut root_order, key.path[0]);
        for part in &key.path[1..] {
            let Node {
                children, order, ..
            } = current;
            current = node(children, order, part);
        }
        current.entries.push((key.name, i));
        current.order.push(false);
    }
    fn write_node(out: &mut String, n: &Node, values: &[Val], depth: usize) {
        out.push_str("{\n");
        let (mut e, mut c) = (0, 0);
        for (k, is_child) in n.order.iter().enumerate() {
            if k > 0 {
                out.push_str(",\n");
            }
            indent(out, depth + 1);
            if *is_child {
                let child = &n.children[c];
                c += 1;
                out.push_str(&quote(child.name));
                out.push_str(": ");
                write_node(out, child, values, depth + 1);
            } else {
                let (name, index) = n.entries[e];
                e += 1;
                out.push_str(&quote(name));
                out.push_str(": ");
                write_value(out, &values[index], depth + 1);
            }
        }
        out.push('\n');
        indent(out, depth);
        out.push('}');
    }
    let mut out = String::from("{\n");
    indent(&mut out, 1);
    out.push_str(&quote("#README"));
    out.push_str(": ");
    out.push_str(&quote(README));
    for n in &roots {
        out.push_str(",\n");
        indent(&mut out, 1);
        out.push_str(&quote(n.name));
        out.push_str(": ");
        write_node(&mut out, n, values, 1);
    }
    out.push_str("\n}");
    out
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn write_value(out: &mut String, value: &Val, depth: usize) {
    match value {
        Val::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Val::Double(d) => out.push_str(&java_double(*d)),
        Val::Str(s) => out.push_str(&quote(s)),
        Val::List(list) if list.is_empty() => out.push_str("[]"),
        Val::List(list) => {
            out.push_str("[\n");
            for (i, s) in list.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                indent(out, depth + 1);
                out.push_str(&quote(s));
            }
            out.push('\n');
            indent(out, depth);
            out.push(']');
        }
    }
}

/// `Double.toString` for the values a ranged key can hold (finite).
fn java_double(d: f64) -> String {
    let a = d.abs();
    if d == 0.0 || (1e-3..1e7).contains(&a) {
        let s = format!("{d:?}");
        if s.contains('.') { s } else { format!("{s}.0") }
    } else {
        let s = format!("{d:e}");
        let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let mantissa = if mantissa.contains('.') {
            mantissa.to_string()
        } else {
            format!("{mantissa}.0")
        };
        format!("{mantissa}E{exp}")
    }
}

/// Gson's string escaping with HTML-safe output (its default).
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '<' | '>' | '&' | '=' | '\'' | '\u{2028}' | '\u{2029}' => {
                out.push_str(&format!("\\u{:04x}", c as u32))
            }
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// --- the keys, as their consumers read them -------------------------------------------------------------------

/// `settings.enableRopeReeling`.
pub fn rope_reeling_enabled() -> bool {
    COMMON.bool(0)
}

/// `settings.canvasSignDarkBackgroundList`: dye names whose canvas signs start with white text.
pub fn canvas_sign_dark_background_list() -> Vec<String> {
    COMMON.list(1)
}

/// `farming.richSoilBoostChance`.
pub fn rich_soil_boost_chance() -> f64 {
    COMMON.double(2)
}

/// `farming.enableTomatoVineClimbingTaggedRopes`.
pub fn tomato_vine_climbing_tagged_ropes() -> bool {
    COMMON.bool(3)
}

/// `farming.defaultTomatoVineRope`.
pub fn default_tomato_vine_rope() -> String {
    COMMON.string(4)
}

/// `crafting.enableCookingPotRecipeBook` (consumer: M9-02).
pub fn cooking_pot_recipe_book_enabled() -> bool {
    COMMON.bool(5)
}

/// `crafting.enableVanillaCropCrates`, read by the `vanilla_crates_enabled` load condition.
pub fn vanilla_crop_crates_enabled() -> bool {
    COMMON.bool(6)
}

/// `crafting.cuttingBoardFortuneBonus`.
pub fn cutting_board_fortune_bonus() -> f64 {
    COMMON.double(7)
}

/// `overrides.enableVanillaStrawBedRename` (startup).
pub fn vanilla_straw_bed_rename_enabled() -> bool {
    COMMON.bool(8)
}

/// `overrides.enableVanillaSoupExtraEffects` (startup for item defaults; the tooltip reads it live, M9-04).
pub fn vanilla_soup_extra_effects_enabled() -> bool {
    COMMON.bool(9)
}

/// `overrides.enableRabbitStewBuff` (startup).
pub fn rabbit_stew_buff_enabled() -> bool {
    COMMON.bool(10)
}

/// `overrides.enablePumpkinPieSneakToPlace`.
pub fn pumpkin_pie_sneak_to_place() -> bool {
    COMMON.bool(11)
}

/// `overrides.enableCuttingBoardDispenserBehavior`.
pub fn dispenser_tools_cutting_board() -> bool {
    COMMON.bool(12)
}

/// `overrides.stack_size.enableStackableSoupItems` (startup).
pub fn stackable_soup_items_enabled() -> bool {
    COMMON.bool(13)
}

/// `overrides.stack_size.soupItemList` (startup).
pub fn soup_item_list() -> Vec<String> {
    COMMON.list(14)
}

/// `world.generateFDChestLoot` (loot tables are modified as they load).
pub fn chest_loot_enabled() -> bool {
    COMMON.bool(15)
}

/// `world.generateVillageCompostHeaps`: read, but worldgen data cannot be switched (E-26, A26).
pub fn village_compost_heaps_enabled() -> bool {
    COMMON.bool(16)
}

/// `world.generateFDCropsOnVillageFarms`: read, but worldgen data cannot be switched (E-26, A26).
pub fn village_farm_fd_crops_enabled() -> bool {
    COMMON.bool(17)
}

/// `debug.enableTomatoRopePermanence`.
pub fn tomato_rope_permanence() -> bool {
    COMMON.bool(18)
}

/// `client.enableNourishmentHungerOverlay` (consumer: M9-03).
pub fn nourishment_hunger_overlay() -> bool {
    CLIENT_VALUES.bool(0)
}

/// `client.enableComfortHealthOverlay` (consumer: M9-03).
pub fn comfort_health_overlay() -> bool {
    CLIENT_VALUES.bool(1)
}

/// `client.enableFoodEffectTooltip` (consumer: M9-04).
pub fn food_effect_tooltip() -> bool {
    CLIENT_VALUES.bool(2)
}

// --- engine hooks ---------------------------------------------------------------------------------------------

/// `Hooks::register_configs`: both files, every time (each game reads only its own kinds).
pub fn register(mod_id: &str, configs: &mut Vec<mod_api::Config>) {
    use mod_api::{Config, ConfigKind};
    configs.push(
        Config::new(mod_id, ConfigKind::Common, &default_text(&COMMON_KEYS)).file(COMMON_FILE),
    );
    configs.push(
        Config::new(mod_id, ConfigKind::Client, &default_text(&CLIENT_KEYS)).file(CLIENT_FILE),
    );
}

/// `config_loading` / `config_reloading`: read the file over the current values, then write the normalised form
/// back when it differs (`saveConfig` after every load).
pub fn on_file(host: mod_api::HostSide<'_>, mod_id: &str, file: &mod_api::ConfigFile) {
    if file.mod_id != mod_id {
        return;
    }
    let values: &Values = match file.file.as_str() {
        COMMON_FILE => &COMMON,
        CLIENT_FILE => &CLIENT_VALUES,
        _ => return,
    };
    let normalised = values.load(&file.text);
    // A failed rewrite (read-only folder) leaves the file as the player wrote it; the values are loaded anyway.
    if normalised != file.text
        && let Ok(path) = mod_api::ConfigPath::new(&file.file)
    {
        let files = match host {
            mod_api::HostSide::Server(h) => h.config_files(),
            mod_api::HostSide::Client(h) => h.config_files(),
        };
        let _ = files.write(&path, normalised.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Values {
        Values::new(&COMMON_KEYS)
    }

    #[test]
    fn default_file_is_the_reference_layout() {
        let text = default_text(&COMMON_KEYS);
        let v: Value = serde_json::from_str(&text).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        for cat in [
            "#README",
            "settings",
            "farming",
            "crafting",
            "overrides",
            "world",
            "debug",
        ] {
            assert!(keys.contains(&cat.to_string()), "{cat}");
        }
        assert_eq!(
            v["overrides"]["stack_size"]["soupItemList"][2],
            "minecraft:rabbit_stew"
        );
        assert_eq!(v["farming"]["richSoilBoostChance"], 0.2);
        assert!(text.starts_with("{\n  \"#README\": \"This config"));
        assert!(text.contains("\n    \"richSoilBoostChance\": 0.2,\n"));
        assert!(text.contains("\"canvasSignDarkBackgroundList\": [\n      \"gray\",\n"));
        // Reading the default text changes nothing and renders it again.
        assert_eq!(fresh().load(&text), text);
    }

    #[test]
    fn values_are_read_with_the_reference_fallbacks() {
        let v = fresh();
        let out = v.load(
            r#"{"settings": {"enableRopeReeling": "TRUE", "canvasSignDarkBackgroundList": ["red", 5, null, "x"]},
                "farming": {"richSoilBoostChance": 1.5, "defaultTomatoVineRope": 7},
                "crafting": {"cuttingBoardFortuneBonus": "0.25d", "enableVanillaCropCrates": 1},
                "overrides": {"enablePumpkinPieSneakToPlace": [true], "stack_size": {"soupItemList": "nope"}},
                "world": "not an object", "unknown": {"a": 1}}"#,
        );
        assert_eq!(v.get(0), Val::Bool(true));
        assert_eq!(v.get(1), Val::List(vec!["red".into(), "5".into()]));
        assert_eq!(v.get(2), Val::Double(0.2), "out of range → default");
        assert_eq!(v.get(4), Val::Str("7".into()));
        assert_eq!(
            v.get(6),
            Val::Bool(false),
            "a number reads as parseBoolean(\"1\")"
        );
        assert_eq!(v.get(7), Val::Double(0.25));
        assert_eq!(v.get(11), Val::Bool(true));
        assert_eq!(
            v.get(14),
            (COMMON_KEYS[14].default)(),
            "a non-array keeps the list"
        );
        assert_eq!(
            v.get(15),
            Val::Bool(true),
            "a non-object category keeps its keys"
        );
        let reread: Value = serde_json::from_str(&out).unwrap();
        assert!(reread.get("unknown").is_none());
        assert_eq!(reread["world"]["generateFDChestLoot"], true);
        assert_eq!(reread["crafting"]["cuttingBoardFortuneBonus"], 0.25);
    }

    #[test]
    fn a_reload_keeps_values_it_cannot_read() {
        let v = fresh();
        v.load(r#"{"farming": {"richSoilBoostChance": 0.5}}"#);
        v.load(r#"{"farming": {"richSoilBoostChance": "half"}}"#);
        assert_eq!(v.get(2), Val::Double(0.5));
        v.load("not json");
        assert_eq!(v.get(2), Val::Double(0.5));
        v.reset();
        assert_eq!(v.get(2), Val::Double(0.2));
    }

    #[test]
    fn doubles_and_strings_print_as_gson_does() {
        assert_eq!(java_double(0.0), "0.0");
        assert_eq!(java_double(1.0), "1.0");
        assert_eq!(java_double(0.0005), "5.0E-4");
        assert_eq!(java_double(0.123), "0.123");
        let u = '\\';
        assert_eq!(quote("a=b<'"), format!("\"a{u}u003db{u}u003c{u}u0027\""));
    }
}
