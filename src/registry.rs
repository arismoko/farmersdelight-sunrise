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
    removed_skillets: &crate::blocks::skillet::Removed,
    pots: &crate::blocks::cooking_pot::SharedRef,
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
    let stove = registrar
        .block_entity_type(crate::blocks::stove::ID)
        .blocks(&[crate::blocks::stove::ID])
        .build();
    let smoke = registrar.particle_key("minecraft:smoke");
    let flame = registrar.particle_key("minecraft:flame");
    registrar
        .block_entity_part(
            &stove,
            crate::blocks::stove::Stove {
                smoke,
                flame,
                client_ticks: Default::default(),
            },
        )
        .expect("stove block entity type");
    let skillet_block = registrar
        .block_entity_type(crate::blocks::skillet::ID)
        .blocks(&[crate::blocks::skillet::ID])
        .build();
    registrar
        .block_entity_part(
            &skillet_block,
            crate::blocks::skillet::Skillet {
                removed: removed_skillets.clone(),
            },
        )
        .expect("skillet block entity type");
    use crate::blocks::cooking_pot;
    let pot = registrar
        .block_entity_type(cooking_pot::ID)
        .blocks(&[cooking_pot::ID])
        .build();
    // FD's three particle types, registered at their first use (M3-09); providers are carried.
    let steam = registrar.particle_key("farmersdelight:steam");
    let _ = crate::items::foods::STAR.set(registrar.particle_key("farmersdelight:star"));
    let _ = registrar.particle_key("farmersdelight:sparkle");
    let bubble_pop = registrar.particle_key("minecraft:bubble_pop");
    registrar
        .block_entity_part(
            &pot,
            cooking_pot::CookingPot {
                recipes: recipes.clone(),
                shared: pots.clone(),
                bubble_pop,
                steam,
                client_ticks: Default::default(),
            },
        )
        .expect("cooking pot block entity type");
    registrar.menu_part(cooking_pot::MENU, cooking_pot::CookingPotMenu);
    let skillet = registrar.item_key(crate::items::skillet::ITEM);
    registrar.item_part(skillet, crate::items::skillet::SkilletItem);
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
        // Six one-item slots; the reference exposes them to automation on every side.
        crate::blocks::stove::ID => Some(ModContainer {
            slots: crate::blocks::stove::SLOTS,
            faces: std::array::from_fn(|_| (0..crate::blocks::stove::SLOTS).collect()),
            inputs: (0..crate::blocks::stove::SLOTS).collect(),
            outputs: (0..crate::blocks::stove::SLOTS).collect(),
            menu: None,
        }),
        crate::blocks::skillet::ID => Some(ModContainer {
            slots: 1,
            faces: std::array::from_fn(|_| vec![0]),
            inputs: vec![0],
            outputs: vec![0],
            menu: None,
        }),
        // CookingPotItemHandler: the top reaches the ingredients (in and out), every other side
        // puts into the container slot and takes from the output.
        crate::blocks::cooking_pot::ID => {
            use crate::blocks::cooking_pot::{CONTAINER, INPUTS, MENU, OUTPUT, SLOTS};
            let sides = vec![CONTAINER, OUTPUT];
            Some(ModContainer {
                slots: SLOTS,
                faces: [
                    sides.clone(),
                    (0..INPUTS).collect(),
                    sides.clone(),
                    sides.clone(),
                    sides.clone(),
                    sides,
                ],
                inputs: (0..INPUTS).chain([CONTAINER]).collect(),
                outputs: (0..INPUTS).chain([OUTPUT]).collect(),
                menu: Some(MENU.into()),
            })
        }
        _ => None,
    }
}
