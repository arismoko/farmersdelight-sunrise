//! Feasts (J/common/block/FeastBlock.java, RotatedFeastBlock.java, GleamingSaladBlock.java,
//! RiceRollMedleyBlock.java; PORT_PLAN M7-01).
//!
//! A feast holds `servings` (4, the rice roll medley 8). A use takes one serving into the player's
//! inventory when the serving's crafting remainder (its bowl) is in that hand, or with anything in
//! hand when the serving has no remainder; at 0 servings a feast with leftovers lingers and the
//! next use breaks it (its loot table drops the bowl and bones), one without leftovers (stuffed
//! pumpkin) breaks as the last serving is taken. Outlines, light and placement are declared
//! (tools/decl/blocks.toml.in); the rest is here.
use crate::blocks::cutting_board::state_value;
use mod_api::*;

pub const ROAST_CHICKEN: &str = "farmersdelight:roast_chicken_block";
pub const STUFFED_PUMPKIN: &str = "farmersdelight:stuffed_pumpkin_block";
pub const HONEY_GLAZED_HAM: &str = "farmersdelight:honey_glazed_ham_block";
pub const SHEPHERDS_PIE: &str = "farmersdelight:shepherds_pie_block";
pub const GLEAMING_SALAD: &str = "farmersdelight:gleaming_salad_block";
pub const RICE_ROLL_MEDLEY: &str = "farmersdelight:rice_roll_medley_block";

pub const FEASTS: [&str; 6] = [
    ROAST_CHICKEN,
    STUFFED_PUMPKIN,
    HONEY_GLAZED_HAM,
    SHEPHERDS_PIE,
    GLEAMING_SALAD,
    RICE_ROLL_MEDLEY,
];

/// `RiceRollMedleyBlock#riceRollServings`, indexed by `servings - 1`.
const MEDLEY_SERVINGS: [&str; 8] = [
    "farmersdelight:cod_roll",
    "farmersdelight:cod_roll",
    "farmersdelight:salmon_roll",
    "farmersdelight:salmon_roll",
    "farmersdelight:salmon_roll",
    "farmersdelight:kelp_roll_slice",
    "farmersdelight:kelp_roll_slice",
    "farmersdelight:kelp_roll_slice",
];

const SOUND_SERVE: &str = "minecraft:item.armor.equip_generic";
const SOUND_LEFTOVERS: &str = "minecraft:block.wood.break";
const USE_CONTAINER: &str = "block.farmersdelight.feast.use_container";

// ---------------------------------------------------------------- pure rules

/// One feast block's constructor arguments (ModBlocks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kind {
    /// `hasLeftovers`: the block stays at 0 servings and breaks on the next use.
    pub leftovers: bool,
    /// `hasServingParticles`: three block particles each time a serving is taken.
    pub particles: bool,
}

pub fn kind(block: &str) -> Option<Kind> {
    match block {
        STUFFED_PUMPKIN => Some(Kind {
            leftovers: false,
            particles: true,
        }),
        ROAST_CHICKEN | HONEY_GLAZED_HAM | SHEPHERDS_PIE | GLEAMING_SALAD => Some(Kind {
            leftovers: true,
            particles: true,
        }),
        RICE_ROLL_MEDLEY => Some(Kind {
            leftovers: true,
            particles: false,
        }),
        _ => None,
    }
}

/// `getServingItem(state)`: the item a use takes from a feast that has `servings` (> 0) left.
pub fn serving_item(block: &str, servings: u32) -> Option<&'static str> {
    Some(match block {
        ROAST_CHICKEN => "farmersdelight:roast_chicken",
        STUFFED_PUMPKIN => "farmersdelight:stuffed_pumpkin",
        HONEY_GLAZED_HAM => "farmersdelight:honey_glazed_ham",
        SHEPHERDS_PIE => "farmersdelight:shepherds_pie",
        GLEAMING_SALAD => "farmersdelight:gleaming_salad",
        RICE_ROLL_MEDLEY => MEDLEY_SERVINGS.get((servings as usize).checked_sub(1)?)?,
        _ => return None,
    })
}

/// What one use does (`FeastBlock#takeServing`).
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No servings left: break the block with drops (leftovers).
    BreakLeftovers,
    /// Take a serving. `shrink`: one held container is used up; `vanish`: the block breaks after.
    Serve { shrink: bool, vanish: bool },
    /// The serving needs a container the hand does not hold: say so and pass on.
    NeedContainer,
}

/// `servings`: left before the use. `needs_container`: the serving has a crafting remainder;
/// `holds_it`: the hand's stack is that remainder's item.
pub fn decide(
    kind: Kind,
    servings: u32,
    needs_container: bool,
    holds_it: bool,
    creative: bool,
) -> Outcome {
    if servings == 0 {
        return Outcome::BreakLeftovers;
    }
    if needs_container && !holds_it {
        return Outcome::NeedContainer;
    }
    Outcome::Serve {
        shrink: needs_container && !creative,
        vanish: servings == 1 && !kind.leftovers,
    }
}

/// `level.getBlockState(pos.below())`, approximated as the cutting board's support (A9): the hook's
/// level host has no face-sturdiness query at the pin. `canSupportRigidBlock(below)`.
fn supported(level: &dyn LevelHost, pos: [i32; 3]) -> bool {
    crate::blocks::cutting_board::supported(level, pos)
}

fn servings_of(state: Option<String>) -> u32 {
    state
        .as_deref()
        .and_then(|n| state_value(n, "servings"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

// ---------------------------------------------------------------- the Part

pub struct Feast {
    pub block: &'static str,
    pub kind: Kind,
    /// `minecraft:block`, for `BlockParticleOption(ParticleTypes.BLOCK, state)`.
    pub block_particle: ParticleKey,
}

#[mod_api::horizon_part]
impl BlockPart for Feast {
    fn can_survive(&self, level: &dyn LevelHost, event: &mut CanSurviveEvent) -> Option<bool> {
        Some(supported(level, [event.pos.x, event.pos.y, event.pos.z]))
    }

    /// `updateShape`: unsupported from below, the feast becomes air (and drops, as vanilla's
    /// `updateOrDestroy` destroys it).
    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        if event.direction == Direction::Down && !supported(&*level, pos) {
            return level.block_state_id("minecraft:air");
        }
        None
    }

    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        Some(self.take_serving(host, event.click, event.state))
    }

    /// FeastBlock keeps Block's `useWithoutItem` (PASS): the template's menu never opens.
    fn use_without_item(
        &self,
        _: &mut dyn ServerHost,
        _: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        Some(InteractionResult::Pass)
    }

    fn has_analog_output(&self, _: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }

    /// `getAnalogOutputSignal`: the servings left (the medley's 8 is a signal of 8).
    fn analog_output(&self, level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = servings_of(level.state_string(event.state)) as i32;
    }

    /// `isPathfindable` is false for every type.
    fn is_pathfindable(&self, _: &dyn Host, event: &mut IsPathfindableEvent) {
        event.pathfindable = false;
    }
}

impl Feast {
    fn take_serving(
        &self,
        host: &mut dyn ServerHost,
        click: &BlockClick<'_>,
        state: BlockState,
    ) -> InteractionResult {
        let Some(player) = host.players().acting_player() else {
            return InteractionResult::Pass;
        };
        let Some(level) = host.level(click.dimension.key().id()) else {
            return InteractionResult::Pass;
        };
        let pos = click.pos;
        let at = [
            pos[0] as f64 + 0.5,
            pos[1] as f64 + 0.5,
            pos[2] as f64 + 0.5,
        ];
        let servings = servings_of(host.block_state_string(state));
        let serving = serving_item(self.block, servings).and_then(|id| stack_of(&*host, id));
        let container = serving
            .as_ref()
            .and_then(|s| host.item_crafting_remainder(s));
        let held = click.stack.cloned();
        let holds_it = matches!((&held, &container), (Some(h), Some(c)) if h.same_item(c));
        let creative = player.is_creative(&*host).unwrap_or(false);
        match decide(self.kind, servings, container.is_some(), holds_it, creative) {
            Outcome::BreakLeftovers => {
                level.play_sound(host, SOUND_LEFTOVERS, at, SoundSource::Players, 0.8, 0.8);
                let _ = level.destroy_block(host, pos, true, None, false);
                InteractionResult::Success
            }
            Outcome::NeedContainer => {
                if let Some(container) = &container {
                    let name = Text::translate(item_name_key(&*host, container), vec![]);
                    let _ = player.action_bar(host, Text::translate(USE_CONTAINER, vec![name]));
                }
                InteractionResult::TryWithEmptyHand
            }
            Outcome::Serve { shrink, vanish } => {
                let Some(serving) = serving else {
                    return InteractionResult::Pass;
                };
                host.set_block_property_with(pos, "servings", &(servings - 1).to_string(), 3);
                if shrink
                    && let Some(held) = held
                    && let Some(living) = host.living(player.entity())
                {
                    let _ = living.set_hand_item(host, click.hand, held.shrink(1));
                }
                let _ = host.transaction(|host, tx| {
                    crate::inventory::give_or_drop(host, tx, player, serving.clone());
                    Ok::<_, ()>(())
                });
                if vanish {
                    let _ = level.destroy_block(host, pos, true, None, false);
                }
                level.play_sound(host, SOUND_SERVE, at, SoundSource::Blocks, 1.0, 1.0);
                if self.kind.particles {
                    let _ = level.send_particles(
                        host,
                        &ParticleOptions::simple(self.block_particle),
                        Some(state),
                        at,
                        [0.1, 0.1, 0.1],
                        0.001,
                        3,
                    );
                }
                InteractionResult::Success
            }
        }
    }
}

fn stack_of(host: &dyn Host, id: &str) -> Option<ItemStack> {
    ItemStack::new(host.item_key(&Identifier::parse(id).ok()?)?, 1)
}

/// `getHoverName()` of a fresh stack: the item's translation key (`item.<ns>.<path>`).
fn item_name_key(host: &dyn Host, stack: &ItemStack) -> String {
    let id = stack.item().id(host);
    let id = id.to_string();
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", &id));
    format!("item.{ns}.{path}")
}

/// The client's prediction of a feast use (`useItemOn` runs `takeServing` on the client too and
/// answers SUCCESS when it acts). The client otherwise predicts the stonecutter template, whose
/// empty-hand use claims every main-hand click, so it would never send the off hand's click
/// (a bowl in the off hand). Server-side the block's Part answers; only the client is answered here.
pub fn predict(host: HostSide<'_>, event: &mut UseItemOnBlockEvent<'_>) -> Flow {
    let HostSide::Client(host) = host else {
        return Flow::Continue;
    };
    if event.phase != UsePhase::Block {
        return Flow::Continue;
    }
    let host: &dyn ClientHost = &*host;
    let Some(level) = host.level(host.dimension()) else {
        return Flow::Continue;
    };
    let Some(state) = level.block_state(host, event.click.pos) else {
        return Flow::Continue;
    };
    let name = host.block_state_string(state);
    let Some(block) = name.as_deref().and_then(|n| {
        let id = n.split('[').next().unwrap_or(n);
        FEASTS.into_iter().find(|f| *f == id)
    }) else {
        return Flow::Continue;
    };
    let Some(kind) = kind(block) else {
        return Flow::Continue;
    };
    let servings = servings_of(name);
    let container = serving_item(block, servings)
        .and_then(|id| stack_of(host, id))
        .and_then(|s| host.item_crafting_remainder(&s));
    let holds_it = matches!((event.click.stack, &container), (Some(h), Some(c)) if h.same_item(c));
    *event.cancellation_result =
        match decide(kind, servings, container.is_some(), holds_it, false) {
            // TRY_WITH_EMPTY_HAND, then Block's useWithoutItem (PASS): on to the item.
            Outcome::NeedContainer => InteractionResult::Pass,
            _ => InteractionResult::Success,
        };
    Flow::Cancel
}

// ---------------------------------------------------------------- gleaming salad sparkles

/// `GleamingSaladBlock#animateTick` (a sparkle with chance 0.05 × servings), from a client-ticking
/// block entity: the engine has no animate-tick hook for blocks without one (E-06). Vanilla asks
/// a block near the player about 0.0229 times a tick (`ClientLevel.animateTick`: 667 random cells
/// in 16 and 667 in 32 blocks each tick); each tick here asks with that chance.
pub struct SaladSparkle {
    pub sparkle: ParticleKey,
    pub client_ticks: std::sync::atomic::AtomicU64,
}

/// The chance a given block is picked by one `ClientLevel.animateTick` (radius 16 and 32 rounds).
pub const ANIMATE_CHANCE: f64 = 667.0 / 32_768.0 + 667.0 / 262_144.0;

#[mod_api::horizon_part]
impl BlockPart for SaladSparkle {}

#[mod_api::horizon_part]
impl BlockEntityPart for SaladSparkle {
    fn block_entity_client_ticks(
        &self,
        _: &dyn ClientHost,
        event: &mut BlockEntityClientTicksEvent<'_>,
    ) {
        event.ticks = true;
    }

    fn block_entity_client_tick(
        &self,
        host: &mut dyn ClientHost,
        event: &mut BlockEntityClientTickEvent<'_>,
    ) {
        let servings = servings_of(host.block_state_string(event.state));
        if servings == 0 {
            return;
        }
        let tick = self
            .client_ticks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut rng = crate::blocks::stove::ClientRng::new(event.pos, tick as i64);
        if rng.next() >= ANIMATE_CHANCE || rng.next() >= 0.05 * servings as f64 {
            return;
        }
        let pos = event.pos;
        let at = [
            pos[0] as f64 + 0.5 + (rng.next() * 0.5 - 0.25),
            pos[1] as f64 + 0.5,
            pos[2] as f64 + 0.5 + (rng.next() * 0.5 - 0.25),
        ];
        let _ = host.add_particle(&ParticleOptions::simple(self.sparkle), at, [0.0; 3]);
    }
}

// ---------------------------------------------------------------- items

/// `PlaceableItem#appendHoverText`: "Placeable", dark gray italic.
pub struct Placeable;

#[mod_api::horizon_part]
impl ItemPart for Placeable {
    fn tooltip(&self, _: &dyn ClientHost, event: &mut TooltipEvent<'_>) {
        let mut style = mod_api::text::TextStyle::default();
        style.color = Some("dark_gray".into());
        style.italic = Some(true);
        event.lines.push((
            Text::translate("tooltip.farmersdelight.placeable", vec![]).with_style(style),
            u32::MAX,
        ));
    }
}

pub fn register(registrar: &mut dyn Registrar) {
    let block_particle = registrar.particle_key("minecraft:block");
    for block in FEASTS {
        let key = registrar.block_key(block);
        registrar.block_part(
            key,
            Feast {
                block,
                kind: kind(block).expect("feast kind"),
                block_particle,
            },
        );
        let item = registrar.item_key(block);
        registrar.item_part(item, Placeable);
    }
    let salad = registrar
        .block_entity_type(GLEAMING_SALAD)
        .blocks(&[GLEAMING_SALAD])
        .build();
    let sparkle = registrar.particle_key("farmersdelight:sparkle");
    registrar
        .block_entity_part(
            &salad,
            SaladSparkle {
                sparkle,
                client_ticks: Default::default(),
            },
        )
        .expect("gleaming salad block entity type");
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Kind = Kind {
        leftovers: true,
        particles: true,
    };
    const PUMPKIN: Kind = Kind {
        leftovers: false,
        particles: true,
    };

    #[test]
    fn bowl_needed_unless_creative_keeps_it() {
        assert_eq!(decide(PLAIN, 4, true, false, false), Outcome::NeedContainer);
        assert_eq!(
            decide(PLAIN, 4, true, true, false),
            Outcome::Serve {
                shrink: true,
                vanish: false
            }
        );
        // Creative still needs the bowl in hand, but keeps it.
        assert_eq!(decide(PLAIN, 4, true, false, true), Outcome::NeedContainer);
        assert_eq!(
            decide(PLAIN, 4, true, true, true),
            Outcome::Serve {
                shrink: false,
                vanish: false
            }
        );
    }

    #[test]
    fn leftovers_linger_then_break() {
        assert_eq!(
            decide(PLAIN, 1, true, true, false),
            Outcome::Serve {
                shrink: true,
                vanish: false
            }
        );
        assert_eq!(decide(PLAIN, 0, true, false, false), Outcome::BreakLeftovers);
        assert_eq!(
            decide(PUMPKIN, 1, true, true, false),
            Outcome::Serve {
                shrink: true,
                vanish: true
            }
        );
    }

    #[test]
    fn medley_serves_in_reverse_order_with_any_hand() {
        let order: Vec<_> = (1..=8)
            .rev()
            .map(|s| serving_item(RICE_ROLL_MEDLEY, s).unwrap())
            .collect();
        assert_eq!(
            order,
            [
                "farmersdelight:kelp_roll_slice",
                "farmersdelight:kelp_roll_slice",
                "farmersdelight:kelp_roll_slice",
                "farmersdelight:salmon_roll",
                "farmersdelight:salmon_roll",
                "farmersdelight:salmon_roll",
                "farmersdelight:cod_roll",
                "farmersdelight:cod_roll",
            ]
        );
        assert_eq!(serving_item(RICE_ROLL_MEDLEY, 0), None);
        // Rolls have no remainder: served whatever the hand holds, nothing used up.
        let medley = kind(RICE_ROLL_MEDLEY).unwrap();
        assert_eq!(
            decide(medley, 8, false, false, false),
            Outcome::Serve {
                shrink: false,
                vanish: false
            }
        );
    }
}
