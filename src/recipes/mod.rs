//! Mod-side recipe types (`farmersdelight:cutting`, `farmersdelight:cooking`), parsed from raw JSON.
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
