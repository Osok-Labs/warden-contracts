//! Simple (unweighted) M-of-N threshold policy for Warden smart accounts.
//!
//! Thin `#[contract]` wrapper over OZ's `stellar_accounts::policies::simple_threshold` —
//! all real threshold logic lives upstream; this crate only supplies the deployable
//! contract shell plus a `would_remain_reachable` query the smart account probes
//! before honoring a signer removal.
#![no_std]

use soroban_sdk::{auth::Context, contract, contractimpl, Address, Env, Vec};
use stellar_accounts::{
    policies::{simple_threshold, simple_threshold::SimpleThresholdAccountParams, Policy},
    smart_account::{ContextRule, Signer},
};

#[contract]
pub struct ThresholdPolicy;

#[contractimpl]
impl Policy for ThresholdPolicy {
    type AccountParams = SimpleThresholdAccountParams;

    fn enforce(
        e: &Env,
        context: Context,
        authenticated_signers: Vec<Signer>,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        simple_threshold::enforce(
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
        simple_threshold::install(e, &install_params, &context_rule, &smart_account)
    }

    fn uninstall(e: &Env, context_rule: ContextRule, smart_account: Address) {
        simple_threshold::uninstall(e, &context_rule, &smart_account)
    }
}

#[contractimpl]
impl ThresholdPolicy {
    pub fn get_threshold(e: &Env, context_rule_id: u32, smart_account: Address) -> u32 {
        simple_threshold::get_threshold(e, context_rule_id, &smart_account)
    }

    pub fn set_threshold(
        e: Env,
        threshold: u32,
        context_rule: ContextRule,
        smart_account: Address,
    ) {
        simple_threshold::set_threshold(&e, threshold, &context_rule, &smart_account)
    }

    /// Reports whether the threshold would still be satisfiable after a
    /// proposed removal, given how many signers would remain. The smart
    /// account probes this (by raw function name, not a Rust trait bound —
    /// `Policy` is defined upstream and we don't own it) before allowing a
    /// `remove_signer` call to go through.
    pub fn would_remain_reachable(
        e: &Env,
        context_rule_id: u32,
        smart_account: Address,
        _signer_to_remove: Signer,
        remaining_signer_count: u32,
    ) -> bool {
        let threshold = simple_threshold::get_threshold(e, context_rule_id, &smart_account);
        remaining_signer_count >= threshold
    }
}

#[cfg(test)]
mod test;
