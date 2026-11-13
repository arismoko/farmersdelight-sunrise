//! Tool ingredients of `farmersdelight:cutting` recipes, including the Fabric custom forms the
//! reference writes (`fabric:any`, `farmersdelight:item_ability`). Pure: no host calls here.
use mod_api::serde_json::Value;

/// `J/refabricated/ItemAbility.java:canPerformAction` — each ability is membership of one tag.
pub fn ability_tag(action: &str) -> Option<&'static str> {
    Some(match action {
        "shears_carve" | "shears_harvest" | "shears_dig" => "#c:tools/shear",
        "sword_dig" => "#minecraft:swords",
        "shovel_dig" => "#minecraft:shovels",
        "pickaxe_dig" => "#minecraft:pickaxes",
        "axe_dig" | "axe_strip" => "#minecraft:axes",
        "hoe_dig" => "#minecraft:hoes",
        // CommonTags.Items.TOOLS_KNIFE
        "knife_dig" | "knife_harvest" => "#c:tools/knife",
        _ => return None,
    })
}

/// Flatten a tool ingredient into plain vanilla ingredient JSON values, any one of which
/// accepts the tool (`fabric:any` is a union; an ability is its tag). Plain values (an id, a
/// `#tag` or a list of them) pass through unchanged.
pub fn tool_alternatives(tool: &Value) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    flatten(tool, &mut out)?;
    if out.is_empty() {
        return Err("empty tool ingredient".into());
    }
    Ok(out)
}

fn flatten(value: &Value, out: &mut Vec<Value>) -> Result<(), String> {
    let Some(kind) = value.get("fabric:type") else {
        out.push(value.clone());
        return Ok(());
    };
    match kind.as_str() {
        Some("fabric:any") => {
            let list = value["ingredients"]
                .as_array()
                .ok_or("fabric:any without an ingredients list")?;
            list.iter().try_for_each(|child| flatten(child, out))
        }
        Some("farmersdelight:item_ability") => {
            let action = value["action"]
                .as_str()
                .ok_or("item_ability without action")?;
            let tag =
                ability_tag(action).ok_or_else(|| format!("unknown item ability {action}"))?;
            out.push(Value::String(tag.into()));
            Ok(())
        }
        other => Err(format!("unsupported custom ingredient {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mod_api::serde_json::json;

    #[test]
    fn plain_tag_passes_through() {
        assert_eq!(
            tool_alternatives(&json!("#c:tools/knife")).unwrap(),
            vec![json!("#c:tools/knife")]
        );
    }

    #[test]
    fn ability_and_any_flatten() {
        let any = json!({"fabric:type": "fabric:any", "ingredients": [
            {"action": "axe_dig", "fabric:type": "farmersdelight:item_ability"}, "#minecraft:axes"]});
        assert_eq!(
            tool_alternatives(&any).unwrap(),
            vec![json!("#minecraft:axes"), json!("#minecraft:axes")]
        );
        let strip = json!({"action": "axe_strip", "fabric:type": "farmersdelight:item_ability"});
        assert_eq!(
            tool_alternatives(&strip).unwrap(),
            vec![json!("#minecraft:axes")]
        );
    }

    #[test]
    fn unknown_forms_are_refused() {
        assert!(
            tool_alternatives(&json!({"fabric:type": "farmersdelight:tool_action", "action": "x"}))
                .is_err()
        );
        assert!(
            tool_alternatives(
                &json!({"fabric:type": "farmersdelight:item_ability", "action": "fly"})
            )
            .is_err()
        );
        assert!(
            tool_alternatives(&json!({"fabric:type": "fabric:any", "ingredients": []})).is_err()
        );
    }
}
