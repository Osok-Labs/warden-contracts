# Warden Issue Backlog

125 candidate issues across the three Warden repos, sized for seeding GitHub Issues ahead of
a [Drips Wave](https://www.drips.network/wave) application — each row maps to one issue, labeled
with Drips' own complexity tiers so Points can be assigned directly:

| Complexity | Points |
|---|---|
| Trivial | 100 |
| Medium | 150 |
| High | 200 |

Grounded in each repo's actual current state (see each repo's README "Status" table) — nothing
here is generic filler; titles reference real crates, files, and open questions already on record.
To use: pick a row, open it as a GitHub issue in the named repo with the title as-is and the note
as the issue body, apply the complexity label.

**Totals:** `warden-contracts` 55 · `warden-backend` 40 · `warden-frontend` 30 · **125**

---

## warden-contracts (55)

### time-window-policy (8)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 1 | Medium | Scaffold `time-window-policy` crate | Cargo.toml + module doc, mirroring `session-policy`'s layout. |
| 2 | High | Implement `install`/`enforce`/`uninstall` | `window_start_offset` + `period_seconds` over UTC ledger timestamps — no on-chain timezone data, by design. |
| 3 | Medium | Add error variants + `get_window_config` query | `InvalidWindow`, `OutsideWindow`, matching `spending-limit-policy`'s error-code style. |
| 4 | Medium | Implement `set_window` | Same "reject if it breaks the rule" discipline as `spending-limit-policy::set_spending_limit`. |
| 5 | Trivial | Document `would_remain_reachable` stance | Almost certainly opaque (no numeric reachability) — record the reasoning in the module doc. |
| 6 | Medium | Write unit test suite | Mirror `spending-limit-policy`'s ~17-test shape: install validation, inside/outside window, boundary ledger, not-installed, uninstall. |
| 7 | Trivial | Verify WASM build + wire into workspace | `stellar contract build`, add to root `Cargo.toml` members. |
| 8 | Medium | Add real-crate integration test in `warden-smart-account` | Same pattern as the `session-policy`/`weighted-threshold-policy` regression tests. |

### call-count-limit-policy (8)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 9 | Medium | Scaffold `call-count-limit-policy` crate | |
| 10 | High | Implement `install`/`enforce`/`uninstall` | Reuse `spending-limit-policy`'s rolling-window eviction pattern, counting calls instead of amounts. |
| 11 | Medium | Add error variants + `get_call_count_data` query | `CallCountExceeded`, `InvalidLimitOrPeriod`. |
| 12 | Medium | Implement `set_call_limit` | Same validate-on-change discipline as `spending-limit-policy`. |
| 13 | Trivial | Document `would_remain_reachable` stance | Opaque, same reasoning as `time-window-policy`. |
| 14 | Medium | Write unit test suite | Install validation, enforce under/at/over limit, rolling-window eviction, not-installed, uninstall. |
| 15 | Trivial | Verify WASM build + wire into workspace | |
| 16 | Medium | Add real-crate integration test in `warden-smart-account` | |

### approval-delay-policy (10)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 17 | High | Design the propose/approve/execute state machine | Including the `PendingAction` storage schema. |
| 18 | High | Implement `propose()` | Signs + stores the exact call's context digest, establishing the pending action. |
| 19 | High | Implement `approve()` | Co-signer approval; enforce a minimum delay in ledgers before it's executable. |
| 20 | High | Implement `execute()` | Re-submitting the original call must produce a byte-identical encoded context or it's rejected. |
| 21 | Medium | Add `PendingAction`/`Approved`/`Executed`/`Expired` events | `#[contractevent]`, for `warden-backend`'s indexer. |
| 22 | Medium | Implement `would_remain_reachable` returning `Some(2)` | Matches the project README's design table (propose + approve signer). |
| 23 | Medium | Add pending-action expiry (TTL) | So a proposed-but-never-approved action doesn't squat storage forever. |
| 24 | Medium | Write unit test suite | Happy path, premature execute (before delay elapses), mismatched re-submission rejection. |
| 25 | Trivial | Verify WASM build + wire into workspace | |
| 26 | Medium | Add real-crate integration test in `warden-smart-account` | |

### account-factory (10)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 27 | High | Scaffold `account-factory/contracts/factory-contract/` | Cargo.toml + module doc referencing Latch's own factory as the base pattern. |
| 28 | High | Implement deterministic address derivation | Same signer set → same address, matching Latch's property. |
| 29 | Medium | Implement canonical signer-input sorting | Signer order shouldn't affect the derived address. |
| 30 | High | Implement `create_account()` | Deploys a new `warden-smart-account` instance, passing pre-deployed verifier/policy addresses. |
| 31 | Medium | Implement idempotent `create_account` | Same params twice returns the existing account, doesn't redeploy. |
| 32 | Medium | Implement `account_salt` support | Same signer set can own multiple accounts. |
| 33 | Medium | Add factory-level `AccountCreated` event | For `warden-backend`'s indexer. |
| 34 | Medium | Write unit test suite | Determinism, idempotency, canonical ordering. |
| 35 | Trivial | Verify WASM build | Record the deployed factory address in `deployments/testnet.json`. |
| 36 | Medium | Deploy factory to testnet, redeploy the demo account through it | Replaces the direct-deploy path currently recorded. |

### warden-verifiers (12)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 37 | Medium | Implement `ed25519-verifier` | Raw hash, no wrapping — matches Latch's shape. |
| 38 | Trivial | Unit tests + WASM build for `ed25519-verifier` | |
| 39 | Trivial | Deploy `ed25519-verifier` to testnet | Record address in `deployments/testnet.json`. |
| 40 | Medium | Implement `p256-verifier` | Raw P-256 session/external signers, 65-byte uncompressed SEC1 key. |
| 41 | Trivial | Unit tests + WASM build for `p256-verifier` | |
| 42 | Trivial | Deploy `p256-verifier` to testnet | |
| 43 | High | Implement `secp256k1-verifier` | Low-S `r\|\|s\|\|recovery_id` verification; document the raw-recovery-ID (not Ethereum 27/28) client normalization requirement. |
| 44 | Medium | Unit tests + WASM build for `secp256k1-verifier` | |
| 45 | Trivial | Deploy `secp256k1-verifier` to testnet | |
| 46 | High | Implement `webauthn-verifier` | P-256 key + credential ID, full WebAuthn ceremony check. |
| 47 | Medium | Unit tests + WASM build for `webauthn-verifier` | |
| 48 | Trivial | Deploy `webauthn-verifier` to testnet | |

### fee-forwarder (5)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 49 | High | Implement `fee-forwarder` | Thin wrapper over OZ's `stellar-fee-abstraction` helpers, following `examples/fee-forwarder-permissioned`. |
| 50 | Medium | Implement `enable_fee_token`/`disable_fee_token`/`sweep_tokens` | Manager-gated entry points. |
| 51 | Medium | Wire the `executor` role | Only `warden-backend`'s relayer credential can call `forward()`. |
| 52 | Medium | Write unit test suite | Authorized forward succeeds, unauthorized executor rejected, fee cap respected. |
| 53 | Trivial | Deploy `fee-forwarder` to testnet | Record address in `deployments/testnet.json`. |

### Infra / open questions (2)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 54 | Medium | Resolve "where threshold changes actually live" | Decide policy-calls-back-into-account vs. event-triggered post-mutation sweep, and implement it — the one remaining open question in the README. |
| 55 | Trivial | Move `idea2.md`/`idea3.md` into `docs/` | `docs/smart-account-authz-spec.md` and `docs/repo-architecture-spec.md`, as the top-level `project.md` already says is pending; update cross-links. |

---

## warden-backend (40)

### Persistence (6)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 56 | Medium | Add sqlite-backed storage for indexed events | Replaces the in-memory `Vec<IndexedEvent>` in `state.rs`. |
| 57 | Medium | Persist the `getEvents` cursor across restarts | So the indexer resumes instead of re-scanning the lookback window. |
| 58 | Trivial | Add `WARDEN_DB_PATH` config option | Default `./warden-indexer.sqlite`, alongside the existing env-var config pattern. |
| 59 | Medium | Write a migration/schema-init step | Run on startup. |
| 60 | Medium | Add an index on `(contract_id, ledger)` | For efficient per-contract queries once per-account filtering lands. |
| 61 | Trivial | Document the persistence model in the README | Update the Status table's "Persistent storage" row. |

### API split (4)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 62 | Medium | Split the HTTP surface into its own `api/` crate | Per the target layout — indexer writes, API reads from the shared store. |
| 63 | Trivial | Add `GET /events/:contract_id` filtered endpoint | Now that storage supports efficient per-contract queries. |
| 64 | Medium | Add response pagination to `GET /events` | Cursor- or offset-based, instead of the full unbounded list. |
| 65 | Trivial | Add OpenAPI/JSON schema docs | For the API surface. |

### Relayer (8)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 66 | High | Scaffold `relayer/` crate | Accepts a signed `forward()` authorization tree from a client. |
| 67 | High | Implement `fee_amount` filling logic | Real fee ≤ the client's signed cap, before submission. |
| 68 | High | Implement executor-credential key custody | Secrets manager integration, never committed — per the Trust boundary section. |
| 69 | Medium | Implement submission + confirmation polling | Reuse `stellar-rpc-client`'s `send_transaction_polling`. |
| 70 | Medium | Add relayer-side rate limiting / abuse protection | So it can't be used to drain XLM via spam. |
| 71 | Medium | Add relayer events/logging | Submitted `forward()` calls, queryable by the API. |
| 72 | Medium | Write unit + integration tests | Against a real deployed `fee-forwarder` on testnet. |
| 73 | Trivial | Document relayer deployment/runbook | In the README. |

### Shared bindings (4)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 74 | Medium | Add a `shared/` crate with generated Rust bindings | `stellar contract bindings rust`, replacing the ad hoc `stellar-rpc-client` + raw `ScVal` decoding in `poller.rs`. |
| 75 | Medium | Regenerate bindings in CI | Whenever `warden-contracts` ships a new deployment. |
| 76 | Trivial | Publish the bindings crate's version alongside `deployments/testnet.json` | So backend/frontend can pin to a matching version. |
| 77 | Medium | Replace `poller.rs`'s `Debug`-string event decoding | Typed decoding via the generated bindings' event types. |

### Per-account filtering & factory integration (3)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 78 | Medium | Add `GET /accounts/:address/events` | Once `account-factory` exists and produces more than one account. |
| 79 | Medium | Index `AccountCreated` events from the factory | Builds the address→account registry the filtering above needs. |
| 80 | Trivial | Update README's "Per-account filtering" Status row | ⬜ → ✅ once done. |

### Testing & CI (6)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 81 | High | Add an integration test against a local Soroban network (or recorded RPC fixture) | Asserts events are correctly indexed and served end-to-end. |
| 82 | Medium | Unit test `poller.rs`'s `decode_scval` fallback | Malformed base64, valid XDR. |
| 83 | Medium | Unit test `api.rs` handlers | `health`, `events`, via axum's test utilities. |
| 84 | Trivial | Set up GitHub Actions CI | `cargo build`, `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`. |
| 85 | Trivial | Add a Dockerfile | For the indexer/api services. |
| 86 | Medium | Add a `docker-compose.yml` | Wires indexer + (future) relayer + a local Soroban network for full local dev. |

### Operational hardening (9)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 87 | Medium | Add exponential backoff + jitter on RPC poll failures | Instead of a fixed `poll_interval_secs` retry. |
| 88 | Medium | Replace `eprintln!` logging with structured logging | `tracing` + `tracing-subscriber`. |
| 89 | Medium | Add basic metrics | Events indexed, poll latency, poll error rate (Prometheus-style). |
| 90 | Trivial | Add CORS configuration to the axum router | So `warden-frontend` can call it from the browser, not just server-side. |
| 91 | Trivial | Add a `/readiness` endpoint distinct from `/health` | Ready = has completed at least one successful poll. |
| 92 | Medium | Add graceful shutdown | SIGTERM handling so the poller and HTTP server both drain cleanly. |
| 93 | Medium | Add request timeouts + a max-body-size guard | On the axum router. |
| 94 | Trivial | Pin exact dependency versions | Same reproducibility discipline `warden-contracts` uses for `stellar-accounts`. |
| 95 | Medium | Add an indexer backfill mode (`--from-ledger`) | Resolves the "backfill from genesis vs. forward-only" open question still in the README. |

---

## warden-frontend (30)

### Wallet integration + account creation (6)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 96 | High | Integrate Freighter (or Stellar Wallets Kit) | Browser-side transaction signing. |
| 97 | High | Build the account-creation flow | Construct a deploy transaction (direct today, via `account-factory` once that lands) and get it signed by the connected wallet. |
| 98 | Medium | Add a "connect wallet" UI state | Connected / disconnected / wrong network, surfaced across the app. |
| 99 | Medium | Handle and surface signing rejections / submission failures | Clear error states, not silent failures. |
| 100 | Medium | Redirect to the new account's viewer page after creation | Depends on issue #124 (multi-account routing). |
| 101 | Trivial | Update README's "Create an account" Status row | ⬜ → ✅ once done. |

### Policy setup UIs (8)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 102 | High | Build the `time-window-policy` setup UI | Convert the user's local-timezone window into the contract's UTC-relative params before signing install. |
| 103 | Trivial | Add a read view for an installed `time-window-policy` | Converts stored UTC window back to local time for display. |
| 104 | High | Build the `approval-delay-policy` two-phase flow UI | Proposer signs once; a co-approver sees a "pending approvals" inbox (fed by `warden-backend`) to review and sign `approve`. |
| 105 | Medium | Solve the "byte-identical re-submission" UX | The proposer's client must reconstruct and re-submit the exact original call after approval. |
| 106 | Medium | Build the `call-count-limit-policy` dashboard | "3 of 5 automated calls used today," backed by the API's indexed policy state. |
| 107 | Trivial | Build the `spending-limit-policy` dashboard | Spent vs. limit within the rolling window. |
| 108 | Medium | Build the session-key creation flow | Install `session-policy` + a `CallContract` rule with a fresh ephemeral signer scoped to one dApp. |
| 109 | Trivial | Build a policy-management view | Lists all policies currently installed on the viewed account. |

### Generated TS bindings / shared (2)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 110 | Medium | Add a `shared/` package with generated TypeScript bindings | `stellar contract bindings typescript`, replacing the hand-typed `WardenSmartAccountContract` interface in `src/lib/warden.ts`. |
| 111 | Trivial | Wire the bindings package version to `deployments/testnet.json` | Same pattern suggested for `warden-backend`. |

### Browser extension (3)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 112 | High | Scaffold `extension/` | Browser extension signing surface — manifest, build tooling. |
| 113 | Medium | Share UI components between `web-app/` and `extension/` | Via the `shared/` package. |
| 114 | Medium | Implement the extension's own signing popup flow | Session-key/account-management actions. |

### Testing & CI (4)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 115 | Medium | Add unit tests for `src/lib/warden.ts` and `src/lib/indexer.ts` | Mock the RPC/indexer calls. |
| 116 | Medium | Add an end-to-end test (Playwright) | Loads the account viewer against a local/testnet deployment, asserts real data renders. |
| 117 | Trivial | Set up GitHub Actions CI | `npm run lint`, `npm run build`, `npm test`. |
| 118 | Trivial | Add a pre-commit hook (or CI check) for eslint + prettier | |

### UX polish (5)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 119 | Medium | Add a loading skeleton for the account/events sections | Instead of a blank page while the server component awaits data. |
| 120 | Medium | Add an error boundary around the account-view section | So an RPC failure doesn't take down the whole page. |
| 121 | Trivial | Audit color contrast / accessibility | The current `page.module.css` dark-mode block, against WCAG AA. |
| 122 | Trivial | Verify and fix layout at phone width (~375px) | Currently untested below desktop. |
| 123 | Trivial | Add a favicon/OG image reflecting the Warden project | Instead of the default Next.js one. |

### Multi-account support / routing (2)

| # | Complexity | Title | Notes |
|---|---|---|---|
| 124 | Medium | Replace the hardcoded `WARDEN_DEMO_ACCOUNT_ID` with a dynamic `/accounts/[address]` route | |
| 125 | Trivial | Add a "look up an account by address" input on the homepage | |
