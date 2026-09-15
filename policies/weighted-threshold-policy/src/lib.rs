//! Weighted M-of-N threshold policy for Warden smart accounts.
//!
//! Thin `#[contract]` wrapper over OZ's `stellar_accounts::policies::weighted_threshold`
//! — all real weighted-threshold logic lives upstream; this crate only supplies
//! the deployable contract shell plus a `would_remain_reachable` query the smart
//! account probes before honoring a signer removal.
//!
//! Unlike `threshold-policy` (every signer counts as 1, so reachability is a
//! bare count comparison), signers here carry individual weights, so
//! reachability after removing a specific signer is: does the combined weight
//! of every *other* currently-configured signer still meet the threshold? Since
//! `enforce` accepts whichever subset of signers actually authenticates, the
//! best case after a removal is every remaining signer authenticating at once —
//! so this is one O(n) sum over the installed `signer_weights` map, not a
//! subset-sum/knapsack search. That resolves the open question the project
//! README carried over from the original design sketch (`min_signers_required`,
//! "smallest signer count whose weight meets threshold"): the mechanism that
//! actually shipped only ever needs to ask "is the ceiling still high enough,"
//! never "what's the cheapest way to reach it," so there's no search space to
//! bound or cap.
#![no_std]

use soroban_sdk::{
    auth::Context, contract, contractimpl, panic_with_error, Address, Env, Map, Vec,
};
use stellar_accounts::{
    policies::{
        weighted_threshold,
        weighted_threshold::{
            WeightedThresholdAccountParams, WeightedThresholdError, WeightedThresholdStorageKey,
        },
        Policy,
    },
    smart_account::{ContextRule, Signer},
};

#[contract]
pub struct WeightedThresholdPolicy;

#[contractimpl]
impl Policy for WeightedThresholdPolicy {
    type AccountParams = WeightedThresholdAccountParams;

    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        weighted_threshold::enforce(
            e,
            &context,
            &authenticated_signers,
            &context_rule,
            &smart_account,
        )
    }

    fn install(
        e: &Env,
        install_params: Self::AccountParams,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        weighted_threshold::install(e, &install_params, &context_rule, &smart_account)
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        weighted_threshold::uninstall(e, &context_rule, &smart_account)
    }
}

#[contractimpl]
impl WeightedThresholdPolicy {
    pub fn get_threshold(e: &Env, context_rule_id: u32, smart_account: Address) -> u32 {
        weighted_threshold::get_threshold(e, context_rule_id, &smart_account)
    }

    pub fn get_signer_weights(
        e: &Env,
        context_rule: ContextRule,
        smart_account: Address,
    ) -> Map<Signer, u32> {
        weighted_threshold::get_signer_weights(e, &context_rule, &smart_account)
    }

    pub fn set_threshold(
        e: Env,
        threshold: u32,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        weighted_threshold::set_threshold(&e, threshold, &context_rule, &smart_account)
    }

    pub fn set_signer_weight(
        e: Env,
        signer: Signer,
        weight: u32,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        weighted_threshold::set_signer_weight(&e, &signer, weight, &context_rule, &smart_account)
    }

    /// Reports whether the threshold would still be satisfiable after a
    /// proposed removal, given the maximum weight every *other* currently
    /// configured signer could contribute. The smart account probes this (by
    /// raw function name, not a Rust trait bound — `Policy` is defined
    /// upstream and we don't own it) before allowing a `remove_signer` call
    /// to go through.
    ///
    /// `remaining_signer_count` is unused here — unlike `threshold-policy`,
    /// a bare count doesn't determine weighted reachability; the removed
    /// signer's own weight does, so this reads the signer-weights map
    /// directly instead.
    pub fn would_remain_reachable(
        e: &Env,
        context_rule_id: u32,
        smart_account: Address,
        signer_to_remove: Signer,
        _remaining_signer_count: u32,
    ) -> bool {
        let key = WeightedThresholdStorageKey::AccountContext(smart_account, context_rule_id);
        let params: WeightedThresholdAccountParams =
            e.storage().persistent().get(&key).unwrap_or_else(|| {
                panic_with_error!(e, WeightedThresholdError::SmartAccountNotInstalled)
            });

        let mut remaining_weight: u32 = 0;
        for (signer, weight) in params.signer_weights.iter() {
            if signer != signer_to_remove {
                remaining_weight = remaining_weight.saturating_add(weight);
            }
        }

        remaining_weight >= params.threshold
    }
}

#[cfg(test)]
mod test;
