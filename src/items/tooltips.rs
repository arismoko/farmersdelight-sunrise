//! Food tooltips (ConsumableItem / DogFoodItem / HorseFeedItem `appendHoverText`, TextUtils.addFoodEffectTooltip,
//! TooltipEvents.addTooltipToVanillaSoups), all behind the client key `enableFoodEffectTooltip`.
use super::food_tooltips::{CUSTOM_TOOLTIPS, EFFECT_TOOLTIPS};
use mod_api::text::TextStyle;
use mod_api::*;

const KEEP: u32 = u32::MAX;

fn styled(text: Text, color: &str) -> Text {
    let mut style = TextStyle::default();
    style.color = Some(color.into());
    text.with_style(style)
}

/// `MobEffectCategory#getTooltipFormatting`: harmful red, beneficial and neutral blue.
fn category_color(effect: &str) -> &'static str {
    match effect {
        "minecraft:hunger"
        | "minecraft:nausea"
        | "minecraft:poison"
        | "minecraft:slowness"
        | "minecraft:weakness"
        | "minecraft:mining_fatigue"
        | "minecraft:blindness"
        | "minecraft:wither"
        | "minecraft:instant_damage"
        | "minecraft:unluck"
        | "minecraft:bad_omen"
        | "minecraft:darkness" => "red",
        _ => "blue",
    }
}

/// `MobEffect#getDescriptionId`.
fn effect_name(effect: &str) -> Text {
    let (ns, path) = effect.split_once(':').unwrap_or(("minecraft", effect));
    Text::translate(format!("effect.{ns}.{path}"), vec![])
}

/// `StringUtil.formatTickDuration` at 20 ticks per second: `mm:ss`, or `hh:mm:ss` past an hour.
pub fn format_duration(ticks: i32) -> String {
    let seconds = ticks / 20;
    let (hours, minutes, secs) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

/// `addFoodEffectTooltip`'s lines for these effects.
pub fn effect_lines(effects: &[(&str, i32, i32)]) -> Vec<Text> {
    let mut lines = Vec::new();
    let mut modifiers: Vec<(&str, i32, f64)> = Vec::new();
    for &(id, duration, amplifier) in effects {
        let mut text = effect_name(id);
        // Effect::createModifiers for the effects FD foods give: Speed (+20% per level, multiplied total)
        // and Absorption (+4 max absorption per level).
        match id {
            "minecraft:speed" => {
                modifiers.push(("movement_speed", 2, 0.2 * (amplifier + 1) as f64 * 100.0))
            }
            "minecraft:absorption" => {
                modifiers.push(("max_absorption", 0, 4.0 * (amplifier + 1) as f64))
            }
            _ => {}
        }
        if amplifier > 0 {
            text = Text::translate(
                "potion.withAmplifier",
                vec![
                    text,
                    Text::translate(format!("potion.potency.{amplifier}"), vec![]),
                ],
            );
        }
        if duration > 20 {
            text = Text::translate(
                "potion.withDuration",
                vec![text, Text::literal(format_duration(duration))],
            );
        }
        lines.push(styled(text, category_color(id)));
    }
    if !modifiers.is_empty() {
        lines.push(Text::literal(""));
        lines.push(styled(
            Text::translate("potion.whenDrank", vec![]),
            "dark_purple",
        ));
        for (attribute, operation, amount) in modifiers {
            // ItemAttributeModifiers.ATTRIBUTE_MODIFIER_FORMAT: `#.##`.
            let shown = format!("{:.2}", amount)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
            lines.push(styled(
                Text::translate(
                    format!("attribute.modifier.plus.{operation}"),
                    vec![
                        Text::literal(shown),
                        Text::translate(format!("attribute.name.{attribute}"), vec![]),
                    ],
                ),
                "blue",
            ));
        }
    }
    lines
}

/// The pet foods' list: a gray heading, then ` <effect>[ <potency>][ (<duration>)]`.
fn pet_lines(item: &str, effects: &[(&str, i32, i32)]) -> Vec<Text> {
    let heading = match item {
        super::foods::DOG_FOOD => "tooltip.farmersdelight.dog_food.when_feeding",
        _ => "tooltip.farmersdelight.horse_feed.when_feeding",
    };
    let mut lines = vec![styled(Text::translate(heading, vec![]), "gray")];
    for &(id, duration, amplifier) in effects {
        let mut text = Text::literal(" ").append(effect_name(id));
        if amplifier > 0 {
            text = text.append(Text::literal(" ")).append(Text::translate(
                format!("potion.potency.{amplifier}"),
                vec![],
            ));
        }
        if duration > 20 {
            text = text.append(Text::literal(format!(" ({})", format_duration(duration))));
        }
        lines.push(styled(text, category_color(id)));
    }
    lines
}

/// An FD food's own lines (`appendHoverText`).
pub struct FoodTooltip {
    item: &'static str,
}

#[mod_api::horizon_part]
impl ItemPart for FoodTooltip {
    fn tooltip(&self, _: &dyn ClientHost, event: &mut TooltipEvent<'_>) {
        if !crate::config::food_effect_tooltip() {
            return;
        }
        let lines = if CUSTOM_TOOLTIPS.contains(&self.item) {
            let path = self.item.trim_start_matches("farmersdelight:");
            vec![styled(
                Text::translate(format!("tooltip.farmersdelight.{path}"), vec![]),
                "blue",
            )]
        } else if let Some((_, effects)) = EFFECT_TOOLTIPS.iter().find(|(id, _)| *id == self.item) {
            effect_lines(effects)
        } else if let Some((_, effects, _, _)) = super::foods::pet_food(self.item) {
            pet_lines(self.item, effects)
        } else {
            return;
        };
        event.lines.extend(lines.into_iter().map(|l| (l, KEEP)));
    }
}

pub fn register(registrar: &mut dyn Registrar) {
    let items = CUSTOM_TOOLTIPS
        .iter()
        .copied()
        .chain(EFFECT_TOOLTIPS.iter().map(|(id, _)| *id))
        .chain([super::foods::DOG_FOOD, super::foods::HORSE_FEED]);
    for item in items {
        let key = registrar.item_key(item);
        registrar.item_part(key, FoodTooltip { item });
    }
}

/// `TooltipEvents#addTooltipToVanillaSoups`, at the end of the tooltip: the pumpkin pie's "Placeable" line (or
/// "Placeable when sneaking"), then, with soup effects and the food tooltip on, the soups' Nourishment line.
pub fn vanilla(host: &dyn ClientHost, event: &mut ItemTooltipEvent<'_>) {
    let item = event.stack.item().id(host);
    if item == "minecraft:pumpkin_pie" {
        let key = if crate::config::pumpkin_pie_sneak_to_place() {
            "tooltip.farmersdelight.placeable_sneaking"
        } else {
            "tooltip.farmersdelight.placeable"
        };
        let mut style = TextStyle::default();
        style.color = Some("dark_gray".into());
        style.italic = Some(true);
        event
            .lines
            .push((Text::translate(key, vec![]).with_style(style), KEEP));
    }
    if !crate::config::vanilla_soup_extra_effects_enabled() || !crate::config::food_effect_tooltip()
    {
        return;
    }
    if let Some(duration) = super::foods::soup_nourishment(&item) {
        let text = Text::translate(
            "potion.withDuration",
            vec![
                effect_name(crate::effects::NOURISHMENT),
                Text::literal(format_duration(duration)),
            ],
        );
        event.lines.push((styled(text, "blue"), KEEP));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_format_as_vanilla() {
        assert_eq!(format_duration(1200), "01:00");
        assert_eq!(format_duration(6000), "05:00");
        assert_eq!(format_duration(72000), "01:00:00");
        assert_eq!(format_duration(30), "00:01");
    }

    #[test]
    fn absorption_adds_the_when_applied_section() {
        let lines = effect_lines(&[("minecraft:absorption", 1200, 0)]);
        assert_eq!(lines.len(), 4);
        let speed = effect_lines(&[("minecraft:speed", 1200, 1)]);
        assert_eq!(speed.len(), 4);
        assert_eq!(
            effect_lines(&[("farmersdelight:nourishment", 3600, 0)]).len(),
            1
        );
    }
}
