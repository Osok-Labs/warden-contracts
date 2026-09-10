#![cfg(test)]

extern crate std;

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, testutils::Address as _, vec, Address,
    Env, IntoVal, Map, String, Val, Vec,
};
use stellar_accounts::{
    policies::{simple_threshold::SimpleThresholdAccountParams, Policy},
    smart_account::{ContextRuleType, Signer},
};

use super::{WardenSmartAccount, WardenSmartAccountClient};

/// A policy that implements `enforce`/`install`/`uninstall` (so it's a
/// legal Latch/Warden policy — it unconditionally rejects zero
/// authenticated signers, matching every real policy in this ecosystem's
/// lineup) but deliberately does **not** implement `would_remain_reachable`
/// — standing in for `session-policy`/`spending-limit-policy`, which have
/// no numeric reachability concept to report.
#[contract]
struct OpaquePolicyContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
enum OpaquePolicyError {
    NoAuthenticatedSigners = 1,
}

#[contractimpl]
impl Policy for OpaquePolicyContract {
    type AccountParams = Val;

    fn enforce(
        e: &Env,
        _context: soroban_sdk::auth::Context,
        authenticated_signers: Vec<Signer>,
        _context_rule: stellar_accounts::smart_account::ContextRule,
        _smart_account: Address,
    ) {
        if authenticated_signers.is_empty() {
            panic_with_error!(e, OpaquePolicyError::NoAuthenticatedSigners);
        }
    }

    fn install(
        _e: &Env,
        _install_params: Val,
        _context_rule: stellar_accounts::smart_account::ContextRule,
        _smart_account: Address,
    ) {
    }

    fn uninstall(
        _e: &Env,
        _context_rule: stellar_accounts::smart_account::ContextRule,
        _smart_account: Address,
    ) {
    }
}

fn default_signers(env: &Env) -> Vec<Signer> {
    vec![env, Signer::Delegated(Address::generate(env))]
}

fn register_account<'a>(
    env: &'a Env,
    signers: &Vec<Signer>,
    policies: &Map<Address, Val>,
) -> (Address, WardenSmartAccountClient<'a>) {
    let account_id = env.register(WardenSmartAccount, (signers.clone(), policies.clone()));
    let client = WardenSmartAccountClient::new(env, &account_id);
    (account_id, client)
}

#[test]
fn constructor_creates_one_default_rule_named_default() {
    let env = Env::default();
    let signers = default_signers(&env);
    let policies = Map::new(&env);
    let (_account_id, client) = register_account(&env, &signers, &policies);

    assert_eq!(client.get_context_rules_count(), 1);
    let rule = client.get_context_rule(&0);
    assert_eq!(rule.name, String::from_str(&env, "default"));
    assert_eq!(rule.context_type, ContextRuleType::Default);
    assert_eq!(rule.signers, signers);
}

// ################## THRESHOLD REACHABILITY (baseline, matches upstream) ##################

fn setup_with_threshold(
    env: &Env,
    signer_count: u32,
    threshold: u32,
) -> (Address, WardenSmartAccountClient<'_>) {
    env.mock_all_auths();

    let mut signers = Vec::new(env);
    for _ in 0..signer_count {
        signers.push_back(Signer::Delegated(Address::generate(env)));
    }
    let policies = Map::new(env);
    let (account_id, client) = register_account(env, &signers, &policies);

    let threshold_policy_id = env.register(threshold_policy::ThresholdPolicy, ());
    let install_param: Val = SimpleThresholdAccountParams { threshold }.into_val(env);
    client.add_policy(&0, &threshold_policy_id, &install_param);

    (account_id, client)
}

#[test]
fn remove_signer_succeeds_when_threshold_still_reachable() {
    let env = Env::default();
    // 3 signers, threshold=2. Remove one -> 2 remaining >= 2 -> ok.
    let (_account_id, client) = setup_with_threshold(&env, 3, 2);

    env.mock_all_auths();
    client.remove_signer(&0, &0);

    let rule = client.get_context_rule(&0);
    assert_eq!(rule.signers.len(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn remove_signer_blocked_when_threshold_would_become_unreachable() {
    let env = Env::default();
    // 3 signers, threshold=3. Remove one -> 2 remaining < 3 -> blocked.
    let (_account_id, client) = setup_with_threshold(&env, 3, 3);

    env.mock_all_auths();
    client.remove_signer(&0, &0);
}

// ################## ZERO-SIGNER LOCKOUT (the fix this repo adds) ##################
//
// Regression coverage for the still-open upstream gap this account closes:
// removing the last signer from a rule whose only attached policy has no
// `would_remain_reachable` opinion succeeds under OZ's core check (which
// only requires "at least one signer or one policy") and under the
// threshold-reachability fix alone (which only probes policies that *do*
// answer the query) -- and then permanently locks the rule, since every
// real policy's `enforce` unconditionally rejects zero authenticated
// signers. `WardenSmartAccount::remove_signer` closes this by blocking the
// removal outright instead of letting it through.

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn remove_signer_blocked_when_it_would_zero_out_an_opaque_policy_gated_rule() {
    let env = Env::default();
    env.mock_all_auths();

    let signers = default_signers(&env);
    let policies = Map::new(&env);
    let (_account_id, client) = register_account(&env, &signers, &policies);

    // Attach a policy with no would_remain_reachable opinion, matching
    // session-policy/spending-limit-policy's real shape.
    let opaque_policy_id = env.register(OpaquePolicyContract, ());
    let install_param: Val = Val::from_void().into();
    client.add_policy(&0, &opaque_policy_id, &install_param);

    // Only 1 signer on this rule. Removing it would zero out the rule while
    // the opaque policy stays attached -- must be blocked.
    client.remove_signer(&0, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3004)")]
fn remove_signer_with_no_policies_still_blocked_by_oz_own_floor() {
    let env = Env::default();
    env.mock_all_auths();

    let signers = default_signers(&env);
    let policies = Map::new(&env);
    let (_account_id, client) = register_account(&env, &signers, &policies);

    // No policies attached at all -- our new check (#4) never fires here
    // since `has_opaque_policy` stays false, but OZ's own pre-existing
    // floor ("at least one signer or one policy") already rejects a rule
    // left with neither, independent of anything this crate adds.
    client.remove_signer(&0, &0);
}

#[test]
fn remove_signer_succeeds_zeroing_out_when_other_signers_remain_on_rule() {
    let env = Env::default();
    env.mock_all_auths();

    // 2 signers + an opaque policy. Removing one leaves 1 remaining, not 0
    // -- the new check only fires when the removal would reach exactly
    // zero, so this must still succeed.
    let signer_a = Signer::Delegated(Address::generate(&env));
    let signer_b = Signer::Delegated(Address::generate(&env));
    let signers = vec![&env, signer_a, signer_b];
    let policies = Map::new(&env);
    let (_account_id, client) = register_account(&env, &signers, &policies);

    let opaque_policy_id = env.register(OpaquePolicyContract, ());
    let install_param: Val = Val::from_void().into();
    client.add_policy(&0, &opaque_policy_id, &install_param);

    client.remove_signer(&0, &0);

    let rule = client.get_context_rule(&0);
    assert_eq!(rule.signers.len(), 1);
}
