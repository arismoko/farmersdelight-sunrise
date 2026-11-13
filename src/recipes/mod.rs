//! Mod-side recipe types (`farmersdelight:cutting`, `farmersdelight:cooking`), parsed from raw JSON.
pub mod cache;

/// Every table rebuilt per data load. Cleared when a server stops or is about to start so a
/// new world never sees the previous world's definitions.
#[derive(Default)]
pub struct RecipeTables {
    /// Filled by M1-04 (cutting) and M3-02 (cooking); ids of the published recipes for now.
    pub cutting: cache::Published<String>,
}

impl RecipeTables {
    pub fn clear(&self) {
        self.cutting.clear();
    }
}
