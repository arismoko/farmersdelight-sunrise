//! The cooking pot's recipe book (CookingPotRecipeBookComponent, ModRecipeBookCategories, the
//! RecipeBookType/SearchRecipeBookCategory class tweaks): one book, `farmersdelight:cooking`, its search tab and
//! the three tabs Meals (vegetable noodles), Drinks (apple cider) and Misc (dumplings), filled from the pot's
//! recipes by their `recipe_book_tab`. Placement goes to the six ingredient slots (a 3×2 grid, shapeless).
//!
//! FD's `farmersdelight:cutting` category is registered for the cutting board's recipes but no screen shows it
//! (the board has no menu); the engine takes categories only with a target menu, so it is not declared.
use crate::blocks::cooking_pot::{self, INPUTS, OUTPUT};
use crate::recipes::cooking::{self, CookingRecipe, Tab};
use mod_api::*;
use std::sync::OnceLock;

/// `RecipeBookType.FARMERSDELIGHT_COOKING`.
pub const BOOK: &str = "farmersdelight:cooking";
/// `SearchRecipeBookCategory.FARMERSDELIGHT_COOKING`.
const SEARCH: &str = "farmersdelight:cooking_search";

static KEY: OnceLock<RecipeBookTypeKey> = OnceLock::new();

/// The book's key, once registered.
pub fn key() -> Option<&'static RecipeBookTypeKey> {
    KEY.get()
}

/// `CookingPotScreen#getRecipeBookButtonPosition`: `leftPos + 5`, `height / 2 - 49`, i.e. 34 below the
/// 166-high menu's top edge.
const TOGGLE: [f32; 2] = [5.0, 34.0];

/// Registration: the book type and the pot menu's placement layout and screen.
pub fn register(registrar: &mut dyn Registrar) {
    let Ok(key) = registrar.recipe_book_type(BOOK) else {
        return;
    };
    let _ = KEY.set(key.clone());
    let screen = screen();
    // The ghost result goes to the output slot: the meal display (slot 6) is not a clickable slot here.
    let layout = RecipeBookLayout::Crafting {
        width: 3,
        height: 2,
        slots: (0..INPUTS as u16).collect(),
        result: OUTPUT as u16,
    };
    let _ = registrar.menu_recipe_book(cooking_pot::MENU, &key, cooking::TYPE, layout, screen);
}

/// The pot's screen with its book: the same value for the menu's frozen declaration and the block's
/// `screen` hook, so the shown screen names the book whichever path opened it.
pub fn screen() -> Screen {
    let mut screen = cooking_pot::screen_layout();
    if let Some(key) = key() {
        screen.recipe_book = Some(key.clone());
        screen.recipe_book_toggle = Some(TOGGLE);
    }
    screen
}

fn tab(tab: Tab) -> (&'static str, &'static str) {
    match tab {
        Tab::Meals => (
            "farmersdelight:cooking_meals",
            "farmersdelight:vegetable_noodles",
        ),
        Tab::Drinks => (
            "farmersdelight:cooking_drinks",
            "farmersdelight:apple_cider",
        ),
        Tab::Misc => ("farmersdelight:cooking_misc", "farmersdelight:dumplings"),
    }
}

/// One ingredient slot: its alternatives merged (a `fabric:any` lists several).
fn merged(alternatives: &[Ingredient]) -> Ingredient {
    let mut merged = alternatives[0].clone();
    for other in &alternatives[1..] {
        for item in &other.order {
            if merged.items.insert(item.clone()) {
                merged.order.push(item.clone());
            }
        }
    }
    merged.custom = None;
    merged.children = None;
    merged
}

fn slot(ingredient: &Ingredient) -> RecipeSlotDisplay {
    match ingredient.order.as_slice() {
        [one] => RecipeSlotDisplay::Item(one.clone()),
        many => RecipeSlotDisplay::Composite(
            many.iter().cloned().map(RecipeSlotDisplay::Item).collect(),
        ),
    }
}

fn entry(recipe: &CookingRecipe) -> RecipeBookDisplayEntry {
    let cells: Vec<Ingredient> = recipe.ingredients.iter().map(|a| merged(a)).collect();
    RecipeBookDisplayEntry {
        recipe: recipe.id.clone(),
        displays: vec![RecipeBookDisplay::Shapeless {
            ingredients: cells.iter().map(slot).collect(),
            result: RecipeSlotDisplay::ItemStack(recipe.result.clone()),
            station: RecipeSlotDisplay::Item(cooking_pot::ID.into()),
        }],
        placement: Some(PlacementInfo {
            cells: cells.into_iter().map(Some).collect(),
            shape: None,
        }),
    }
}

/// `recipe_book_categories`: the three tabs from the recipe table this side reads.
pub fn categories(host: &dyn Host, out: &mut Vec<RecipeBookCategory>) {
    let Some(key) = key() else {
        return;
    };
    let (recipes, _) = cooking::build(host);
    for t in [Tab::Meals, Tab::Drinks, Tab::Misc] {
        let (id, icon) = tab(t);
        out.push(RecipeBookCategory {
            id: id.into(),
            recipe_type: cooking::TYPE.into(),
            icon: icon.into(),
            target_menu: cooking_pot::MENU.into(),
            book: key.clone(),
            search_group: SEARCH.into(),
            displays: recipes.iter().filter(|r| r.tab == t).map(entry).collect(),
        });
    }
}

/// `CookingPotScreen#init` with `enableCookingPotRecipeBook` off: an open book is closed when the pot opens.
pub fn close_if_disabled(host: &mut dyn ServerHost, player: PlayerRef) {
    if crate::config::cooking_pot_recipe_book_enabled() {
        return;
    }
    let Some(key) = key() else {
        return;
    };
    if let Some(settings) = player.recipe_book_settings(&*host, key)
        && settings.open
    {
        let _ = player.set_recipe_book_settings(
            host,
            key,
            RecipeBookSettings {
                open: false,
                ..settings
            },
        );
    }
}
