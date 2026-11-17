//! Feasts for add-on blocks (BX-04): `FeastBlock#takeServing` and the rest of FD 26.3's
//! `FeastBlock` behaviour, the same code FD's own six feasts run.
//!
//! A feast block is declared by the add-on (a `servings` integer property, `facing`, shapes,
//! loot); [`register`] attaches the feast Part to it in the add-on's `register` hook:
//!
//! ```ignore
//! fn nachos(_: u32) -> Option<&'static str> { Some("corn_delight:nachos_bowl") }
//! ported_farmersdelight::api::feast::register(registrar, "corn_delight:nachos_block", FeastSpec {
//!     serving_item: nachos,
//!     has_leftovers: true,
//!     serving_particles: true,
//!     servings_property: "servings",
//! });
//! ```
//!
//! The Part answers a use with [`take_serving`] (serving needs the serving's crafting remainder in
//! that hand unless it has none, creative keeps the container, leftovers linger at 0 and break on
//! the next use, "needs container" on the action bar, `item.armor.equip_generic` /
//! `block.wood.break` sounds, three block particles), survives only on a rigid floor (the A28
//! approximation), gives a comparator signal of the servings left, is not pathfindable, and
//! predicts the use on the client (E-28). A feast block with its own interaction (a `FeastBlock`
//! subclass overriding `useItemOn`) keeps the rest through [`register_custom`], whose use can call
//! [`take_serving`], [`servings`] and [`decide`].

pub use crate::blocks::feast::{FeastSpec, Kind, Outcome, PredictFn, UseFn, decide, take_serving};
use mod_api::{BlockState, Host, Registrar};

/// Attaches FD's feast behaviour to `block` (an id in the caller's namespace). Call once from the
/// add-on's `register` hook; registering the same block again replaces its spec.
pub fn register(registrar: &mut dyn Registrar, block: &'static str, spec: FeastSpec) {
    crate::blocks::feast::register_feast(registrar, block, spec);
}

/// [`register`] with the block's own use in place of `takeServing` (server side) and its client
/// prediction (what the client answers before the server, E-28); survival on a rigid floor, the
/// comparator signal, pathfinding and `useWithoutItem` stay FD's. Added in API 1 (additive).
pub fn register_custom(
    registrar: &mut dyn Registrar,
    block: &'static str,
    spec: FeastSpec,
    on_use: UseFn,
    predict: PredictFn,
) {
    crate::blocks::feast::register_feast_with(registrar, block, spec, Some((on_use, predict)));
}

/// The servings left in `state` (0 when the property is missing).
pub fn servings(host: &dyn Host, state: BlockState, spec: &FeastSpec) -> u32 {
    crate::blocks::feast::servings_in(host.block_state_string(state), spec.servings_property)
}

/// The spec a block was registered with (FD's feasts and add-ons' alike).
pub fn spec_of(block: &str) -> Option<FeastSpec> {
    crate::blocks::feast::registered(block)
}
