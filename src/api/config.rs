//! FD config reads for add-ons (BX-06).
//!
//! The base reads FD's two config files (`config/ported_farmersdelight-{common,client}.json`, FD's
//! key names) and applies live edits; these functions return the current values.

/// A boolean key of FD's common config by its name (`generateFDChestLoot`, `enableVanillaCropCrates`,
/// …); `None` for an unknown or non-boolean key.
pub fn common_bool(name: &str) -> Option<bool> {
    crate::config::COMMON.bool_named(name)
}

/// A boolean key of FD's client config by its name (`enableFoodEffectTooltip`, …).
pub fn client_bool(name: &str) -> Option<bool> {
    crate::config::CLIENT_VALUES.bool_named(name)
}

/// Upstream FD's `Configuration.FARMERS_BUY_FD_CROPS` ("farmers buy FD crops"), which add-ons
/// written for upstream FD (Corn Delight) read before adding farmer trades. FD Refabricated 26.3,
/// the reference of this port, has no such key: its farmer trades are unconditional data, so
/// farmers always buy FD crops and this is always true.
pub fn farmers_buy_fd_crops() -> bool {
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn named_keys() {
        assert_eq!(super::common_bool("generateFDChestLoot"), Some(true));
        assert_eq!(
            super::common_bool("enablePumpkinPieSneakToPlace"),
            Some(false)
        );
        assert_eq!(super::common_bool("richSoilBoostChance"), None);
        assert_eq!(super::common_bool("farmersBuyFDCrops"), None);
        assert!(super::client_bool("enableFoodEffectTooltip").is_some());
        assert!(super::farmers_buy_fd_crops());
    }
}
