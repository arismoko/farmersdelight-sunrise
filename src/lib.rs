//! Farmer's Delight — native Horizon port. `Hooks` only dispatches; logic lives in modules.
#![allow(dead_code)] // skeleton: modules are filled in by later milestones

pub mod api;
mod blocks;
mod client;
mod config;
mod effects;
mod enchant;
mod heat;
mod ids;
mod inventory;
mod items;
mod loot;
mod net;
mod recipes;
mod registry;
mod world;

use mod_api::*;

#[derive(Default)]
struct FarmersDelight {
    recipes: std::sync::Arc<recipes::RecipeTables>,
    removed_skillets: blocks::skillet::Removed,
    pots: blocks::cooking_pot::SharedRef,
    boosts: blocks::rich_soil::Boosts,
    trample: blocks::rich_soil::Trample,
    drops: blocks::wild::Drops,
    baskets: blocks::basket::Signals,
    container_names: blocks::basket::Names,
    sneaking: blocks::safety_net::Sneaking,
    backstabs: enchant::Pending,
}

#[mod_api::horizon_mod]
impl Hooks for FarmersDelight {
    fn register_load_conditions(
        &self,
        _: &RegistrationContext,
        event: &mut RegisterLoadConditionsEvent<'_>,
    ) {
        registry::register_load_conditions(event);
    }

    fn register_configs(&self, _: HostSide<'_>, event: &mut RegisterConfigsEvent<'_>) {
        config::register(ids::MOD_ID, event.configs);
    }

    fn config_loading(&self, host: HostSide<'_>, event: &mut ConfigLoadingEvent<'_>) {
        config::on_file(host, ids::MOD_ID, event.config);
    }

    fn config_reloading(&self, host: HostSide<'_>, event: &mut ConfigReloadingEvent<'_>) {
        config::on_file(host, ids::MOD_ID, event.config);
    }

    fn register_gui_layers(&self, _: &RegistrationContext, event: &mut RegisterGuiLayersEvent<'_>) {
        client::hud::register(event.layers);
    }

    fn client_tick_post(&self, _: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        client::hud::tick();
    }

    fn item_tooltip(&self, host: &dyn ClientHost, event: &mut ItemTooltipEvent<'_>) {
        items::tooltips::vanilla(host, event);
    }

    fn gui_layer_draw(&self, host: &dyn ClientHost, event: &mut GuiLayerDrawEvent<'_>) {
        client::hud::draw(host, event);
    }

    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        registry::register(registrar, &self.recipes, &self.removed_skillets, &self.pots);
        effects::register(registrar);
        recipes::book::register(registrar);
        blocks::rope::register(registrar);
        blocks::tomato::register(registrar);
        blocks::rice::register(registrar);
        blocks::rich_soil::register(registrar, &self.boosts, &self.trample);
        blocks::compost::register(registrar);
        blocks::colony::register(registrar);
        blocks::wild::register(registrar, &self.drops);
        blocks::feast::register(registrar);
        blocks::pie::register(registrar);
        items::knife::register(registrar);
        items::tooltips::register(registrar);
        items::rotten_tomato::register(registrar);
        blocks::storage::register(registrar);
        blocks::basket::register(registrar, &self.baskets, &self.container_names);
        blocks::tatami::register(registrar);
        blocks::safety_net::register(registrar, &self.sneaking);
        blocks::cabinet::register(registrar, &self.baskets, &self.container_names);
    }

    fn block_entity_container(
        &self,
        _: &dyn Host,
        event: &mut BlockEntityContainerEvent<'_>,
    ) -> Option<ModContainer> {
        registry::container(event.block)
    }

    fn recipe_property_sets(
        &self,
        _: &dyn DataLoadContext,
        event: &mut RecipePropertySetsEvent<'_>,
    ) {
        use blocks::cooking_pot::{SERVING_ITEMS, SERVING_SET};
        event.additions.push(RecipePropertyAddition::new(
            SERVING_SET,
            SERVING_ITEMS.iter().map(|s| s.to_string()).collect(),
        ));
    }

    fn recipe_book_categories(
        &self,
        host: HostSide<'_>,
        event: &mut RecipeBookCategoriesEvent<'_>,
    ) {
        let host: &dyn Host = match host {
            HostSide::Server(h) => h,
            HostSide::Client(h) => h,
        };
        recipes::book::categories(host, event.categories);
    }

    fn modify_default_components(
        &self,
        _: &dyn DataLoadContext,
        event: &mut ModifyDefaultComponentsEvent<'_>,
    ) {
        items::foods::modify_default_components(event.item, event.components);
    }

    fn consume_effect(&self, host: &mut dyn ServerHost, event: &mut ConsumeEffectEvent<'_>) {
        effects::consume_effect(host, event);
    }

    fn should_apply_effect_tick_this_tick(
        &self,
        host: &dyn Host,
        event: &mut ShouldApplyEffectTickThisTickEvent,
    ) {
        effects::should_apply(host, event);
    }

    fn apply_effect_tick(&self, host: &dyn Host, event: &mut ApplyEffectTickEvent<'_>) {
        effects::apply_tick(host, event);
    }

    fn entity_interact(
        &self,
        host: &mut dyn ServerHost,
        event: &mut EntityInteractEvent<'_>,
    ) -> Flow {
        if blocks::wild::entity_interact(host, event) == Flow::Cancel {
            return Flow::Cancel;
        }
        items::foods::entity_interact(host, event)
    }

    fn tags_updated(&self, host: HostSide<'_>, event: &mut TagsUpdatedEvent) {
        // The server's data tables were (re)built: rebuild the recipe tables from them.
        if let (HostSide::Server(host), TagsUpdateCause::ServerDataLoad { .. }) =
            (host, event.cause)
        {
            self.recipes.rebuild(&*host);
            loot::refresh_smoking(&*host);
        }
    }

    fn component_types(&self, _: &RegistrationContext, event: &mut ComponentTypesEvent<'_>) {
        event.types.extend(
            items::skillet::COMPONENT_TYPES
                .iter()
                .chain(blocks::cooking_pot::COMPONENT_TYPES.iter())
                .map(|t| t.to_string()),
        );
    }

    fn living_entity_use_item_start(
        &self,
        host: HostSide<'_>,
        event: &mut LivingEntityUseItemStartEvent<'_>,
    ) -> Flow {
        items::skillet::on_use_start(host, event)
    }

    fn living_entity_use_item_finish(
        &self,
        host: HostSide<'_>,
        event: &mut LivingEntityUseItemFinishEvent<'_>,
    ) {
        items::skillet::on_finish(host, event);
    }

    fn register_item_models(
        &self,
        _: &RegistrationContext,
        event: &mut RegisterItemModelsEvent<'_>,
    ) {
        event.models.push(items::skillet::FLIP_MODEL.into());
    }

    /// Draws the cooking skillet's model; FD's flip animation is omitted (delta A12).
    fn item_model_update(&self, _: &dyn ClientHost, event: &mut ItemModelUpdateEvent<'_>) {
        if event.kind == items::skillet::FLIP_MODEL {
            event.models.push("cooking".into());
        }
    }

    fn item_model_property(&self, _: &dyn ClientHost, event: &mut ItemModelPropertyEvent<'_>) {
        if event.property == items::skillet::IS_COOKING {
            event.value = items::skillet::is_cooking(event.stack);
        }
    }

    /// Known kinds keep the rolled stack: `farmersdelight:copy_skillet`'s copy is made in
    /// `loot_drops` (blocks::skillet), where the block entity is still readable.
    fn loot_function(&self, event: &mut LootFunctionEvent<'_>) {
        loot::on_function(event);
    }

    fn loot_condition(&self, event: &mut LootConditionEvent<'_>) {
        loot::on_condition(event);
    }

    fn loot_table_load(&self, _: &dyn DataLoadContext, event: &mut LootTableLoadEvent<'_>) {
        loot::on_table_load(event.id, event.table);
    }

    fn block_drops(&self, level: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) -> Flow {
        blocks::wild::block_drops(level, event);
        blocks::basket::name_drops(&self.container_names, level, event);
        Flow::Continue
    }

    fn loot_drops(&self, host: &mut dyn ServerHost, event: &mut LootDropsEvent<'_>) {
        blocks::skillet::on_loot_drops(&self.removed_skillets, host, event);
        blocks::cooking_pot::on_loot_drops(&self.recipes, &self.pots, host, event);
    }

    /// Client-side predictions for blocks whose use is a server Part (E-28).
    fn use_item_on_block(
        &self,
        mut host: HostSide<'_>,
        event: &mut UseItemOnBlockEvent<'_>,
    ) -> Flow {
        if blocks::feast::predict(host.reborrow(), event) == Flow::Cancel
            || blocks::basket::predict(host.reborrow(), event) == Flow::Cancel
            || blocks::cabinet::predict(host.reborrow(), event) == Flow::Cancel
        {
            return Flow::Cancel;
        }
        if blocks::pie::predict_use(host.reborrow(), event) == Flow::Cancel {
            return Flow::Cancel;
        }
        items::knife::predict_use(host, event)
    }

    fn entity_place(&self, host: &mut dyn ServerHost, event: &mut EntityPlaceEvent) -> Flow {
        blocks::tatami::on_placed(host, event);
        blocks::canvas_sign::on_placed(host, event);
        Flow::Continue
    }

    fn mod_container_viewers(
        &self,
        host: &mut dyn ServerHost,
        event: &mut ModContainerViewersEvent<'_>,
    ) {
        blocks::cabinet::on_viewers(host, event);
    }

    fn right_click_block(
        &self,
        mut host: HostSide<'_>,
        event: &mut RightClickBlockEvent<'_>,
    ) -> Flow {
        if blocks::cutting_board::on_right_click(host.reborrow(), event) == Flow::Cancel {
            return Flow::Cancel;
        }
        items::knife::on_right_click(host, event)
    }

    fn living_incoming_damage(
        &self,
        _: &mut dyn LevelRandomHost,
        event: &mut LivingIncomingDamageEvent<'_>,
    ) -> Flow {
        enchant::on_incoming(&self.backstabs, event);
        Flow::Continue
    }

    fn hears_projectile_impacts(&self, _: &dyn Host, event: &mut HearsProjectileImpactsEvent) {
        event.hears = true;
    }

    fn projectile_impact(
        &self,
        host: &mut dyn ServerHost,
        event: &mut ProjectileImpactEvent<'_>,
    ) -> Flow {
        items::rotten_tomato::on_impact(host, event);
        Flow::Continue
    }

    fn living_knockback(
        &self,
        _: &mut dyn LevelRandomHost,
        event: &mut LivingKnockbackEvent<'_>,
    ) -> Flow {
        items::knife::on_knockback(event);
        Flow::Continue
    }

    fn attack_entity(&self, host: &mut dyn ServerHost, event: &mut AttackEntityEvent<'_>) -> Flow {
        items::skillet::on_attack(host, event);
        Flow::Continue
    }

    /// Rich soil boosts decided in this tick's random ticks land at its end.
    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if matches!(event.phase, ServerTickPhase::ServerTickStart { .. }) {
            self.sneaking.refresh(&*host);
        }
        if matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            blocks::rich_soil::drain(&self.boosts, host);
            blocks::wild::drain(&self.drops, host);
            enchant::drain(&self.backstabs, host);
            blocks::canvas_sign::drain(host);
        }
    }

    fn farmland_trample(&self, _: &dyn LoopContext, event: &mut FarmlandTrampleEvent) -> Flow {
        self.trample.on_trample(event)
    }

    fn server_started(&self, host: &mut dyn ServerHost, _: &mut ServerStartedEvent) {
        self.trample.predict(&*host);
    }

    fn server_about_to_start(&self, _: &mut dyn ServerHost, _: &mut ServerAboutToStartEvent) {
        self.recipes.clear();
    }

    fn server_stopped(&self, _: &mut dyn ServerHost, _: &mut ServerStoppedEvent) {
        self.recipes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn beef_recipe() -> recipes::cutting::CuttingRecipe {
        recipes::cutting::CuttingRecipe {
            id: "farmersdelight:cutting/beef".into(),
            input: Ingredient::of(vec!["minecraft:beef".into()]),
            tools: vec![],
            results: vec![],
            sound: None,
        }
    }

    fn with_mod(recipes: Arc<recipes::RecipeTables>, f: impl FnOnce()) {
        let fd = FarmersDelight {
            recipes,
            ..Default::default()
        };
        testing::with_mods(vec![ModEntry::new(ids::MOD_ID, Box::new(fd))].into(), f);
    }

    #[test]
    fn lifecycle_hooks_clear_published_recipes() {
        let tables = Arc::new(recipes::RecipeTables::default());
        let mut host = testing::FakeHost::new();
        with_mod(tables.clone(), || {
            tables.cutting.publish(vec![beef_recipe()]);
            dispatch_server_stopped(&mut host, false);
            assert!(tables.cutting.snapshot().is_empty());
            tables.cutting.publish(vec![beef_recipe()]);
            dispatch_server_about_to_start(&mut host, true);
            assert!(tables.cutting.snapshot().is_empty());
        });
    }

    #[test]
    fn crate_condition_follows_config_default() {
        let result = registry::VanillaCratesEnabled.test(&serde_json::json!({}));
        assert_eq!(result.ok(), Some(config::vanilla_crop_crates_enabled()));
        assert!(config::vanilla_crop_crates_enabled());
    }
}
