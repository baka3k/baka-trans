# Gemini Translation Engine + Batch Translation Optimization — 2026-10-14

Delivered by commit `b8217a46f8eb2955621787eadebf224040167a31`
("feat(local): add Gemini translation engine with batched JSON-array
requests"), implementing
`plans/261014-1537-gemini-translation-engine/plan.md` (phase 16.6 of the
local-LLM umbrella) with its three phase docs
(`phase-01-gemini-engine-backend.md`, `phase-02-gemini-settings-ui.md`,
`phase-03-batch-translation-optimization.md`).

## Context

The Local Whisper workspace had exactly two translation engines: the managed
offline Hy-MT2 / TranslateGemma 4B models and an OpenAI-compatible API. The
per-sentence hot loop in `spawn_translation_worker` pulled one
`TranslationJob` at a time and called `client.translate(&str)`, so every ASR
fragment was translated in isolation — no cross-chunk context, no stitching
of awkwardly cut fragments. Meanwhile the OCR overlay already talked to
Google's `generativelanguage` REST API (`src-tauri/src/overlay.rs`) using the
shared Google credential (`google-gemini-api-key` / `GEMINI_API_KEY`,
`src-tauri/src/security.rs`), but the `?key=` query-param transport used
there leaks credentials into URLs and logs. This change adds Gemini as the
third engine with a batched JSON-array contract and a cabin-interpreter
system prompt, without disturbing the two existing engines.

## Change

### Backend engine (`src-tauri/src/local_translation/gemini.rs`, new)

- `GeminiClient` (765 lines) POSTs
  `{API_BASE}/models/{model}:generateContent` with one JSON array of Whisper
  sentences per request; `translate()` rides the same batch contract with a
  single element (`gemini.rs:96-108`, `gemini.rs:110`).
- Request body: `system_instruction` = parameterized cabin-interpreter prompt
  (`gemini.rs:283`), `generationConfig` with `temperature 0.3`
  (`gemini.rs:15,129`), `response_mime_type: "application/json"` (JSON mode),
  and `thinkingConfig: { thinkingBudget: 0 }` (`gemini.rs:134`).
- Auth via the `x-goog-api-key` header (`gemini.rs:144`), never `?key=`; the
  engine-test probe `probe_key` lists `GET /v1beta/models` with the same
  header (`gemini.rs:211,228`); a request-shape test asserts the key never
  rides the query string (`gemini.rs:464-467`).
- Parse path: `candidates[0].content.parts[].text` joined, code-fence strip
  (`gemini.rs:373`), strict `Vec<String>` array parse (`gemini.rs:346`); a
  length mismatch maps returned indexes 1:1, missing indexes get
  `local_gemini_batch_length_mismatch`, extras are truncated.
- Every error message passes through `redact_message` (`gemini.rs:397`) —
  exact-match, key-tail, and prefix-shape scrubbing; `Debug` impls for the
  client never carry the key (`gemini.rs:29-34`).
- Model normalization `normalize_gemini_model` (`gemini.rs:266`): trim, strip
  a leading `models/`, restrict to URL-path-safe charset, empty/invalid →
  `DEFAULT_MODEL` = `gemini-2.5-flash-lite` (`gemini.rs:8`).
- 1 MiB response cap, 30 s timeout, `redirect(Policy::none())`
  (`gemini.rs:11-12`).

### Dispatcher + gate (`src-tauri/src/local_translation.rs`)

- `TranslationClient::Gemini` variant (`local_translation.rs:368`); the
  construction arm loads the shared Google credential via
  `security::load_translation_api_key(GoogleLiveTranslate)`
  (`local_translation.rs:516-526`).
- `translate_batch` dispatch: Gemini fans to the native batch client; the
  OpenAI-compatible and Hy-MT arms loop `translate()` per item — the
  sequential default that keeps their behavior byte-for-byte
  (`local_translation.rs:542-562`).
- `batch_window()` returns `Some((5, 500 ms))` only for Gemini
  (`local_translation.rs:565-570`, constants at `:36-37`).
- Start-time gate `enforce_runtime_key_gate` now covers the always-hosted
  Gemini engine (`local_translation.rs:594-610`);
  `ensure_gemini_key_available` (`:626`) fails with
  `local_gemini_api_key_missing` and `gemini_key_missing_message`
  (`gemini.rs:279`) names both fixes: save a key in the Cloud settings panel
  or set `GEMINI_API_KEY`.
- Engine test: full-pipeline one-sentence probe for Test Local Translation
  (`local_translation.rs:843`), free credential-only `GET /v1beta/models`
  probe for the Test Translation Engine button (`:972`), with reachability
  mapped so request-level failures are not reported as "reachable".
- Config schema v4 → v5 (`local_translation.rs:25`):
  `migrate_v4_gemini_model_config` (`:1373`, dispatch at `:1323`) relies on
  serde per-field defaults for the new `geminiModel`, normalizes it, and
  leaves every stored value (including `useGpu`, already settled by v3→v4)
  untouched. Covered by `migrates_v4_config_with_default_gemini_model`
  (`:2150`).
- New tests: `batch_window_only_applies_to_gemini` (`:2186`),
  `runtime_key_gate_requires_google_key_for_gemini` (`:2237`), request-shape
  and parse tables in `gemini.rs`.

### Worker: batch loop + Runtime genericization (`src-tauri/src/ai/local_worker.rs`)

- `spawn_translation_worker` branches on `client.batch_window()`
  (`local_worker.rs:454-500`): `None` → `run_single_translation_loop`
  (`:508`, the pre-batching loop extracted verbatim for Hy-MT2/OpenAI);
  `Some` → `run_batched_translation_loop` (`:597`).
- `collect_batch_window` (`:756`) drains already-queued jobs with `try_recv`,
  then waits out the remaining ≤500 ms max-wait polling cancellation in
  25 ms ticks (`BATCH_WAIT_POLL_TICK`, `:749`) — a cancelled session returns
  an empty window and never fires the paid request; a disconnected producer
  flushes what is held.
- The batch loop emits per-item Final/Error snapshots in arrival order via
  `batch_snapshot_item` (`:682`, revision 2, per-job `started` latency) and
  shared `enqueue_tts_request` (`:709`); batch failure marks every job in
  the window — no retry, no engine fallback.
- The worker is genericized over `tauri::Runtime`
  (`SentenceTranslator<R>`, `local_worker.rs:70`; also `settle_pipeline_activity`
  `:897`, `emit_snapshot` `:996`, `session.rs:518`), so tests drive the real
  loop with a `tauri::test::mock_app()` (`local_worker.rs:1859,2145`) and the
  new test-only loopback capture server
  (`src-tauri/src/local_translation/test_http.rs:24`), which records the raw
  request and answers one canned response — a second request fails to
  connect, letting tests assert exactly how many engine calls ran.

### Credential commands (`src-tauri/src/commands.rs`, `security.rs`, `lib.rs`, `api.ts`)

- New provider-generic `delete_translation_api_key`
  (`commands.rs:102` → `security.rs:37`: keyring `delete_credential`,
  `NoEntry` → `Ok`, cache clear), registered in `lib.rs:61` and wrapped in
  `src/api.ts:82`. Added because the Gemini key row needs a working Clear,
  and no cloud-panel delete command existed.

### Frontend (`src/components/settings/LocalLlmSettings.tsx`, `MainApp.tsx`, `types.ts`)

- Third engine option "Gemini (Google AI)" (`LocalLlmSettings.tsx:331`) with
  a conditional field group: model input (`:366-374`), egress advisory
  ("sends your meeting transcript to Google's Generative Language API",
  `:379`), and a status-only Google key row save/cleared via the
  provider-generic commands (`:382-386`; handlers in `MainApp.tsx:1071-1095`
  using `google_live_translate`). The key is never echoed.
- Client validation mirrors the Rust normalizer
  (`LocalLlmSettings.tsx:210-212`); `LocalTranslationEngine` gains
  `"gemini"` (`src/types.ts:229`) and `geminiModel` joins the config types
  (`types.ts:102,201`).

### Bundled pre-existing fix (`src-tauri/src/local_translation/api_key.rs`)

- The HuggingFace env tests deadlocked the full `cargo test` suite on this
  macOS: each test took `env_lock()` itself and then called
  `EnvSnapshot::capture()`, which takes the same non-reentrant std
  `Mutex` again (`api_key.rs:423-426`). The outer lock is removed from the
  three tests (`api_key.rs:365-370` carries the explanation; `:370,391,401`
  now rely on the snapshot's lock alone).

### Docs

- `README.md` and `ARCHITECTURE.md` engine lists gain Gemini with the
  batch note (windows of 5, ≤500 ms), the shared-credential note, and a
  `local_gemini_api_key_missing` troubleshooting row. `session.rs` /
  `hy_mt.rs` deltas are the Runtime genericization and `cargo fmt`.

## Impact

**Medium.** The config schema migrates v4 → v5 for every local-translation
user and the shared translation-worker hot loop is restructured into two
loops, but exposure is contained: the v5 migration is additive (all stored
values pass through untouched), the offline and OpenAI-compatible engines
keep the extracted single-sentence loop verbatim (regression-tested via the
existing worker tests), the credential is reused rather than duplicated (no
new keychain entry), and transcript/event contracts are unchanged (per-item
snapshots, revision 2, TTS backlog semantics preserved). The Gemini engine
itself is new surface, off by default (`huggingface_offline` remains the
default engine), and gated at session start. All gates ran: `cargo test`
(unblocked by the bundled deadlock fix), `cargo fmt --check`, `cargo clippy
-D warnings`, `npm test`, `npm run build`.

## Decision

1. **Batch ships Gemini-first only.** `TranslationClient` gained
   `translate_batch` + `batch_window()` with a sequential per-item default so
   Hy-MT2/OpenAI keep today's latency and per-item error semantics exactly
   (`src-tauri/src/local_translation.rs:542,565`; loops in
   `src-tauri/src/ai/local_worker.rs:508,597,756`). The worker engages
   batching only for engines that declare a window; the OpenAI-compatible
   prompt is untouched. Window = 5 sentences / 500 ms max wait as engine
   constants, tying context retention to the 1-3 s end-to-end promise
   (`local_translation.rs:32-37`).
2. **Credential reuse over a new keyring entry.** The local Gemini engine
   shares the Cloud panel's Google key (`google-gemini-api-key` /
   `GEMINI_API_KEY`) — the OCR overlay already uses it, so a dedicated entry
   would force two keys for one Google account
   (`local_translation.rs:516-526`). The gate error
   `local_gemini_api_key_missing` names both fixes so a key cleared in the
   Cloud panel does not strand local users (`gemini.rs:279`). The new
   provider-generic `delete_translation_api_key` command
   (`commands.rs:102`, `security.rs:37`) exists because the Gemini key row
   needs Clear and no delete path existed for cloud providers.
3. **Header auth mandated; `?key=` migration deferred.** The new client
   authenticates with `x-goog-api-key` (`gemini.rs:144,228`) because query
   params leak into URLs and logs; tests assert the key never rides the
   query string (`gemini.rs:464-467`). Migrating the overlay's `?key=`
   usage (`overlay.rs`) is explicitly out of scope (plan Scope Decision 5)
   and remains follow-up work.
4. **Schema v4 → v5 with a normalized, defaulted `geminiModel`.**
   `migrate_v4_gemini_model_config` adds only the new field
   (`local_translation.rs:1373`); model names are trimmed, `models/`-stripped,
   charset-restricted to URL-path-safe characters (they are interpolated
   into the endpoint path), and empty/invalid falls back to
   `gemini-2.5-flash-lite` (`gemini.rs:8,266-276`). The frontend validation
   mirrors the Rust normalizer (`LocalLlmSettings.tsx:210`).
5. **`thinkingConfig.thinkingBudget: 0` set after verifying current Google
   docs** that 2.5 Flash-Lite supports disabling thinking, keeping
   interpretation latency flat for the ≤500 ms-window realtime pipeline
   (`gemini.rs:130-135`, asserted in the parse tests at `gemini.rs:512`).
6. **Bundled the pre-existing `api_key.rs` env-test deadlock fix** because it
   blocked the full `cargo test` gate on this macOS: the outer `env_lock()`
   was taken twice (test + `EnvSnapshot::capture`) on a non-reentrant std
   `Mutex` (`api_key.rs:365-370,423-426`). Not Gemini-specific, but a
   prerequisite for shipping green gates.
7. **Worker genericized over `tauri::Runtime`** (`local_worker.rs:70,454`;
   `session.rs:518`) so the real batch loop can be driven end-to-end in
   tests with a `MockRuntime` app (`local_worker.rs:1859`) and the loopback
   capture server (`src-tauri/src/local_translation/test_http.rs`), instead
   of mocking the client and losing request-count/order assertions.

## References

- Commit: `b8217a46f8eb2955621787eadebf224040167a31`
- Plan: `plans/261014-1537-gemini-translation-engine/plan.md` (+ phase-01/02/03 docs)
- Parent plan: `plans/260716-2033-local-llm-audio-translation/plan.md` (phase 16.6)
- Key files: `src-tauri/src/local_translation/gemini.rs`,
  `src-tauri/src/local_translation.rs`, `src-tauri/src/ai/local_worker.rs`,
  `src-tauri/src/local_translation/test_http.rs`,
  `src-tauri/src/commands.rs`, `src-tauri/src/security.rs`,
  `src-tauri/src/local_translation/api_key.rs`,
  `src/components/settings/LocalLlmSettings.tsx`, `src/app/MainApp.tsx`
- Prior art: overlay Gemini text translation `src-tauri/src/overlay.rs`
  (`?key=` transport this change deliberately does not copy); Google
  credential storage `src-tauri/src/security.rs`
- Follow-ups noted: overlay `?key=` → header migration; Gemini
  batch-capability for other engines explicitly out of scope.
