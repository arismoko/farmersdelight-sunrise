//! Every identifier string used by the port. Always fully qualified.

/// The manifest's package id. Load conditions must live in this namespace (Horizon refuses a
/// condition whose namespace differs from the registering mod's id).
pub const MOD_ID: &str = "ported_farmersdelight";
/// The content namespace the package owns (`namespaces` in horizon-mod.toml).
pub const NAMESPACE: &str = "farmersdelight";

/// Load condition gating the vanilla crop crate recipes (`enableVanillaCropCrates`).
pub const VANILLA_CRATES_ENABLED: &str = "ported_farmersdelight:vanilla_crates_enabled";
