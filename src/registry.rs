//! Registration: load conditions now; block-entity types, parts and payloads in later tasks.
use crate::{config, ids};
use mod_api::*;

/// `ported_farmersdelight:vanilla_crates_enabled`: true while the crop crates are enabled.
/// Takes no arguments; the reference's Fabric condition is argument-free too.
pub struct VanillaCratesEnabled;

impl LoadCondition for VanillaCratesEnabled {
    fn test(&self, _args: &serde_json::Value) -> Result<bool, ConditionError> {
        Ok(config::vanilla_crop_crates_enabled())
    }
}

pub fn register_load_conditions(event: &mut RegisterLoadConditionsEvent<'_>) {
    // A registration error refuses admission even if ignored; nothing to recover here.
    let _ = event.conditions.register(
        Identifier::parse(ids::VANILLA_CRATES_ENABLED).expect("valid id"),
        VanillaCratesEnabled,
    );
}
