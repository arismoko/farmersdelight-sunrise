//! Rope, rope fence and rope fence gate (PORT_PLAN M5-02).
//!
//! The rope is declared on the iron-bars template (RopeBlock extends IronBarsBlock): its sides,
//! waterlogging, outline and horizontal side rule (`IronBarsBlock.attachsTo`, which FD's
//! `tieToAnythingValid` equals) are the template's. Code adds what RopeBlock/RopeItem change:
//! placement on a vertical face, `tied_to_bell`, reeling, bell ringing and rope-on-rope extension.
use mod_api::*;

pub const ROPE: &str = "farmersdelight:rope";
pub const ROPE_FENCE: &str = "farmersdelight:rope_fence";
pub const ROPE_FENCE_GATE: &str = "farmersdelight:rope_fence_gate";
const BELL: &str = "minecraft:bell";
/// RopeBlock#useWithoutItem: the bell search goes up at most 24 blocks.
const BELL_SEARCH: usize = 24;

fn name_of(level: &dyn LevelHost, state: BlockState) -> Option<&str> {
    level.state_block(state)
}

fn is_rope(level: &dyn LevelHost, state: BlockState) -> bool {
    name_of(level, state) == Some(ROPE)
}

/// `state.getBlock() instanceof IronBarsBlock || state.is(BlockTags.WALLS)` (RopeBlock#tieToRopeAndWalls):
/// iron and copper bars, glass panes and rope are the vanilla/FD IronBarsBlock classes.
fn ties_to_rope_and_walls(level: &dyn LevelHost, state: BlockState) -> bool {
    let Some(name) = name_of(level, state) else {
        return false;
    };
    name == ROPE
        || name.ends_with("_bars")
        || name.ends_with("glass_pane")
        || level.block_in_tag(state.block(), "minecraft:walls") == Some(true)
}

/// Whether `fluid` is water, source or flowing (`FluidTags.WATER`'s vanilla members).
fn is_water(level: &dyn LevelHost, fluid: FluidState) -> bool {
    ["minecraft:water", "minecraft:water[level=1]"]
        .iter()
        .any(|n| {
            level
                .block_state_id(n)
                .and_then(|s| level.state_fluid_state(s))
                .is_some_and(|w| w.fluid == fluid.fluid)
        })
}

fn is_water_host(host: &dyn Host, fluid: FluidState) -> bool {
    ["minecraft:water", "minecraft:water[level=1]"]
        .iter()
        .any(|n| {
            host.block_state_id(n)
                .and_then(|s| host.state_fluid_state(s))
                .is_some_and(|w| w.fluid == fluid.fluid)
        })
}

fn min_y(host: &dyn Host, level: LevelRef) -> i32 {
    level.height(host).map_or(-64, |(min, _)| min)
}

fn offset(pos: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]]
}

fn bool_str(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

/// RopeBlock#getStateWithConnections for a top/bottom face: sides tie only to rope, bars,
/// panes and walls; `tied_to_bell` from the block above; waterlogged in source water.
fn vertical_state(
    level: &dyn LevelHost,
    base: BlockState,
    pos: [i32; 3],
    water: bool,
) -> Option<BlockState> {
    let mut state = base;
    for (name, d) in [
        ("north", [0, 0, -1]),
        ("south", [0, 0, 1]),
        ("west", [-1, 0, 0]),
        ("east", [1, 0, 0]),
    ] {
        let tie = level
            .block_state(offset(pos, d))
            .is_some_and(|s| ties_to_rope_and_walls(level, s));
        state = level.state_with_property(state, name, bool_str(tie))?;
    }
    let bell = level
        .block_state(offset(pos, [0, 1, 0]))
        .and_then(|s| name_of(level, s))
        == Some(BELL);
    state = level.state_with_property(state, "tied_to_bell", bool_str(bell))?;
    level.state_with_property(state, "waterlogged", bool_str(water))
}

/// The rope a tomato vine leaves (HangingTomatoBlock#placeRope with the default
/// `defaultTomatoVineRope`, farmersdelight:rope): `getStateWithConnections(…, Direction.UP)`.
pub fn rope_state_for_vine(level: &dyn LevelHost, pos: [i32; 3]) -> Option<BlockState> {
    let base = level.block_state_id(ROPE)?;
    let water = level
        .fluid_state(pos)
        .is_some_and(|f| f.source && is_water(level, f));
    vertical_state(level, base, pos, water)
}

pub struct Rope {
    /// `minecraft:bell`, resolved at registration for its block event.
    pub bell: BlockKey,
}

#[mod_api::horizon_part]
impl BlockPart for Rope {
    /// RopeBlock#getStateForPlacement. A click on a side face uses the template's own placement
    /// (vanilla's attachsTo, identical to FD's horizontal rule, and the sturdy-face check only the
    /// engine can make); a click on a top or bottom face ties only to rope, bars, panes and walls.
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let face = event.context.face;
        if !matches!(face, Direction::Up | Direction::Down) {
            return None;
        }
        let p = event.context.pos;
        // `fluidState.getType() == Fluids.WATER`: source water only.
        let water = event.fluid.source && is_water(level, event.fluid);
        Some(vertical_state(level, event.state, [p.x, p.y, p.z], water))
    }

    /// RopeBlock#updateShape: the block above sets `tied_to_bell`; the sides stay the template's.
    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        if event.direction != Direction::Up {
            return None;
        }
        let bell = name_of(level, event.neighbour) == Some(BELL);
        let current = level
            .state_string(event.state)
            .is_some_and(|s| s.contains("tied_to_bell=true"));
        if bell == current {
            return None;
        }
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        if level
            .state_string(event.state)
            .is_some_and(|s| s.contains("waterlogged=true"))
        {
            // Fluids.WATER.getTickDelay: 5 (the template's own update would have scheduled it).
            level.schedule_fluid_tick(pos, "minecraft:water", 5);
        }
        level.state_with_property(event.state, "tied_to_bell", bool_str(bell))
    }

    /// RopeBlock#useItemOn: holding rope passes to the item (extension, `RopeItem` below);
    /// anything else goes on to the empty-hand use, as the template's default does.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        let rope = event
            .click
            .stack
            .is_some_and(|s| s.item().id(&*host) == ROPE);
        Some(if rope {
            InteractionResult::Pass
        } else {
            InteractionResult::TryWithEmptyHand
        })
    }

    /// RopeBlock#useWithoutItem: sneaking reels the lowest rope of the column up into the
    /// inventory (config `enableRopeReeling`); otherwise a bell up the rope (≤ 24 blocks) rings.
    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        let click = event.click;
        let level = host.level(click.dimension.key().id())?;
        let player = host.players().acting_player()?;
        let sneaking = player.is_secondary_use_active(&*host) == Some(true);
        if crate::config::rope_reeling_enabled() && sneaking {
            return Some(reel(host, level, player, click.pos));
        }
        let mut pos = offset(click.pos, [0, 1, 0]);
        for _ in 0..BELL_SEARCH {
            let Some(state) = level.block_state(&*host, pos) else {
                return Some(InteractionResult::Pass);
            };
            let name = host.block_state_string(state).unwrap_or_default();
            if name.starts_with(BELL) {
                ring_bell(host, level, player, pos, &name, self.bell);
                return Some(InteractionResult::Success);
            }
            if !name.starts_with(ROPE) || name.starts_with(ROPE_FENCE) {
                return Some(InteractionResult::Pass);
            }
            pos = offset(pos, [0, 1, 0]);
        }
        Some(InteractionResult::Pass)
    }

    /// RopeBlock#isPathfindable: always.
    fn is_pathfindable(&self, _: &dyn Host, event: &mut IsPathfindableEvent) {
        event.pathfindable = true;
    }
}

/// The reeling half of RopeBlock#useWithoutItem: the player must be able to build and (outside
/// creative) take one rope into the inventory; the lowest rope of the column below is destroyed
/// without drops.
fn reel(
    host: &mut dyn ServerHost,
    level: LevelRef,
    player: PlayerRef,
    clicked: [i32; 3],
) -> InteractionResult {
    if player.may_build(&*host) != Some(true) {
        return InteractionResult::Pass;
    }
    if player.is_creative(&*host) != Some(true) {
        let Some(one) = Identifier::parse(ROPE)
            .ok()
            .and_then(|id| host.item_key(&id))
            .and_then(|item| ItemStack::new(item, 1))
        else {
            return InteractionResult::Pass;
        };
        let added = host
            .transaction(|host, tx| {
                let inserted = player
                    .entity()
                    .capability(host, &ENTITY_ITEMS, ())
                    .map_or(0, |inv| {
                        inv.insert_any(host, tx, &ItemResource::of(&one), 1)
                    });
                if inserted == 1 { Ok(()) } else { Err(()) }
            })
            .is_ok();
        if !added {
            // Inventory.add failed: nothing happens (the reference returns PASS).
            return InteractionResult::Pass;
        }
    }
    let min_y = min_y(&*host, level);
    let mut pos = offset(clicked, [0, -1, 0]);
    while pos[1] >= min_y - 1 {
        let below_is_rope = level
            .block_state(&*host, pos)
            .and_then(|s| host.block_state_string(s))
            .is_some_and(|n| n == ROPE || n.starts_with("farmersdelight:rope["));
        if below_is_rope {
            pos = offset(pos, [0, -1, 0]);
            continue;
        }
        let lowest = offset(pos, [0, 1, 0]);
        let _ = level.destroy_block(host, lowest, false, None, false);
        return InteractionResult::Consume;
    }
    InteractionResult::Pass
}

/// BellBlock#attemptToRing(level, pos, facing.getClockWise()): the bell's block event 1 with the
/// direction's 3D value, its sound at volume 2.0 and a block-change game event.
fn ring_bell(
    host: &mut dyn ServerHost,
    level: LevelRef,
    player: PlayerRef,
    pos: [i32; 3],
    name: &str,
    bell: BlockKey,
) {
    // facing.getClockWise(): north→east, east→south, south→west, west→north;
    // Direction.get3DDataValue: down 0, up 1, north 2, south 3, west 4, east 5.
    let facing = crate::blocks::cutting_board::state_value(name, "facing").unwrap_or("north");
    let data = match facing {
        "north" => 5,
        "east" => 3,
        "south" => 4,
        _ => 2,
    };
    let _ = level.block_event(host, pos, bell, 1, data);
    let at = [
        pos[0] as f64 + 0.5,
        pos[1] as f64 + 0.5,
        pos[2] as f64 + 0.5,
    ];
    level.play_sound(
        host,
        "minecraft:block.bell.use",
        at,
        SoundSource::Blocks,
        2.0,
        1.0,
    );
    let affected_state = level.block_state(&*host, pos);
    let _ = level.game_event(
        host,
        "minecraft:block_change",
        at,
        GameEventContext {
            source_entity: Some(player.entity()),
            affected_state,
        },
    );
}

/// RopeItem#updatePlacementContext: rope used on rope goes to the first free block below the
/// column (or, sneaking, one block out of the clicked face). Other clicks place as a block item.
pub struct RopeItem;

#[mod_api::horizon_part]
impl ItemPart for RopeItem {
    fn use_on(&self, host: &mut dyn ServerHost, event: &mut UseOnEvent<'_>) -> Flow {
        let Some(player) = host.players().acting_player() else {
            return Flow::Continue;
        };
        let Some(level) = player.level(&*host) else {
            return Flow::Continue;
        };
        let is_rope_at = |host: &dyn Host, pos: [i32; 3]| {
            level
                .block_state(host, pos)
                .and_then(|s| host.block_state_string(s))
                .is_some_and(|n| n.starts_with("farmersdelight:rope[") || n == ROPE)
        };
        // BlockItem#place: the clicked block if the rope may replace it (rope only), else the
        // block beside the clicked face.
        let start = if is_rope_at(&*host, event.pos) {
            event.pos
        } else {
            offset(event.pos, event.face)
        };
        if !is_rope_at(&*host, start) {
            return Flow::Continue;
        }
        let sneaking = player.is_secondary_use_active(&*host) == Some(true);
        let step = if sneaking { event.face } else { [0, -1, 0] };
        let min_y = min_y(&*host, level);
        let mut pos = offset(start, step);
        while pos[1] >= min_y {
            if is_rope_at(&*host, pos) {
                if step != [0, -1, 0] {
                    return Flow::Cancel;
                }
                pos = offset(pos, step);
                continue;
            }
            if let Some(fluid) = level.fluid_state(&*host, pos)
                && !fluid.is_empty()
                && !is_water_host(&*host, fluid)
            {
                return Flow::Cancel;
            }
            place_rope(host, level, player, event, pos);
            return Flow::Cancel;
        }
        Flow::Cancel
    }
}

/// BlockItem#place at the moved context: the rope's placement state for a click on `step`'s
/// face, the wool place sound, one rope used outside creative.
fn place_rope(
    host: &mut dyn ServerHost,
    level: LevelRef,
    player: PlayerRef,
    event: &UseOnEvent<'_>,
    pos: [i32; 3],
) {
    let replaceable = level.block_state(&*host, pos).is_some_and(|s| {
        host.block_state_string(s).is_some_and(|n| {
            n.starts_with("minecraft:air")
                || n.starts_with("minecraft:cave_air")
                || n.starts_with("minecraft:void_air")
        }) || host.block_in_tag(s.block(), "minecraft:replaceable") == Some(true)
    });
    if !replaceable || player.may_build(&*host) != Some(true) {
        return;
    }
    let Some(state) = rope_state_at(host, level, pos) else {
        return;
    };
    if level.set_block(
        host,
        pos,
        state,
        BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
    ) != Ok(true)
    {
        return;
    }
    let at = [
        pos[0] as f64 + 0.5,
        pos[1] as f64 + 0.5,
        pos[2] as f64 + 0.5,
    ];
    // SoundType.WOOL place: volume (1.0 + 1) / 2, pitch 1.0 × 0.8.
    level.play_sound(
        host,
        "minecraft:block.wool.place",
        at,
        SoundSource::Blocks,
        1.0,
        0.8,
    );
    if player.is_creative(&*host) != Some(true)
        && let Some(living) = host.living(player.entity())
    {
        let _ = living.set_hand_item(host, event.hand, event.stack.clone().shrink(1));
    }
}

/// The state an extended rope takes: the vertical-face rule (the moved context's clicked face
/// is the step's, DOWN when not sneaking) — ties to rope, bars, panes and walls, the bell above,
/// water.
fn rope_state_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<BlockState> {
    let mut name = String::from("farmersdelight:rope[");
    let mut parts = Vec::new();
    for (side, d) in [
        ("east", [1, 0, 0]),
        ("north", [0, 0, -1]),
        ("south", [0, 0, 1]),
        ("west", [-1, 0, 0]),
    ] {
        let tie = level.block_state(host, offset(pos, d)).is_some_and(|s| {
            host.block_state_string(s).is_some_and(|n| {
                n.starts_with(ROPE) && !n.starts_with(ROPE_FENCE)
                    || n.split('[')
                        .next()
                        .is_some_and(|b| b.ends_with("_bars") || b.ends_with("glass_pane"))
            }) || host.block_in_tag(s.block(), "minecraft:walls") == Some(true)
        });
        parts.push(format!("{side}={}", bool_str(tie)));
    }
    let bell = level
        .block_state(host, offset(pos, [0, 1, 0]))
        .and_then(|s| host.block_state_string(s))
        .is_some_and(|n| n.starts_with(BELL));
    parts.push(format!("tied_to_bell={}", bool_str(bell)));
    let water = level
        .fluid_state(host, pos)
        .is_some_and(|f| f.source && is_water_host(host, f));
    parts.push(format!("waterlogged={}", bool_str(water)));
    name.push_str(&parts.join(","));
    name.push(']');
    host.block_state_id(&name)
}

/// RopeFenceBlock#connectsTo and RopeFenceConnectionMixin, as one `connects_to` part on
/// `#minecraft:fences`: a rope fence joins only sturdy faces, rope fences and rope fence gates
/// across its arm (vanilla's rule minus other fences and gates); other fences never join a rope
/// fence or a rope fence gate.
pub struct FenceRule;

#[mod_api::horizon_part]
impl BlockPart for FenceRule {
    fn connects_to(&self, level: &dyn LevelHost, event: &mut ConnectsToEvent) {
        let own = name_of(level, event.state) == Some(ROPE_FENCE);
        let other = name_of(level, event.neighbour).unwrap_or("");
        if own {
            let fence =
                level.block_in_tag(event.neighbour.block(), "minecraft:fences") == Some(true);
            let gate =
                level.block_in_tag(event.neighbour.block(), "minecraft:fence_gates") == Some(true);
            if (fence && other != ROPE_FENCE) || (gate && other != ROPE_FENCE_GATE) {
                event.connected = false;
            }
        } else if other == ROPE_FENCE || other == ROPE_FENCE_GATE {
            event.connected = false;
        }
    }
}

/// RopeFenceGateBlock#updateShape: `in_wall` only when walls border both sides of the gate's
/// axis (vanilla: either side).
pub struct RopeFenceGate;

#[mod_api::horizon_part]
impl BlockPart for RopeFenceGate {
    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut UpdateShapeEvent,
    ) -> Option<BlockState> {
        let name = level.state_string(event.state)?;
        let facing = crate::blocks::cutting_board::state_value(&name, "facing")?;
        let side_axis_x = matches!(facing, "north" | "south");
        let along = match event.direction {
            Direction::East | Direction::West => side_axis_x,
            Direction::North | Direction::South => !side_axis_x,
            _ => return None,
        };
        if !along {
            return None;
        }
        let n = event.direction.normal();
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        let opposite = level.block_state([pos[0] - n[0], pos[1] - n[1], pos[2] - n[2]]);
        let is_wall =
            |s: BlockState| level.block_in_tag(s.block(), "minecraft:walls") == Some(true);
        let in_wall = is_wall(event.neighbour) && opposite.is_some_and(is_wall);
        level.state_with_property(event.state, "in_wall", bool_str(in_wall))
    }

    fn fire_odds(&self, _: &dyn LevelHost, _: &mut FireOddsEvent) -> Option<(i32, i32)> {
        Some((0, 0))
    }
}

/// FD registers no flammability for the rope fence (the oak-fence template's is not kept).
pub struct NotFlammable;

#[mod_api::horizon_part]
impl BlockPart for NotFlammable {
    fn fire_odds(&self, _: &dyn LevelHost, _: &mut FireOddsEvent) -> Option<(i32, i32)> {
        Some((0, 0))
    }
}

pub fn register(registrar: &mut dyn Registrar) {
    let rope = registrar.block_key(ROPE);
    let bell = registrar.block_key(BELL);
    registrar.block_part(rope, Rope { bell });
    let item = registrar.item_key(ROPE);
    registrar.item_part(item, RopeItem);
    registrar.block_part_tag("minecraft:fences", FenceRule);
    let gate = registrar.block_key(ROPE_FENCE_GATE);
    registrar.block_part(gate, RopeFenceGate);
    let fence = registrar.block_key(ROPE_FENCE);
    registrar.block_part(fence, NotFlammable);
}
