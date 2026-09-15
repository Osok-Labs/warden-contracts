# warden-contracts

Soroban smart-account contracts, built on [OpenZeppelin's `stellar-accounts`](https://github.com/OpenZeppelin/stellar-contracts)
— the same base layer as [Latch](https://github.com/3K1-Labs/latch-contracts), reused unchanged
where possible. This repo's job is closing one gap Latch's own `AUDIT_SCOPE.md` documents as an
accepted, unfixed v1 risk, and adding three new authorization policies on top of the same
framework.

## The problem, in one paragraph

A Soroban smart account's `ContextRule` is satisfied when enough attached signers authenticate
*and* every attached policy's `enforce` succeeds. Nothing in OZ's default `SmartAccount` checks
that the specific combination of signers and policies on a rule stays jointly satisfiable after a
mutation — only that the rule has "at least one signer or one policy," a much weaker check. A
2-of-3 threshold policy can end up on a rule with one signer left; a session-key rule can end up
with zero signers while `session-policy` stays attached. Both are permanently stuck, and both are
legal under today's check. Latch documents this as a known, unresolved risk (issue #38/#77).
`warden-smart-account` closes it, generically, at the account level.

## How the fix works

`warden-smart-account` overrides `remove_signer`. Before delegating to OZ's own implementation, it
probes every policy attached to the rule for an optional `would_remain_reachable(context_rule_id,
smart_account, signer_to_remove, remaining_count) -> bool` entry point — called by raw symbol name
via `try_invoke_contract`, not a shared Rust trait bound, since `Policy` is an upstream trait this
repo doesn't own and can't extend. Two independent checks follow from that probe:

1. **A reachability-aware policy says no.** If a policy implements the query and reports the
   removal would make its own requirement unreachable (e.g. a 2-of-3 threshold with one signer
   left), the whole call reverts with `SignerRemovedWouldBreakPolicy`.
2. **An opaque policy would be left on a zero-signer rule.** A policy that doesn't implement the
   query has "no opinion" on numeric reachability — but every policy in this repo's own lineup
   unconditionally rejects an empty `authenticated_signers` list in `enforce`, so leaving one
   attached to a rule with zero signers is a *guaranteed*, not merely possible, permanent lockout.
   That combination reverts too, with `SignerRemovalWouldZeroOutPolicyGatedRule`.

`batch_add_signer` gets a smaller companion fix: it requires an explicit
`confirm_threshold_unchanged` acknowledgment, since silent ratio weakening (3-of-3 quietly becoming
3-of-5) has no on-chain "correct" answer to enforce automatically. The single-signer `add_signer`
entry point is disabled outright — its fixed trait signature has no room for that acknowledgment
parameter — callers use `batch_add_signer` even for one signer.

## Status

| Crate | State |
|---|---|
| `warden-smart-account` | ✅ Zero-signer / unreachable-threshold lockout fix, 9 passing tests (including regression replays against the real `session-policy` and `weighted-threshold-policy` crates, not just a mock) |
| `policies/threshold-policy` | ✅ Thin wrapper over OZ `simple_threshold`; implements `would_remain_reachable`; tested (2 tests) |
| `policies/weighted-threshold-policy` | ✅ Thin wrapper over OZ `weighted_threshold`; implements `would_remain_reachable`; tested (16 tests, plus 2 real-crate integration tests in `warden-smart-account`) |
| `policies/session-policy` | ✅ Wrapped, tested (14 tests) — opaque to reachability, covered by branch 2 above |
| `policies/spending-limit-policy` | ✅ Wrapped, tested (17 tests) — opaque to reachability, covered by branch 2 above |
| `policies/time-window-policy` | ⬜ Not started — new policy |
| `policies/call-count-limit-policy` | ⬜ Not started — new policy |
| `policies/approval-delay-policy` | ⬜ Not started — new policy |
| `account-factory` | ⬜ Not started |
| `warden-verifiers/*` (ed25519, p256, secp256k1, webauthn) | ⬜ Not started |
| `fee-forwarder` | ⬜ Not started |

## Repository structure

```
warden-contracts/
├── account-factory/
│   └── contracts/factory-contract/
├── warden-smart-account/            # ✅ the invariant fix lives here
├── warden-verifiers/
│   ├── ed25519-verifier/
│   ├── p256-verifier/
│   ├── secp256k1-verifier/
│   └── webauthn-verifier/
├── policies/
│   ├── threshold-policy/            # ✅
│   ├── weighted-threshold-policy/   # ✅
│   ├── session-policy/              # ✅
│   ├── spending-limit-policy/       # ✅
│   ├── time-window-policy/          # new
│   ├── call-count-limit-policy/     # new
│   └── approval-delay-policy/       # new
├── fee-forwarder/
└── docs/
    ├── smart-account-authz-spec.md  # full design: invariant fix + 3 new policies
    └── repo-architecture-spec.md    # three-repo split, interfaces
```

## Relationship to Latch

Warden is not a fork and does not claim to replace Latch — it's an independent implementation on
the same OZ base layer, built to test a specific fix and a specific set of policy additions. Where
a design choice here matches Latch's own convention (stating out-of-scope items explicitly,
validate-at-install, atomic revert-on-failure), that's intentional reuse of a pattern that already
works, not incidental similarity.

## Scope

**In scope for v1:**
- The `remove_signer` / `batch_add_signer` invariant enforcement described above.
- Three new policy crates: `time-window-policy`, `call-count-limit-policy`,
  `approval-delay-policy`.

**Explicitly out of scope for v1** (named, not silently skipped):
- Full symbolic/semantic satisfiability checking for arbitrary custom policies — only a policy
  that implements `would_remain_reachable` gets a real answer; everything else is "opaque" and
  falls back to the zero-signer floor described above, not treated as safe by default.
- Guardian/social recovery.
- New verifier types (BLS/RSA/ZK/email).

## Development

Prerequisites: Rust (`rustup`), Stellar CLI v25.2.0+ (`cargo install --locked stellar-cli`).

```sh
cargo +nightly fmt --all -- --check     # whole workspace
cargo test --workspace                  # all crates
cd <crate> && cargo clippy --all-targets --all-features -- -D warnings
cd <crate> && stellar contract build
```

## Resolved questions

**`weighted-threshold-policy`'s `would_remain_reachable` cost.** The original design sketch
(`min_signers_required`, "smallest signer count whose weight meets threshold") would have been
knapsack-shaped and needed benchmarking. The mechanism that actually shipped only asks "is the
maximum possible remaining weight still >= the threshold" — a single O(n) sum over the installed
`signer_weights` map with the removed signer excluded, since `enforce` accepts whichever signers
choose to authenticate and the best case is all of them. No search space, no cap needed. See
`policies/weighted-threshold-policy/src/lib.rs`.

## Open questions

1. **Where threshold changes actually live.** `set_threshold` / `set_signer_weight` are exposed by
   the *policy* contracts today, not the account, so a threshold shrink made directly through the
   policy contract isn't caught by the account-side `remove_signer` override above. Needs a
   decision: policy calls back into the account to re-validate, or the account gets an
   event-triggered post-mutation sweep.

## Deployments

`deployments/testnet.json` — addresses and WASM hashes for what's live on Stellar testnet so far:
`threshold-policy` (shared singleton) and a demo `warden-smart-account` instance (single delegated
signer, no policies, deployed directly since `account-factory` doesn't exist yet). This is what
`warden-backend`'s indexer and `warden-frontend`'s account viewer point at for their own first
milestones.

## Issue backlog

[`issues.md`](./issues.md) — 125 scoped issues across all three Warden repos (55 here, 40 in
`warden-backend`, 30 in `warden-frontend`), sized with Drips' Trivial/Medium/High complexity tiers
for seeding GitHub Issues.

## Related repos

Part of the Warden project — [`warden-backend`](https://github.com/Osok-Labs/warden-backend)
(relayer, indexer, API) and [`warden-frontend`](https://github.com/Osok-Labs/warden-frontend)
(web extension / dApp) consume this repo's deployed addresses and generated bindings, and talk to
it only through those published artifacts — never a shared repo or shared secrets.

## License

[MIT](./LICENSE) — same as Latch.
