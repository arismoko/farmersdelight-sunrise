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

/// Block-entity types and their Parts (PORT_PLAN §6.4 step 4).
pub fn register(
    registrar: &mut dyn Registrar,
    recipes: &std::sync::Arc<crate::recipes::RecipeTables>,
) {
    use crate::blocks::cutting_board;
    let board = registrar
        .block_entity_type(cutting_board::ID)
        .blocks(&[cutting_board::ID])
        .build();
    let item_particle = registrar.particle_key("minecraft:item");
    registrar
        .block_entity_part(
            &board,
            cutting_board::CuttingBoard {
                recipes: recipes.clone(),
                item_particle,
            },
        )
        .expect("cutting board block entity type");
}

/// Item storage of the mod's block entities (`ModContainer`).
pub fn container(block: &str) -> Option<ModContainer> {
    match block {
        // One slot, open to automation on every face (Fabric Transfer, all sides).
        crate::blocks::cutting_board::ID => Some(ModContainer {
            slots: 1,
            faces: std::array::from_fn(|_| vec![0]),
            inputs: vec![0],
            outputs: vec![0],
            menu: None,
        }),
        _ => None,
    }
}
