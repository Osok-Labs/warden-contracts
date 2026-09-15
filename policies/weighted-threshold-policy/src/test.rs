#![cfg(test)]

extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext},
    symbol_short,
    testutils::Address as _,
    Address, Env, Map, String, Vec,
};
use stellar_accounts::{
    policies::weighted_threshold,
    smart_account::{ContextRule, ContextRuleType, Signer},
};

use super::{WeightedThresholdAccountParams, WeightedThresholdPolicy};

fn create_context_rule(e: &Env, signers: &[Signer]) -> ContextRule {
    let mut signer_vec = Vec::new(e);
    for signer in signers {
        signer_vec.push_back(signer.clone());
    }

    ContextRule {
        id: 0,
        context_type: ContextRuleType::CallContract(Address::generate(e)),
        name: String::from_str(e, "weighted"),
        signers: signer_vec,
        signer_ids: Vec::new(e),
        policies: Vec::new(e),
        policy_ids: Vec::new(e),
        valid_until: None,
    }
}

fn signer_weights(e: &Env, pairs: &[(Signer, u32)]) -> Map<Signer, u32> {
    let mut map = Map::new(e);
    for (signer, weight) in pairs {
        map.set(signer.clone(), *weight);
    }
    map
}

fn any_context(e: &Env) -> Context {
    Context::Contract(ContractContext {
        contract: Address::generate(e),
        fn_name: symbol_short!("go"),
        args: Vec::new(e),
    })
}

/// CEO(100), CTO(75), CFO(75), threshold=150 — matches the module doc's own
/// example.
fn install_default(
    e: &Env,
    context_rule: &ContextRule,
    smart_account: &Address,
    ceo: &Signer,
    cto: &Signer,
    cfo: &Signer,
) {
    let params = WeightedThresholdAccountParams {
        signer_weights: signer_weights(
            e,
            &[(ceo.clone(), 100), (cto.clone(), 75), (cfo.clone(), 75)],
        ),
        threshold: 150,
    };
    weighted_threshold::install(e, &params, context_rule, smart_account);
}

#[test]
fn install_success() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);

        assert_eq!(
            WeightedThresholdPolicy::get_threshold(&e, context_rule.id, smart_account.clone()),
            150
        );
        let weights =
            WeightedThresholdPolicy::get_signer_weights(&e, context_rule.clone(), smart_account);
        assert_eq!(weights.get(ceo).unwrap(), 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn install_rejects_zero_threshold() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, core::slice::from_ref(&ceo));

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = WeightedThresholdAccountParams {
            signer_weights: signer_weights(&e, &[(ceo.clone(), 100)]),
            threshold: 0,
        };
        weighted_threshold::install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn install_rejects_threshold_exceeding_total_weight() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, core::slice::from_ref(&ceo));

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = WeightedThresholdAccountParams {
            signer_weights: signer_weights(&e, &[(ceo.clone(), 100)]),
            threshold: 101,
        };
        weighted_threshold::install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3214)")]
fn install_rejects_already_installed() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });
}

#[test]
fn enforce_allows_when_weight_meets_threshold() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    e.as_contract(&address, || {
        // CTO + CFO = 150, meets the threshold without the CEO.
        let authenticated = Vec::from_array(&e, [cto.clone(), cfo.clone()]);
        let context = any_context(&e);
        weighted_threshold::enforce(&e, &context, &authenticated, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3213)")]
fn enforce_rejects_when_weight_below_threshold() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    e.as_contract(&address, || {
        // CTO alone = 75, below the 150 threshold.
        let authenticated = Vec::from_array(&e, [cto.clone()]);
        let context = any_context(&e);
        weighted_threshold::enforce(&e, &context, &authenticated, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3210)")]
fn enforce_rejects_when_not_installed() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, core::slice::from_ref(&ceo));

    e.mock_all_auths();

    e.as_contract(&address, || {
        let authenticated = Vec::from_array(&e, [ceo.clone()]);
        let context = any_context(&e);
        weighted_threshold::enforce(&e, &context, &authenticated, &context_rule, &smart_account);
    });
}

#[test]
fn set_threshold_updates_value() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    e.as_contract(&address, || {
        weighted_threshold::set_threshold(&e, 175, &context_rule, &smart_account);
        assert_eq!(
            WeightedThresholdPolicy::get_threshold(&e, context_rule.id, smart_account.clone()),
            175
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn set_threshold_rejects_exceeding_total_weight() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    e.as_contract(&address, || {
        // Total configured weight is 250; 251 is unreachable.
        weighted_threshold::set_threshold(&e, 251, &context_rule, &smart_account);
    });
}

#[test]
fn set_signer_weight_updates_value() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    e.as_contract(&address, || {
        weighted_threshold::set_signer_weight(&e, &cfo, 200, &context_rule, &smart_account);
        let weights = WeightedThresholdPolicy::get_signer_weights(
            &e,
            context_rule.clone(),
            smart_account.clone(),
        );
        assert_eq!(weights.get(cfo).unwrap(), 200);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3211)")]
fn set_signer_weight_rejects_when_new_total_below_threshold() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        // Threshold set equal to total weight (175) -- tight, so reducing
        // any signer's weight at all makes it unreachable.
        let params = WeightedThresholdAccountParams {
            signer_weights: signer_weights(&e, &[(ceo.clone(), 100), (cto.clone(), 75)]),
            threshold: 175,
        };
        weighted_threshold::install(&e, &params, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        weighted_threshold::set_signer_weight(&e, &cto, 50, &context_rule, &smart_account);
    });
}

#[test]
fn uninstall_success() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    e.as_contract(&address, || {
        weighted_threshold::uninstall(&e, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3210)")]
fn uninstall_rejects_when_not_installed() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, core::slice::from_ref(&ceo));

    e.mock_all_auths();

    e.as_contract(&address, || {
        weighted_threshold::uninstall(&e, &context_rule, &smart_account);
    });
}

#[test]
fn would_remain_reachable_true_when_remaining_weight_meets_threshold() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    // Removing the CFO (weight 75) leaves CEO(100) + CTO(75) = 175, still
    // >= the 150 threshold.
    e.as_contract(&address, || {
        assert!(WeightedThresholdPolicy::would_remain_reachable(
            &e,
            context_rule.id,
            smart_account.clone(),
            cfo.clone(),
            2,
        ));
    });
}

#[test]
fn would_remain_reachable_false_when_remaining_weight_below_threshold() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    // At the default 150-of-250 setup, removing any single signer still
    // leaves >= 150 (worst case 250-100=150). Tighten the threshold first
    // so a removal actually crosses it.
    e.as_contract(&address, || {
        weighted_threshold::set_threshold(&e, 176, &context_rule, &smart_account);
    });

    // Now removing the CFO (weight 75) leaves 175, below the 176 threshold.
    e.as_contract(&address, || {
        assert!(!WeightedThresholdPolicy::would_remain_reachable(
            &e,
            context_rule.id,
            smart_account.clone(),
            cfo.clone(),
            2,
        ));
    });
}

#[test]
fn would_remain_reachable_ignores_signer_not_in_weights_map() {
    let e = Env::default();
    let address = e.register(WeightedThresholdPolicy, ());
    let smart_account = Address::generate(&e);
    let ceo = Signer::Delegated(Address::generate(&e));
    let cto = Signer::Delegated(Address::generate(&e));
    let cfo = Signer::Delegated(Address::generate(&e));
    let stranger = Signer::Delegated(Address::generate(&e));
    let context_rule = create_context_rule(&e, &[ceo.clone(), cto.clone(), cfo.clone()]);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account, &ceo, &cto, &cfo);
    });

    // A signer with no configured weight contributes 0 either way --
    // removing it doesn't change the remaining weight (250), still >= 150.
    e.as_contract(&address, || {
        assert!(WeightedThresholdPolicy::would_remain_reachable(
            &e,
            context_rule.id,
            smart_account.clone(),
            stranger,
            3,
        ));
    });
}
