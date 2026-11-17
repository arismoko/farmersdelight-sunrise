//! Organic compost (PORT_PLAN M5-06): OrganicCompostBlock. Eight composting stages, then rich soil.
//! Its mycelium motes (`animateTick`) are not drawn: no client tick for a block without a block
//! entity (delta A5, E-06).
use super::tomato::{at, draw};
use crate::blocks::cutting_board::state_value;
use mod_api::*;

pub const ID: &str = "farmersdelight:organic_compost";
const ACTIVATORS: &str = "farmersdelight:compost_activators";
/// OrganicCompostBlock#getMaxCompostingStage.
const MAX_STAGE: i32 = 7;

fn stage(level: &dyn LevelHost, state: BlockState) -> i32 {
    level
        .state_string(state)
        .and_then(|s| state_value(&s, "composting").and_then(|v| v.parse().ok()))
        .unwrap_or(0)
}

/// The chance of one random tick advancing a stage: 0.02 per `#compost_activators` block in the
/// 3×3×3 around it, 0.1 if any neighbour has sky light above 12 over it (else 0.05), 0.1 with
/// water among them.
pub fn chance(activators: u32, max_sky_light: i32, has_water: bool) -> f32 {
    let mut chance = 0.0f32;
    for _ in 0..activators {
        chance += 0.02;
    }
    chance += if max_sky_light > 12 { 0.1 } else { 0.05 };
    chance += if has_water { 0.1 } else { 0.0 };
    chance
}

pub struct OrganicCompost;

#[crate::engine::part]
impl BlockPart for OrganicCompost {
    /// OrganicCompostBlock#randomTick.
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> Flow {
        let pos = at(event.pos);
        let (mut activators, mut has_water, mut max_light) = (0u32, false, 0);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let p = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
                    if let Some(s) = level.block_state(p) {
                        if level.block_in_tag(s.block(), ACTIVATORS) == Some(true) {
                            activators += 1;
                        }
                        if level
                            .state_fluid_state(s)
                            .is_some_and(|f| crate::blocks::rope::is_water(level, f))
                        {
                            has_water = true;
                        }
                    }
                    max_light =
                        max_light.max(level.brightness(LightLayer::Sky, [p[0], p[1] + 1, p[2]]));
                }
            }
        }
        let chance = chance(activators, max_light, has_water);
        if draw(level, |r| r.next_float()).is_some_and(|f| f <= chance) {
            let stage = stage(level, event.state);
            let next = if stage >= MAX_STAGE {
                level.block_state_id(super::rich_soil::SOIL)
            } else {
                level.state_with_property(event.state, "composting", &(stage + 1).to_string())
            };
            if let Some(next) = next {
                level.set_block(
                    pos,
                    next,
                    BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
                );
            }
        }
        Flow::Cancel
    }

    fn has_analog_output(&self, _: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }

    /// OrganicCompostBlock#getAnalogOutputSignal: 8 at the first stage down to 1 at the last.
    fn analog_output(&self, level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = MAX_STAGE + 1 - stage(level, event.state);
    }
}

pub fn register(registrar: &mut dyn Registrar) {
    let compost = registrar.block_key(ID);
    registrar.block_part(compost, OrganicCompost);
}

#[cfg(test)]
mod tests {
    use super::chance;

    #[test]
    fn chance_follows_the_reference_terms() {
        assert!((chance(0, 0, false) - 0.05).abs() < 1e-6);
        assert!((chance(0, 13, false) - 0.1).abs() < 1e-6);
        assert!((chance(0, 12, true) - 0.15).abs() < 1e-6);
        assert!((chance(5, 15, true) - 0.3).abs() < 1e-6);
        assert!((chance(27, 15, true) - 0.74).abs() < 1e-5);
    }
}
