//! Rotten tomato (J/common/item/RottenTomatoItem.java, J/common/entity/RottenTomatoEntity.java).
//! The entity type is declared on the snowball template (`tools/decl/entities.toml.in`, damage 0:
//! the hit is `thrown` damage of 0, so knockback and anger still happen); here the throw with FD's
//! sound, the dispenser's throw, and FD's hit sounds.
use mod_api::*;

pub const ITEM: &str = "farmersdelight:rotten_tomato";
pub const ENTITY: &str = "farmersdelight:rotten_tomato";
const THROW: &str = "farmersdelight:entity.rotten_tomato.throw";
const HIT: &str = "farmersdelight:entity.rotten_tomato.hit";

pub struct RottenTomato;

pub fn register(registrar: &mut dyn Registrar) {
    let key = registrar.item_key(ITEM);
    registrar.item_part(key, RottenTomato);
}

/// `0.4F / (random.nextFloat() * 0.4F + 0.8F)`.
pub fn throw_pitch(r: f32) -> f32 {
    0.4 / (r * 0.4 + 0.8)
}

/// `(random.nextFloat() - random.nextFloat()) * 0.2F + 1.0F`.
pub fn hit_pitch(a: f32, b: f32) -> f32 {
    (a - b) * 0.2 + 1.0
}

/// `Projectile.shoot` for a dispenser: `(x, y + 0.1, z)` normalised, plus triangular noise
/// `0.0172275 × inaccuracy` per axis, times `power`.
pub fn dispense_motion(step: [f64; 3], power: f64, inaccuracy: f64, noise: [f64; 3]) -> [f64; 3] {
    let v = [step[0], step[1] + 0.1, step[2]];
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let spread = 0.0172275 * inaccuracy;
    std::array::from_fn(|i| (v[i] / len + noise[i] * spread) * power)
}

fn step_of(facing: Direction) -> [f64; 3] {
    match facing {
        Direction::North => [0.0, 0.0, -1.0],
        Direction::South => [0.0, 0.0, 1.0],
        Direction::West => [-1.0, 0.0, 0.0],
        Direction::East => [1.0, 0.0, 0.0],
        Direction::Up => [0.0, 1.0, 0.0],
        Direction::Down => [0.0, -1.0, 0.0],
    }
}

#[crate::engine::part]
impl ItemPart for RottenTomato {
    /// `RottenTomatoItem#use`: the throw sound, a projectile at 1.5 with 1.0 spread, the use
    /// counted, one used up outside creative. SUCCESS on the client, CONSUME on the server.
    fn use_item(
        &self,
        host: HostSide<'_>,
        event: &mut UseItemEvent<'_>,
    ) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return Some(InteractionResult::Success);
        };
        let player = host.players().acting_player()?;
        let level = player.level(&*host)?;
        let living = host.living(player.entity())?;
        let at = player.entity().position(&*host)?;
        let r = level.random(&*host).map_or(0.5, |mut r| r.next_float());
        level.play_sound(
            host,
            THROW,
            at,
            SoundSource::Neutral,
            0.5,
            throw_pitch(r),
            None,
        );
        if let Ok(kind) = Identifier::parse(ENTITY) {
            let _ = host.spawn_projectile_from_rotation(living, &kind, event.stack, 0.0, 1.5, 1.0);
        }
        player.award_stat(host, StatType::Used, ITEM, 1);
        if player.is_creative(&*host) != Some(true) {
            let _ = living.set_hand_item(host, event.hand, event.stack.clone().shrink(1));
        }
        Some(InteractionResult::Consume)
    }

    /// `ProjectileItem` + `ProjectileDispenseBehavior` (default config: power 1.1, uncertainty
    /// 6): thrown from 0.7 in front of the dispenser's centre, the launch sound (1002).
    fn dispense_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut DispenseItemEvent<'_>,
    ) -> Option<Dispensed> {
        let level = host.level(event.dimension)?;
        let step = step_of(event.facing);
        let centre = [
            event.pos[0] as f64 + 0.5 + 0.7 * step[0],
            event.pos[1] as f64 + 0.5 + 0.7 * step[1],
            event.pos[2] as f64 + 0.5 + 0.7 * step[2],
        ];
        // RandomSource.triangle(0, 1): nextDouble − nextDouble (level random, A43).
        let noise = level.random(&*host).map_or([0.0; 3], |mut r| {
            std::array::from_fn(|_| r.next_double() - r.next_double())
        });
        let motion = dispense_motion(step, 1.1, 6.0, noise);
        let mut nbt = Compound::new();
        nbt.insert(
            "Motion".into(),
            Tag::List(motion.iter().map(|m| Tag::Double(m.to_bits())).collect()),
        );
        if let Some(one) = event
            .stack
            .clone()
            .split(1)
            .taken
            .and_then(|s| s.to_nbt(&*host))
        {
            nbt.insert("Item".into(), Tag::Compound(one));
        }
        host.summon(event.dimension, ENTITY, centre, nbt)?;
        Some((event.stack.clone().shrink(1), 1002))
    }
}

/// `onHitEntity` + `onHit`: a hit sound for the entity hit, then one for the impact itself.
pub fn on_impact(host: &mut dyn ServerHost, event: &mut ProjectileImpactEvent<'_>) {
    if event.kind != ENTITY {
        return;
    }
    let Some(level) = host.level(event.dimension) else {
        return;
    };
    let sounds = if event.entity.is_some() { 2 } else { 1 };
    for _ in 0..sounds {
        let (a, b) = level
            .random(&*host)
            .map_or((0.5, 0.5), |mut r| (r.next_float(), r.next_float()));
        level.play_sound(
            host,
            HIT,
            event.at,
            SoundSource::Neutral,
            1.0,
            hit_pitch(a, b),
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitches() {
        assert!((throw_pitch(0.0) - 0.5).abs() < 1e-6);
        assert!((throw_pitch(1.0) - 0.4 / 1.2).abs() < 1e-6);
        assert!((hit_pitch(1.0, 0.0) - 1.2).abs() < 1e-6);
    }

    #[test]
    fn dispensed_straight_ahead_with_a_lift() {
        let m = dispense_motion([1.0, 0.0, 0.0], 1.1, 6.0, [0.0; 3]);
        let len = (1.0f64 + 0.01).sqrt();
        assert!((m[0] - 1.1 / len).abs() < 1e-9);
        assert!((m[1] - 0.11 / len).abs() < 1e-9);
        assert_eq!(m[2], 0.0);
    }
}
