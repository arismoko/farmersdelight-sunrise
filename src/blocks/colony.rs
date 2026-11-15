//! Mushroom colonies (PORT_PLAN M5-07): MushroomColonyBlock on the mushroom templates, which bring
//! MushroomBlock's survival (dim light on a solid block, or any light on
//! `#minecraft:overrides_mushroom_light_requirement`); growth, bone meal, harvest and the item's
//! placement at full age follow the reference here.
use super::tomato::{age, at, draw, item_stack, up};
use mod_api::*;

pub const BROWN: &str = "farmersdelight:brown_mushroom_colony";
pub const RED: &str = "farmersdelight:red_mushroom_colony";
const GROWS_COLONIES: &str = "farmersdelight:grows_mushroom_colonies";
const SHEARS: &str = "c:tools/shear";
const KNIVES: &str = "farmersdelight:tools/knives";
/// MushroomColonyBlock#getMaxAge.
const MAX_AGE: i32 = 3;

fn with_age(level: &dyn LevelHost, state: BlockState, age: i32) -> Option<BlockState> {
    level.state_with_property(state, "age", &age.to_string())
}

fn on_colony_ground(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    level
        .block_state(up(pos, -1))
        .is_some_and(|s| level.block_in_tag(s.block(), GROWS_COLONIES) == Some(true))
}

fn in_item_tag(host: &dyn Host, stack: &ItemStack, tag: &str) -> bool {
    let tags = host.tags();
    Identifier::parse(tag)
        .ok()
        .and_then(|id| tags.key(&id))
        .and_then(|key| tags.is_in_tag(stack.item(), key))
        == Some(true)
}

pub struct Colony {
    /// MushroomColonyBlock.mushroomType.
    mushroom: &'static str,
    block_particle: ParticleKey,
}

#[mod_api::horizon_part]
impl BlockPart for Colony {
    /// MushroomColonyBlock#randomTick: on `#grows_mushroom_colonies`, one in 4 grows a stage; no
    /// light check (Q9) and none of the template's spreading.
    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut RandomTickEvent) -> Flow {
        let pos = at(event.pos);
        let age = age(level, event.state);
        if age < MAX_AGE
            && on_colony_ground(level, pos)
            && draw(level, |r| r.next_int_bound(4).unwrap_or(1)) == Some(0)
            && let Some(next) = with_age(level, event.state, age + 1)
        {
            level.set_block(pos, next, BlockUpdateFlags::CLIENTS);
        }
        Flow::Cancel
    }

    /// MushroomColonyBlock#canSurvive: MushroomBlock's own rule (the template's), and any light
    /// on `#grows_mushroom_colonies`.
    fn can_survive(&self, level: &dyn LevelHost, event: &mut CanSurviveEvent) -> Option<bool> {
        on_colony_ground(level, at(event.pos)).then_some(true)
    }

    /// MushroomColonyItem#getPlacementState: placed fully grown.
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        Some(with_age(level, event.state, MAX_AGE))
    }

    fn is_valid_bonemeal_target(
        &self,
        level: LevelSide,
        event: &mut IsValidBonemealTargetEvent,
    ) -> Option<bool> {
        let (LevelSide::Server(level) | LevelSide::Client(level)) = level;
        Some(age(level, event.state) < MAX_AGE)
    }

    fn is_bonemeal_success(&self, _: &mut dyn LevelRandomHost, event: &mut IsBonemealSuccessEvent) {
        event.success = true;
    }

    /// MushroomColonyBlock#performBonemeal: + Mth.nextInt(1, 2) stages.
    fn perform_bonemeal(&self, level: &mut dyn LevelWriteHost, event: &mut PerformBonemealEvent) {
        let Some(inc) = draw(level, |r| 1 + r.next_int_bound(2).unwrap_or(0)) else {
            return;
        };
        let next = (age(level, event.state) + inc).min(MAX_AGE);
        if let Some(state) = with_age(level, event.state, next) {
            level.set_block(at(event.pos), state, BlockUpdateFlags::CLIENTS);
        }
    }

    /// MushroomColonyBlock#useItemOn: shears take one mushroom and a stage, a knife takes them all.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        let click = event.click;
        let held = click.stack?;
        let level_view = super::tomato::LevelOf(&*host);
        let colony_age = age(&level_view, event.state);
        if colony_age == 0 {
            return None;
        }
        let shears = in_item_tag(&*host, held, SHEARS);
        if !shears && !in_item_tag(&*host, held, KNIVES) {
            return None;
        }
        let (taken, left, sound, count, spread, speed) = if shears {
            (
                1,
                colony_age - 1,
                "minecraft:entity.sheep.shear",
                3,
                0.1,
                0.001,
            )
        } else {
            // `soundType.getBreakSound()` of the mushroom templates (SoundType.GRASS).
            (colony_age, 0, "minecraft:block.grass.break", 10, 0.2, 0.1)
        };
        // (`LevelOf` answers names only: the new state is parsed from its name.)
        let name = host.block_state_string(event.state)?;
        let next = host.block_state_id(&crate::items::skillet::set_prop(
            &name,
            "age",
            &left.to_string(),
        ))?;
        let level = host.level(click.dimension.key().id())?;
        let pos = click.pos;
        let centre = [
            pos[0] as f64 + 0.5,
            pos[1] as f64 + 0.5,
            pos[2] as f64 + 0.5,
        ];
        let _ = level.set_block(host, pos, next, BlockUpdateFlags::CLIENTS);
        level.play_sound(host, sound, centre, SoundSource::Blocks, 1.0, 1.0);
        if let Some(stack) = item_stack(&*host, self.mushroom, taken as u32) {
            let _ = level.pop_resource(host, pos, stack);
        }
        if let Some(player) = host.players().acting_player()
            && let Some(living) = host.living(player.entity())
        {
            let slot = match click.hand {
                Hand::Main => EquipmentSlot::MainHand,
                Hand::Off => EquipmentSlot::OffHand,
            };
            let _ = living.hurt_and_break(host, slot, 1);
        }
        level.send_particles(
            host,
            &ParticleOptions::simple(self.block_particle),
            Some(event.state),
            centre,
            [spread; 3],
            speed,
            count,
        );
        Some(InteractionResult::Success)
    }
}

pub fn register(registrar: &mut dyn Registrar) {
    let block_particle = registrar.particle_key("minecraft:block");
    for (block, mushroom) in [
        (BROWN, "minecraft:brown_mushroom"),
        (RED, "minecraft:red_mushroom"),
    ] {
        let key = registrar.block_key(block);
        registrar.block_part(
            key,
            Colony {
                mushroom,
                block_particle,
            },
        );
    }
}
