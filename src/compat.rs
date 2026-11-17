//! Small adapters from the hook shapes the port was written against (Horizon `afe471cf`) to the
//! Sunrise build pin's, so hook bodies keep their logic (RETARGET_PLAN: same behaviour, new API).
use mod_api::{Flow, InteractionResult};

/// A server-side claim (`Flow::Cancel`, the old `claimed`: the hand swings and the item's own use
/// is skipped) as the result-returning hooks give it: `SUCCESS_SERVER`; `Continue` passes.
pub fn claimed(flow: Flow) -> Option<InteractionResult> {
    (flow == Flow::Cancel).then_some(InteractionResult::SuccessServer)
}
