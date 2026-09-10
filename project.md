# warden-contracts

Soroban smart-account contracts. Same stack as [Latch](https://github.com/3K1-Labs/latch-contracts)
(Rust, Soroban, OZ `stellar-accounts` as the base layer), additive rather than a rewrite: the
existing factory / account / verifier architecture is reused unchanged.

> **Status: design phase, no code yet.** See "Suggested next step" below for what to build first.

## Why this repo, in one paragraph

A `ContextRule` is satisfied when enough attached signers authenticate *and* every attached
policy's `enforce` succeeds. Nothing today checks that the specific combination of signers and
policies on a rule stays jointly satisfiable after a mutation — only that the rule has "at least
one signer or one policy," a much weaker check. A 2-of-3 threshold policy can end up on a rule
with two signers; a session policy can end up on a rule with zero signers. Both are permanently
stuck, and both are legal under today's check. This repo's core feature closes that gap
generically, at the account level.

## Scope

**In scope for v1:**
- A `min_signers_required`-style structural query, added as a parallel convention (not a
  required method on OZ's `Policy` trait, since we don't own that trait) so policies that gate on
  signer count can answer "how many signers do I need at minimum," and the account can check it.
- Account-level invariant enforcement on `add_signer` / `remove_signer` / `batch_add_signer` /
  `batch_remove_signer` / `add_policy` / `remove_policy`: reject a mutation that would leave a
  context rule structurally unsatisfiable.
- Three new policy crates: `time-window-policy`, `call-count-limit-policy`,
  `approval-delay-policy`.

**Explicitly out of scope for v1** (named, not silently skipped):
- Full symbolic/semantic satisfiability checking for arbitrary custom policies — only count-based
  policies can answer `min_signers_required` meaningfully. A policy that can't answer returns
  "unknown," and a rule with an "unknown" policy is exempt from the check for that policy, not
  treated as safe by default.
- Guardian/social recovery.
- New verifier types (BLS/RSA/ZK/email).

## Workspace layout (target shape)

```
warden-contracts/
├── account-factory/
│   └── contracts/factory-contract/
├── warden-smart-account/
├── warden-verifiers/
│   ├── ed25519-verifier/
│   ├── p256-verifier/
│   ├── secp256k1-verifier/
│   └── webauthn-verifier/
├── policies/
│   ├── threshold-policy/
│   ├── weighted-threshold-policy/
│   ├── session-policy/
│   ├── spending-limit-policy/
│   ├── time-window-policy/          # new
│   ├── call-count-limit-policy/     # new
│   └── approval-delay-policy/       # new
├── fee-forwarder/
└── docs/
    ├── smart-account-authz-spec.md  # full design: invariant + 3 policies
    └── repo-architecture-spec.md    # three-repo split, interfaces
```

## `min_signers_required` — the core mechanism

```rust
pub fn min_signers_required(e: &Env, context_rule_id: u32, smart_account: Address) -> Option<u32>;
```

| Policy | `min_signers_required` |
|---|---|
| `threshold-policy` | `Some(threshold)` |
| `weighted-threshold-policy` | `Some(smallest signer count whose weight meets threshold)` |
| `session-policy` | `Some(1)` |
| `approval-delay-policy` | `Some(2)` |
| `spending-limit-policy`, `time-window-policy`, `call-count-limit-policy` | `None` — constrain *what*/*when*, not *who* |

`required_signers(rule) = max(1 if zero policies else 0, max over Some(_)-returning policies)`.
Each overridden account entry point calls through to the OZ default first, then reverts the whole
call with `InvariantWouldBreak` if `required_signers(rule) > actual_signer_count` post-mutation.

## Two open questions to resolve before writing code

1. **Where threshold changes actually live.** `set_threshold` / `set_signer_weight` are exposed
   by the *policy* contracts today, not the account. The account-side overrides above can't catch
   a threshold shrink made directly through the policy contract. Needs a decision: policy calls
   back into the account to re-validate, or the account gets an event-triggered post-mutation
   sweep. Until this is resolved, the invariant check is incomplete, not just imperfect.
2. **Cost of `weighted-threshold-policy`'s `min_signers_required`.** It's a knapsack-shaped
   computation, run on every mutation, inside a contract with real per-invocation resource limits.
   Needs benchmarking before assuming it's cheap enough to ship as designed — may need a signer-
   count cap.

Full detail, including the `approval-delay-policy` propose/approve/execute flow and its client-
side UX cost, lives in `docs/smart-account-authz-spec.md` (currently `idea2.md` at the project
root, pending move).

## Suggested next step

Prototype the invariant check alone — using only `threshold-policy` and `session-policy` as the
two `min_signers_required` implementers — against a forked copy of the smart-account crate.
Acceptance bar: two regression tests replaying Latch's own issue #38/#77 scenarios, asserting the
mutation now reverts instead of succeeding. This validates the core mechanism before investing in
the three new policy crates.

## Development

Prerequisites: Rust (`rustup`), Stellar CLI v25.2.0+ (`cargo install --locked stellar-cli`).

```
cargo +nightly fmt --all -- --check     # whole workspace
cd <crate>
cargo clippy --all-targets --all-features -- -D warnings
cargo test
stellar contract build
```
