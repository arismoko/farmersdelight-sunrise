//! FD's HUD overlays (J/client/gui/HUDOverlays.java): gilded hunger icons over the food bar while the player has
//! Nourishment, and a sheen sweeping the bottom row of hearts while Comfort heals. Both are layers drawn right
//! above vanilla's health-and-food layer, each switched by its client config key.
//!
//! The overlay's jitter and sheen follow `Gui#getGuiTicks` (counted here, one per client tick) and a
//! `java.util.Random` seeded with `ticks * 312871`, as FD's.
use crate::effects::{self, COMFORT, NOURISHMENT};
use mod_api::*;
use std::sync::atomic::{AtomicU32, Ordering};

pub const NOURISHMENT_LAYER: &str = "farmersdelight:nourishment";
pub const COMFORT_LAYER: &str = "farmersdelight:comfort";
const ICONS: &str = "farmersdelight:gui/fd_icons";
/// `HUDOverlays.healthIconsOffset` / `foodIconsOffset`.
const ICONS_OFFSET: f32 = 39.0;

static GUI_TICKS: AtomicU32 = AtomicU32::new(0);

/// `Gui#tick`: one GUI tick per client tick.
pub fn tick() {
    GUI_TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn register(layers: &mut Vec<GuiLayer>) {
    for id in [COMFORT_LAYER, NOURISHMENT_LAYER] {
        layers.push(GuiLayer {
            id: id.into(),
            placement: GuiLayerPlacement::Above("minecraft:player_health".into()),
        });
    }
}

/// `java.util.Random`, for the overlays' jitter (seeded each frame as FD does).
struct JavaRandom(i64);

impl JavaRandom {
    fn new(seed: i64) -> Self {
        Self((seed ^ 0x5DEECE66D) & ((1 << 48) - 1))
    }
    fn next(&mut self, bits: u32) -> i32 {
        self.0 = (self.0.wrapping_mul(0x5DEECE66D).wrapping_add(0xB)) & ((1 << 48) - 1);
        (self.0 >> (48 - bits)) as i32
    }
    fn next_int(&mut self, bound: i32) -> i32 {
        if bound & -bound == bound {
            return ((bound as i64 * self.next(31) as i64) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let val = bits % bound;
            if bits - val + (bound - 1) >= 0 {
                return val;
            }
        }
    }
}

struct View {
    player: PlayerRef,
    living: LivingRef,
    ticks: i32,
}

/// `BaseOverlay#shouldRenderOverlay`: a player whose game mode can be hurt (the HUD is not drawn when hidden).
fn view(host: &dyn ClientHost) -> Option<View> {
    let player = host.local_player()?;
    let mode = player.game_mode(host)?;
    if !matches!(mode, GameMode::Survival | GameMode::Adventure) {
        return None;
    }
    let living = host.living(player.entity())?;
    Some(View {
        player,
        living,
        ticks: GUI_TICKS.load(Ordering::Relaxed) as i32,
    })
}

fn has(host: &dyn ClientHost, living: LivingRef, id: &str) -> bool {
    effects::key(id).is_some_and(|key| living.effect(host, key).is_some())
}

pub fn draw(host: &dyn ClientHost, event: &mut GuiLayerDrawEvent<'_>) {
    match event.layer {
        NOURISHMENT_LAYER if crate::config::nourishment_hunger_overlay() => {
            nourishment(host, event.draw)
        }
        COMFORT_LAYER if crate::config::comfort_health_overlay() => comfort(host, event.draw),
        _ => {}
    }
}

/// `NourishmentOverlay#render` + `drawNourishmentOverlay`.
fn nourishment(host: &dyn ClientHost, draw: &mut dyn DrawContext) {
    let Some(v) = view(host) else {
        return;
    };
    if !has(host, v.living, NOURISHMENT) {
        return;
    }
    let Some(food) = v.player.food(host) else {
        return;
    };
    let health = v.living.health(host).unwrap_or(0.0);
    let max = v.living.max_health(host).unwrap_or(20.0);
    // `naturalRegenerationEnabled` is assumed on (delta A15).
    let healing = health < max && food.level >= 18;
    let [w, h] = draw.size();
    let right = (w / 2.0).floor() + 91.0;
    let top = h - ICONS_OFFSET;
    let mut rand = JavaRandom::new(v.ticks as i64 * 312871);
    for j in 0..10 {
        let x = right - j as f32 * 8.0 - 9.0;
        let mut y = top;
        if food.saturation <= 0.0 && v.ticks % (food.level * 3 + 1) == 0 {
            y = top + (rand.next_int(3) - 1) as f32;
        }
        draw.sprite(ICONS, [0.0, 0.0, 9.0, 9.0], [x, y, 9.0, 9.0], None);
        let effective = food.level as f32 / 2.0 - j as f32;
        let offset = if healing { 18.0 } else { 0.0 };
        if effective >= 1.0 {
            draw.sprite(
                ICONS,
                [18.0 + offset, 0.0, 9.0, 9.0],
                [x, y, 9.0, 9.0],
                None,
            );
        } else if effective >= 0.5 {
            draw.sprite(ICONS, [9.0 + offset, 0.0, 9.0, 9.0], [x, y, 9.0, 9.0], None);
        }
    }
}

/// `ComfortOverlay#render` + `drawComfortOverlay`.
fn comfort(host: &dyn ClientHost, draw: &mut dyn DrawContext) {
    let Some(v) = view(host) else {
        return;
    };
    let Some(food) = v.player.food(host) else {
        return;
    };
    let health_now = v.living.health(host).unwrap_or(0.0);
    let health_max = v.living.max_health(host).unwrap_or(20.0);
    let regeneration = has(host, v.living, "minecraft:regeneration");
    let eligible = food.saturation == 0.0 && health_now < health_max && !regeneration;
    if !has(host, v.living, COMFORT) || !eligible {
        return;
    }
    let [w, h] = draw.size();
    let left = (w / 2.0).floor() - 91.0;
    let top = h - ICONS_OFFSET;
    let ticks = v.ticks;
    let mut rand = JavaRandom::new(ticks as i64 * 312871);
    let health = health_now.ceil() as i32;
    // The absorption amount is not readable on the client (delta A50): counted as 0.
    let absorb = 0.0_f32;
    let regen = if regeneration { ticks % 25 } else { -1 };
    let rows = ((health_max + absorb) / 2.0 / 10.0).ceil() as i32;
    let row_height = (10 - (rows - 2)).max(3);
    let sheen = ticks % 50;
    let frame = sheen % 2;
    let widths = [5.0, 9.0];
    let single_row = (health_max.min(20.0) / 2.0).ceil() as i32;
    let bottom_row = ((rows - 1) * row_height) as f32;
    for i in 0..single_row {
        let column = i % 10;
        let x = left + column as f32 * 8.0;
        let mut y = top + bottom_row;
        if health <= 4 {
            y += rand.next_int(2) as f32;
        }
        if i == regen {
            y -= 2.0;
        }
        if column == sheen / 2 {
            let width = widths[frame as usize];
            draw.sprite(ICONS, [0.0, 9.0, width, 9.0], [x, y, width, 9.0], None);
        }
        if column == sheen / 2 - 1 && frame == 0 {
            draw.sprite(ICONS, [5.0, 9.0, 4.0, 9.0], [x + 5.0, y, 4.0, 9.0], None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JavaRandom;

    #[test]
    fn java_random_matches_the_jdk() {
        // Values printed by the system JDK's java.util.Random.
        let mut r = JavaRandom::new(42);
        assert_eq!([r.next_int(3), r.next_int(3), r.next_int(3)], [2, 0, 0]);
        assert_eq!(JavaRandom::new(312871).next_int(2), 1);
        let mut q = JavaRandom::new(7 * 312871);
        assert_eq!([q.next_int(3), q.next_int(2), q.next_int(5)], [1, 1, 1]);
    }
}
