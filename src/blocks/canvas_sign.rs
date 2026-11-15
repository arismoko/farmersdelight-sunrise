//! Canvas signs' text colour (SignBlockEntityMixin + CanvasSign#isDarkBackground): a sign whose board dye is in
//! config `canvasSignDarkBackgroundList` starts with white text.
//!
//! The declarations carry the default list as a static `sign_text_color` (every placement path gets it). A
//! player's placement re-reads the config and sets both sides' colour with `data merge`, so an edited list holds
//! for signs players place; signs made another way (commands, structures) keep the default list (delta A35).
use mod_api::*;

const DYES: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

/// The board dye of a canvas sign block (`None` for the plain board), or `None` twice for any other block.
fn board_dye(block: &str) -> Option<Option<&'static str>> {
    let path = block.strip_prefix("farmersdelight:")?;
    if !path.contains("canvas") || !path.ends_with("sign") {
        return None;
    }
    Some(
        DYES.iter()
            .copied()
            .find(|d| path.strip_prefix(d).is_some_and(|r| r.starts_with('_'))),
    )
}

/// `entity_place`: a player's canvas sign gets the text colour of the current list.
pub fn on_placed(host: &mut dyn ServerHost, event: &mut EntityPlaceEvent) {
    if event.placer.is_none() {
        return;
    }
    let Some(name) = host.block_state_string(event.state) else {
        return;
    };
    let block = name.split('[').next().unwrap_or(&name);
    let Some(dye) = board_dye(block) else {
        return;
    };
    let dark = dye.is_some_and(|d| {
        crate::config::canvas_sign_dark_background_list()
            .iter()
            .any(|e| e == d)
    });
    let color = if dark { "white" } else { "black" };
    let [x, y, z] = [event.pos.x, event.pos.y, event.pos.z];
    let dimension = host.dimension();
    // The sign's block entity is made with the declared colour after this notice, so the colour is set at the
    // tick's end.
    if let Ok(mut queue) = PENDING.lock() {
        queue.push(format!(
            "execute in {dimension} run data merge block {x} {y} {z} \
             {{front_text:{{color:\"{color}\"}},back_text:{{color:\"{color}\"}}}}"
        ));
    }
}

static PENDING: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Server tick end: the colours of this tick's placements.
pub fn drain(host: &mut dyn ServerHost) {
    let lines = match PENDING.lock() {
        Ok(mut queue) if !queue.is_empty() => std::mem::take(&mut *queue),
        _ => return,
    };
    for line in lines {
        let _ = host.commands().run_command(&line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dye_of_each_form() {
        assert_eq!(board_dye("farmersdelight:canvas_sign"), Some(None));
        assert_eq!(
            board_dye("farmersdelight:wall_hanging_canvas_sign"),
            Some(None)
        );
        assert_eq!(
            board_dye("farmersdelight:light_blue_canvas_wall_sign"),
            Some(Some("light_blue"))
        );
        assert_eq!(
            board_dye("farmersdelight:blue_wall_hanging_canvas_sign"),
            Some(Some("blue"))
        );
        assert_eq!(board_dye("farmersdelight:rope"), None);
        assert_eq!(board_dye("minecraft:oak_sign"), None);
    }
}
