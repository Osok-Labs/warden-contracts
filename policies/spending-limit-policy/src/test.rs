#![cfg(test)]

extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext, ContractExecutable, CreateContractHostFnContext},
    symbol_short,
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, IntoVal, String, Vec,
};
use stellar_accounts::smart_account::{ContextRule, ContextRuleType, Signer};

use super::{spending_limit, SpendingLimitAccountParams, SpendingLimitPolicy};

fn create_context_rule(e: &Env) -> ContextRule {
    let mut signers = Vec::new(e);
    signers.push_back(Signer::Delegated(Address::generate(e)));

    ContextRule {
        id: 0,
        context_type: ContextRuleType::CallContract(Address::generate(e)),
        name: String::from_str(e, "spending"),
        signers,
        signer_ids: Vec::new(e),
        policies: Vec::new(e),
        policy_ids: Vec::new(e),
        valid_until: None,
    }
}

fn create_transfer_context(e: &Env, amount: i128) -> Context {
    let from = Address::generate(e);
    let to = Address::generate(e);

    let mut args = Vec::new(e);
    args.push_back(from.into_val(e));
    args.push_back(to.into_val(e));
    args.push_back(amount.into_val(e));

    Context::Contract(ContractContext {
        contract: Address::generate(e),
        fn_name: symbol_short!("transfer"),
        args,
    })
}

fn install_default(e: &Env, context_rule: &ContextRule, smart_account: &Address) {
    let params = SpendingLimitAccountParams {
        spending_limit: 1_000,
        period_ledgers: 100,
    };
    spending_limit::install(e, &params, context_rule, smart_account);
}

#[test]
fn install_success() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);

        let data = SpendingLimitPolicy::get_spending_limit_data(&e, context_rule.id, smart_account);
        assert_eq!(data.spending_limit, 1_000);
        assert_eq!(data.period_ledgers, 100);
        assert_eq!(data.cached_total_spent, 0);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3227)")]
fn install_rejects_non_call_contract_rule() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let mut context_rule = create_context_rule(&e);
    context_rule.context_type = ContextRuleType::Default;

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3222)")]
fn install_rejects_non_positive_limit() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams {
            spending_limit: 0,
            period_ledgers: 100,
        };
        spending_limit::install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3222)")]
fn install_rejects_zero_period() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let params = SpendingLimitAccountParams {
            spending_limit: 1_000,
            period_ledgers: 0,
        };
        spending_limit::install(&e, &params, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3225)")]
fn install_rejects_already_installed() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });
}

#[test]
fn enforce_allows_transfer_within_limit() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, 400);
        spending_limit::enforce(
            &e,
            &context,
            &context_rule.signers,
            &context_rule,
            &smart_account,
        );

        let data = SpendingLimitPolicy::get_spending_limit_data(
            &e,
            context_rule.id,
            smart_account.clone(),
        );
        assert_eq!(data.cached_total_spent, 400);
        assert_eq!(data.spending_history.len(), 1);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3221)")]
fn enforce_rejects_over_limit() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, 1_001);
        spending_limit::enforce(
            &e,
            &context,
            &context_rule.signers,
            &context_rule,
            &smart_account,
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3226)")]
fn enforce_rejects_negative_amount() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, -1);
        spending_limit::enforce(
            &e,
            &context,
            &context_rule.signers,
            &context_rule,
            &smart_account,
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3223)")]
fn enforce_rejects_non_transfer_context() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = Context::CreateContractHostFn(CreateContractHostFnContext {
            salt: BytesN::from_array(&e, &[1u8; 32]),
            executable: ContractExecutable::Wasm(BytesN::from_array(&e, &[1u8; 32])),
        });
        spending_limit::enforce(
            &e,
            &context,
            &context_rule.signers,
            &context_rule,
            &smart_account,
        );
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3223)")]
fn enforce_rejects_empty_authenticated_signers() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, 400);
        spending_limit::enforce(&e, &context, &Vec::new(&e), &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3220)")]
fn enforce_rejects_when_not_installed() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, 400);
        spending_limit::enforce(
            &e,
            &context,
            &context_rule.signers,
            &context_rule,
            &smart_account,
        );
    });
}

#[test]
fn enforce_evicts_entries_outside_rolling_window() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, 900);
        spending_limit::enforce(
            &e,
            &context,
            &context_rule.signers,
            &context_rule,
            &smart_account,
        );
    });

    // period_ledgers is 100 — advance well past the window so the first
    // spend is evicted before the limit check runs.
    e.ledger().with_mut(|l| l.sequence_number += 200);

    e.as_contract(&address, || {
        let context = create_transfer_context(&e, 900);
        spending_limit::enforce(
            &e,
            &context,
            &context_rule.signers,
            &context_rule,
            &smart_account,
        );

        let data = SpendingLimitPolicy::get_spending_limit_data(
            &e,
            context_rule.id,
            smart_account.clone(),
        );
        assert_eq!(data.cached_total_spent, 900);
        assert_eq!(data.spending_history.len(), 1);
    });
}

#[test]
fn set_spending_limit_updates_value() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        spending_limit::set_spending_limit(&e, 5_000, &context_rule, &smart_account);

        let data = SpendingLimitPolicy::get_spending_limit_data(
            &e,
            context_rule.id,
            smart_account.clone(),
        );
        assert_eq!(data.spending_limit, 5_000);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3222)")]
fn set_spending_limit_rejects_non_positive() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        spending_limit::set_spending_limit(&e, 0, &context_rule, &smart_account);
    });
}

#[test]
fn uninstall_success() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        install_default(&e, &context_rule, &smart_account);
    });

    e.as_contract(&address, || {
        spending_limit::uninstall(&e, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3220)")]
fn uninstall_rejects_when_not_installed() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.mock_all_auths();

    e.as_contract(&address, || {
        spending_limit::uninstall(&e, &context_rule, &smart_account);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #3220)")]
fn get_spending_limit_data_rejects_when_not_installed() {
    let e = Env::default();
    let address = e.register(SpendingLimitPolicy, ());
    let smart_account = Address::generate(&e);
    let context_rule = create_context_rule(&e);

    e.as_contract(&address, || {
        SpendingLimitPolicy::get_spending_limit_data(&e, context_rule.id, smart_account);
    });
}
