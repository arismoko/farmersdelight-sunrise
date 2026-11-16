//! Public API for add-on mods (BX-07). An add-on declares
//! `[dependencies] ported_farmersdelight = "=<exact base version>"` in its `horizon-mod.toml`; a native
//! add-on then calls this module (the launcher adds the base as a Cargo dependency named
//! `ported_farmersdelight`).
//!
//! Stable for one `API_VERSION`: everything under `api`, the add-on tag overlay format
//! (`tools/addon_overlays/`), the item tag `ported_farmersdelight:effect_tooltip` and the base's load
//! conditions. A change that breaks any of them bumps `API_VERSION` and the manifest property
//! `fd_port_api`.

pub mod config;
pub mod feast;
pub mod wild_crop;

/// The add-on API version; equals `[properties] fd_port_api` in `horizon-mod.toml`.
pub const API_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    #[test]
    fn manifest_property_matches_api_version() {
        let manifest = include_str!("../../horizon-mod.toml");
        let line = manifest
            .lines()
            .find(|l| l.starts_with("fd_port_api"))
            .expect("fd_port_api property");
        let value: u32 = line.split('=').nth(1).unwrap().trim().parse().unwrap();
        assert_eq!(value, super::API_VERSION);
    }
}
