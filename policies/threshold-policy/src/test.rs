#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use stellar_accounts::smart_account::{ContextRule, ContextRuleType, Signer};

use super::{simple_threshold, SimpleThresholdAccountParams, ThresholdPolicy};

fn create_context_rule(e: &Env, signer_count: u32) -> ContextRule {
    let mut signers = Vec::new(e);
    for _ in 0..signer_count {
        signers.push_back(Signer::Delegated(Address::generate(e)));
    }
    ContextRule {
        id: 0,
        context_type: ContextRuleType::Default,
        name: String::from_str(e, "test"),
        signers,
        signer_ids: Vec::new(e),
        policies: Vec::new(e),
        policy_ids: Vec::new(e),
        valid_until: None,
    }
}

#[test]
fn would_remain_reachable_true_when_remaining_meets_threshold() {
    let e = Env::default();
    let address = e.register(ThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e, 3);
    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SimpleThresholdAccountParams { threshold: 2 };
        simple_threshold::install(&e, &params, &context_rule, &smart_account);
    });

    let signer_to_remove = context_rule.signers.get_unchecked(0);
    e.as_contract(&address, || {
        assert!(ThresholdPolicy::would_remain_reachable(
            &e,
            0,
            smart_account.clone(),
            signer_to_remove.clone(),
            2,
        ));
    });
}

#[test]
fn would_remain_reachable_false_when_remaining_below_threshold() {
    let e = Env::default();
    let address = e.register(ThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e, 3);
    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SimpleThresholdAccountParams { threshold: 3 };
        simple_threshold::install(&e, &params, &context_rule, &smart_account);
    });

    let signer_to_remove = context_rule.signers.get_unchecked(0);
    e.as_contract(&address, || {
        assert!(!ThresholdPolicy::would_remain_reachable(
            &e,
            0,
            smart_account.clone(),
            signer_to_remove.clone(),
            2,
        ));
    });
}
