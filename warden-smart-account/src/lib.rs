//! Warden's smart account contract.
//!
//! Same base as Latch (`stellar_accounts::smart_account`), reusing its
//! `add_signer`/`remove_signer`/`batch_add_signer` machinery unchanged where
//! possible. This crate overrides three entry points to close gaps in what
//! OZ's default `SmartAccount` validates:
//!
//! 1. `remove_signer` probes each attached policy for a `would_remain_reachable`
//!    query (by raw function-name symbol, not a Rust trait bound — `Policy` is
//!    an upstream trait we don't own) and reverts if a policy that implements
//!    it reports the removal would make its own requirement unreachable
//!    (e.g. a 2-of-3 threshold with only 1 signer left).
//! 2. `remove_signer` additionally reverts if the removal would leave a
//!    context rule with **zero signers** while any attached policy has no
//!    opinion on reachability at all ("opaque" — the probe in (1) failed or
//!    the function doesn't exist). OZ's own floor only requires "at least
//!    one signer or one policy" on a rule — that passes for 0 signers + 1
//!    policy, but every policy in this repo's own lineup unconditionally
//!    rejects an empty `authenticated_signers` list in `enforce`, so that
//!    combination is a *guaranteed*, not merely possible, permanent lockout.
//!    Reachability-aware policies (threshold-family) are already covered by
//!    (1) and never reach this branch, since a valid threshold is always
//!    `>= 1` and therefore never reports "reachable" at a remaining count of 0.
//! 3. `batch_add_signer` requires an explicit `confirm_threshold_unchanged`
//!    acknowledgment, since silent ratio weakening (3-of-3 becoming 3-of-5)
//!    has no on-chain "correct" answer to enforce automatically.
#![no_std]

use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl,
    crypto::Hash,
    panic_with_error, vec, Address, BytesN, Env, IntoVal, Map, String, Symbol, Val, Vec,
};
use stellar_accounts::smart_account::{
    self as smart_account, AuthPayload, ContextRule, ContextRuleType, ExecutionEntryPoint, Signer,
    SmartAccount, SmartAccountError,
};
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum WardenSmartAccountError {
    /// Removing the signer would make a reachability-aware policy's
    /// requirement unreachable (e.g. threshold above the remaining count).
    SignerRemovedWouldBreakPolicy = 1,
    /// Adding signers would change the policy-to-signer ratio and the
    /// caller did not explicitly acknowledge it.
    SignerAddedWouldWeakenPolicy = 2,
    /// The single-signer `add_signer` entry point is disabled — its fixed
    /// trait signature has no room for `confirm_threshold_unchanged`. Use
    /// `batch_add_signer` instead, even for a single signer.
    SingleSignerAdditionDisabled = 3,
    /// Removing the signer would leave a context rule with zero signers
    /// while a policy with no reachability opinion is still attached to
    /// it — every policy this account supports requires at least one
    /// authenticated signer, so this combination is a guaranteed permanent
    /// lockout, not merely a possible one.
    SignerRemovalWouldZeroOutPolicyGatedRule = 4,
}

#[contract]
pub struct WardenSmartAccount;

#[contractimpl]
impl WardenSmartAccount {
    pub fn __constructor(e: &Env, signers: Vec<Signer>, policies: Map<Address, Val>) {
        smart_account::add_context_rule(
            e,
            &ContextRuleType::Default,
            &String::from_str(e, "default"),
            None,
            &signers,
            &policies,
        );
    }

    /// Adds signers to a context rule. Requires explicit acknowledgment
    /// that the policy-to-signer ratio may weaken; `confirm_threshold_unchanged
    /// = false` always reverts.
    pub fn batch_add_signer(
        e: &Env,
        context_rule_id: u32,
        signers: Vec<Signer>,
        confirm_threshold_unchanged: bool,
    ) {
        e.current_contract_address().require_auth();

        if !confirm_threshold_unchanged {
            panic_with_error!(e, WardenSmartAccountError::SignerAddedWouldWeakenPolicy);
        }

        smart_account::batch_add_signer(e, context_rule_id, &signers);
    }
}

/// Resolves a global `signer_id` to its `Signer` within a context rule by
/// scanning the rule's positionally-aligned `signer_ids`/`signers` arrays.
fn resolve_signer(e: &Env, rule: &ContextRule, signer_id: u32) -> Signer {
    for (i, id) in rule.signer_ids.iter().enumerate() {
        if id == signer_id {
            return rule.signers.get_unchecked(i as u32);
        }
    }
    panic_with_error!(e, SmartAccountError::SignerNotFound);
}

#[contractimpl]
impl CustomAccountInterface for WardenSmartAccount {
    type Error = SmartAccountError;
    type Signature = AuthPayload;

    fn __check_auth(
        e: Env,
        signature_payload: Hash<32>,
        signatures: AuthPayload,
        auth_contexts: Vec<Context>,
    ) -> Result<(), Self::Error> {
        smart_account::do_check_auth(&e, &signature_payload, &signatures, &auth_contexts)
    }
}

#[contractimpl(contracttrait)]
impl SmartAccount for WardenSmartAccount {
    fn add_signer(e: &Env, _context_rule_id: u32, _signer: Signer) -> u32 {
        panic_with_error!(e, WardenSmartAccountError::SingleSignerAdditionDisabled);
    }

    fn remove_signer(e: &Env, context_rule_id: u32, signer_id: u32) {
        e.current_contract_address().require_auth();

        let rule = smart_account::get_context_rule(e, context_rule_id);
        let signer_to_remove = resolve_signer(e, &rule, signer_id);
        let remaining_count = rule.signers.len().saturating_sub(1);

        let mut has_opaque_policy = false;

        for policy in rule.policies.iter() {
            let args: Vec<Val> = vec![
                e,
                context_rule_id.into_val(e),
                e.current_contract_address().into_val(e),
                signer_to_remove.clone().into_val(e),
                remaining_count.into_val(e),
            ];
            match e.try_invoke_contract::<bool, soroban_sdk::Error>(
                &policy,
                &Symbol::new(e, "would_remain_reachable"),
                args,
            ) {
                Ok(Ok(reachable)) => {
                    if !reachable {
                        panic_with_error!(
                            e,
                            WardenSmartAccountError::SignerRemovedWouldBreakPolicy
                        );
                    }
                }
                _ => {
                    // Policy doesn't implement the query — "no opinion" on
                    // numeric reachability. Doesn't clear it to zero out the
                    // rule's signers, though: see (2) in the module doc.
                    has_opaque_policy = true;
                }
            }
        }

        if remaining_count == 0 && has_opaque_policy {
            panic_with_error!(
                e,
                WardenSmartAccountError::SignerRemovalWouldZeroOutPolicyGatedRule
            );
        }

        smart_account::remove_signer(e, context_rule_id, signer_id);
    }
}

#[contractimpl(contracttrait)]
impl ExecutionEntryPoint for WardenSmartAccount {}

#[contractimpl]
impl Upgradeable for WardenSmartAccount {
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, _operator: Address) {
        e.current_contract_address().require_auth();
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}

#[cfg(test)]
mod test;
