//! Mod-side recipe types (`farmersdelight:cutting`, `farmersdelight:cooking`), parsed from raw JSON.
pub mod cache;
pub mod cutting;
pub mod ingredient;

use mod_api::Host;

/// Every table rebuilt per data load. Cleared when a server stops or is about to start so a
/// new world never sees the previous world's definitions.
#[derive(Default)]
pub struct RecipeTables {
    pub cutting: cache::Published<cutting::CuttingRecipe>,
}

impl RecipeTables {
    pub fn clear(&self) {
        self.cutting.clear();
    }

    /// Rebuild every table from the server's data (first open and each `/reload`).
    pub fn rebuild(&self, host: &dyn Host) {
        let (table, skipped) = cutting::build(host);
        for (id, why) in &skipped {
            eprintln!("[ported_farmersdelight] skipping recipe {id}: {why}");
        }
        self.cutting.publish(table);
    }
}
