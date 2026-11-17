//! Mod-side recipe types (`farmersdelight:cutting`, `farmersdelight:cooking`), parsed from raw JSON.
pub mod book;
pub mod cache;
pub mod cooking;
pub mod cutting;
pub mod ingredient;

use mod_api::Host;

/// Every table rebuilt per data load. Cleared when a server stops or is about to start so a
/// new world never sees the previous world's definitions.
#[derive(Default)]
pub struct RecipeTables {
    pub cutting: cache::Published<cutting::CuttingRecipe>,
    pub cooking: cache::Published<cooking::CookingRecipe>,
}

impl RecipeTables {
    pub fn clear(&self) {
        self.cutting.clear();
        self.cooking.clear();
    }

    /// Rebuild every table from the server's data (first open and each `/reload`).
    pub fn rebuild(&self, host: &dyn Host) {
        let (cutting, mut skipped) = cutting::build(host);
        let (cooking, skipped_cooking) = cooking::build(host);
        skipped.extend(skipped_cooking);
        for (id, why) in &skipped {
            eprintln!("[ported_farmersdelight] skipping recipe {id}: {why}");
        }
        self.cutting.publish(cutting);
        self.cooking.publish(cooking);
    }
}

/// FD's two special crafting serializers (`DoughRecipe`, `FoodServingRecipe`). Not yet ported as crafting kinds:
/// E-12 at Horizon; Sunrise's `register_crafting_recipe_serializer` could carry them (follow-up).
const SPECIAL_CRAFTING: [&str; 2] = ["farmersdelight:dough", "farmersdelight:food_serving"];

/// Sunrise refuses a recipe whose serializer no mod registered (`crafting/mod_kinds.rs`). The pot's and the
/// board's recipes are read by this mod through `recipes_of_type`, so each is a recipe type with a raw
/// (source-preserving) serializer of the same id, as FD registers them. The special crafting files load as
/// inert raw recipes of their own type, which keeps the Horizon behaviour (they never enter the grid).
pub fn register_serializers(registrar: &mut dyn mod_api::Registrar) {
    for id in [cooking::TYPE, cutting::TYPE]
        .into_iter()
        .chain(SPECIAL_CRAFTING)
    {
        registrar.register_recipe_type(id);
        registrar.register_raw_recipe_serializer(id, id);
    }
}
