//! Fresh slot/stack helpers (no LGPL reference code was read or used).
use mod_api::*;

/// Insert `stack` into the player's inventory inside `tx`; whatever does not fit is dropped at
/// the player as `Player.drop` does, so items are never deleted.
pub fn give_or_drop(
    host: &mut dyn ServerHost,
    tx: &Transaction,
    player: PlayerRef,
    stack: ItemStack,
) {
    let resource = ItemResource::of(&stack);
    let total = stack.count();
    let inserted = player
        .entity()
        .capability(host, &ENTITY_ITEMS, ())
        .map_or(0, |inv| inv.insert_any(host, tx, &resource, total));
    if inserted < total
        && let Some(rest) = stack.shrink(inserted)
    {
        let _ = player.drop_stack(host, rest, false);
    }
}
