//! # Method Allowlist Policy Module
//!
//! Restricts a context rule's signers to a fixed set of contract function
//! names. This is the building block behind Warden session keys: a
//! `ContextRule` scoped to `CallContract(dapp_contract)` combines an
//! ephemeral signer with this policy so the session key may only invoke
//! the allowlisted functions on that one contract.
//!
//! Session expiry and revocation are intentionally not reimplemented here —
//! the smart account's own `ContextRule` machinery (`valid_until` and
//! `remove_context_rule`) already provides both. This policy only adds the
//! one thing the account model doesn't: narrowing a `CallContract` rule
//! down to specific function names.
//!
//! This policy has no numeric notion of signer-count reachability and
//! unconditionally rejects an empty `authenticated_signers` list — it's
//! exactly the shape of policy `warden-smart-account`'s zero-signer lockout
//! check exists to protect: removing the last signer from a rule this is
//! attached to would leave a rule that can never satisfy `enforce` again.
use soroban_sdk::{
    auth::{Context, ContractContext},
    contracterror, contractevent, contracttype, panic_with_error, Address, Env, Symbol, Vec,
};
use stellar_accounts::smart_account::{ContextRule, ContextRuleType, Signer};

#[contractevent]
#[derive(Clone)]
pub struct SessionEnforced {
    #[topic]
    pub smart_account: Address,
    pub context_rule_id: u32,
    pub fn_name: Symbol,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct SessionInstalled {
    #[topic]
    pub smart_account: Address,
    pub context_rule_id: u32,
    pub allowed_fns: Vec<Symbol>,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct SessionUninstalled {
    #[topic]
    pub smart_account: Address,
    pub context_rule_id: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SessionAccountParams {
    /// The function names on the rule's `CallContract` target that this
    /// session key is permitted to invoke.
    pub allowed_fns: Vec<Symbol>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SessionData {
    pub allowed_fns: Vec<Symbol>,
}

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum SessionError {
    SmartAccountNotInstalled = 1,
    OnlyCallContractAllowed = 2,
    InvalidAllowedFns = 3,
    MethodNotAllowed = 4,
    AlreadyInstalled = 5,
}

#[contracttype]
pub enum SessionStorageKey {
    AccountContext(Address, u32),
}

const DAY_IN_LEDGERS: u32 = 17280;
pub const SESSION_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const SESSION_TTL_THRESHOLD: u32 = SESSION_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Bounds the linear scan performed in `enforce` and the storage size.
pub const MAX_ALLOWED_FNS: u32 = 10;

pub fn get_allowed_fns(e: &Env, context_rule_id: u32, smart_account: &Address) -> Vec<Symbol> {
    let key = SessionStorageKey::AccountContext(smart_account.clone(), context_rule_id);
    e.storage()
        .persistent()
        .get::<_, SessionData>(&key)
        .inspect(|_| {
            e.storage()
                .persistent()
                .extend_ttl(&key, SESSION_TTL_THRESHOLD, SESSION_EXTEND_AMOUNT);
        })
        .map(|data| data.allowed_fns)
        .unwrap_or_else(|| panic_with_error!(e, SessionError::SmartAccountNotInstalled))
}

pub fn enforce(
    e: &Env,
    context: &Context,
    authenticated_signers: &Vec<Signer>,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    smart_account.require_auth();

    if authenticated_signers.is_empty() {
        panic_with_error!(e, SessionError::MethodNotAllowed)
    }

    let allowed_fns = get_allowed_fns(e, context_rule.id, smart_account);

    match context {
        Context::Contract(ContractContext { fn_name, .. }) => {
            if !allowed_fns.contains(fn_name) {
                panic_with_error!(e, SessionError::MethodNotAllowed)
            }

            SessionEnforced {
                smart_account: smart_account.clone(),
                context_rule_id: context_rule.id,
                fn_name: fn_name.clone(),
            }
            .publish(e);
        }
        _ => panic_with_error!(e, SessionError::MethodNotAllowed),
    }
}

pub fn install(
    e: &Env,
    params: &SessionAccountParams,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    smart_account.require_auth();

    if !matches!(context_rule.context_type, ContextRuleType::CallContract(_)) {
        panic_with_error!(e, SessionError::OnlyCallContractAllowed)
    }

    if params.allowed_fns.is_empty() || params.allowed_fns.len() > MAX_ALLOWED_FNS {
        panic_with_error!(e, SessionError::InvalidAllowedFns)
    }

    let key = SessionStorageKey::AccountContext(smart_account.clone(), context_rule.id);

    if e.storage().persistent().has(&key) {
        panic_with_error!(e, SessionError::AlreadyInstalled)
    }

    let data = SessionData {
        allowed_fns: params.allowed_fns.clone(),
    };
    e.storage().persistent().set(&key, &data);

    SessionInstalled {
        smart_account: smart_account.clone(),
        context_rule_id: context_rule.id,
        allowed_fns: params.allowed_fns.clone(),
    }
    .publish(e);
}

pub fn uninstall(e: &Env, context_rule: &ContextRule, smart_account: &Address) {
    smart_account.require_auth();

    let key = SessionStorageKey::AccountContext(smart_account.clone(), context_rule.id);

    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, SessionError::SmartAccountNotInstalled)
    }

    e.storage().persistent().remove(&key);

    SessionUninstalled {
        smart_account: smart_account.clone(),
        context_rule_id: context_rule.id,
    }
    .publish(e);
}
