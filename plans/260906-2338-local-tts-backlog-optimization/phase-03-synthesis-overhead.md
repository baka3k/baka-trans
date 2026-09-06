# Phase 03 — Synthesis Overhead Reductions

Parent: [plan.md](plan.md)

Covers user-selected optimization **4**. Phases 01-02 make the pipeline
tolerant of synthesis latency; this phase shrinks it.

## Objective

Cut per-sentence synthesis overhead on both desktop platforms without
changing the `tts::synthesize` contract
(`(Option<&AppHandle>, &str, &LocalTranslationConfig, Arc<AtomicBool>) ->
AppResult<SynthesizedAudio>`), its callers, or the 24 kHz PCM16 output
contract.

## Windows — cache the `SpeechSynthesizer` (`src-tauri/src/tts.rs`)

Today every sentence does `SpeechSynthesizer::new()` (WinRT activation / COM
init) plus a full `AllVoices()` enumeration scan to resolve the voice id
(`tts.rs:448-498`). Cache the configured instance:

```rust
#[cfg(target_os = "windows")]
struct CachedSynthesizer {
    synthesizer: windows::Media::SpeechSynthesis::SpeechSynthesizer,
    configured_voice_id: String,
}

#[cfg(target_os = "windows")]
static CACHED_SYNTHESIZER: std::sync::OnceLock<
    std::sync::Mutex<Option<CachedSynthesizer>>,
> = std::sync::OnceLock::new();
```

- Pattern precedent: process-global `OnceLock<Mutex<Option<T>>>` at
  `src-tauri/src/llm.rs:20` and `src-tauri/src/security.rs:8-9`. NOT Tauri
  managed state (keeps `tts::synthesize(None, ...)` test entry working).
- `SpeechSynthesizer` is `Send + Sync` in windows 0.61 (unsafe impls in the
  generated registry, `Media/SpeechSynthesis/mod.rs:376-377`), so holding it
  in a std `Mutex` inside a `OnceLock` is sound.
- **Lock discipline — never hold the std `Mutex` across an `.await`.** Use a
  short critical section + handle clone:
  `let synth = { let guard = lock(); let entry = get_or_create(guard);
  if voice_needs_reload(...) { rescan + SetVoice }; entry.synthesizer.clone() };`
  — WinRT interface pointers are reference-counted, so `.clone()` is an
  `AddRef` (cheap, no device re-init), and the await on
  `SynthesizeTextToStreamAsync` then runs on the cloned handle with no lock
  held. Because the cache always holds its own reference, a synthesis in
  flight keeps the instance alive even if another thread later swaps the
  entry.
- **Poisoning:** a panic while holding the lock poisons it; on
  `Err(PoisonError)`, rebuild the cache entry from scratch (drop the poisoned
  guard via `into_inner()`, create a fresh synthesizer) rather than
  propagating a permanent failure.
- **Orphaned in-flight synthesis guard (red-team finding):** Windows
  synthesis checks `cancelled` only after its awaits (`tts.rs:548, 602`);
  an aborted worker task leaves the WinRT operation running on the cached
  instance, and phase 01+02 make stop-during-synthesis common (backlogs
  exist). A later session cloning and using the same instance while the
  orphaned `SynthesizeTextToStreamAsync` is still executing is unserialized
  WinRT territory. Guard: add an `in_flight: AtomicBool` to the cache entry;
  set it around the await, and on acquire, if already set, use a fresh
  throwaway `SpeechSynthesizer` for that call instead of the cached one.
- **Voice change re-apply:** compare `configured_voice_id` with
  `config.voice_id` on every call; only re-run the `AllVoices()` scan +
  `SetVoice` on mismatch or first use. Also re-apply `SpeakingRate` /
  `AudioVolume` from config on every call (cheap property sets, keeps
  `tts_rate`/`tts_volume` changes immediate) — same as today's behavior.
- **Accepted behavior change (documented):** if the configured voice is
  uninstalled system-wide while the cache holds it, the id comparison still
  matches, the rescan is skipped, and speech continues with the previous
  voice — today's per-sentence scan instead fails loudly with
  `local_tts_voice_missing` (`tts.rs:492-497`). Accepted: the cache is
  session-lived in practice and the settings surface re-lists voices; note
  it in the phase-04 log.
- `list_voices()` (`tts.rs:377-441`) stays as-is: it is a settings action,
  not a per-sentence path.

## macOS — keep per-call unique temp files (red-team rejection)

An earlier draft proposed reusing one fixed `baka-trans-tts.wav` path.
REJECTED after red-team trace: the drain ladder aborts the tokio wrapper but
NOT the `spawn_blocking` thread running `say -o <path>` (`tts.rs:666-718`),
so an orphaned `say` from a stopped session can still be writing the fixed
path while a newly started session's synthesis reads it — and the orphan's
final `remove_file` can delete the file mid-read, corrupting the new
session's audio (`local_tts_audio_format_error`). Today's per-call UUID
names make that impossible.

macOS scope is therefore reduced to: keep the UUID temp file as-is and
delete the now-unused `uuid` import ONLY if another phase removes it (it
does not — no macOS change in this phase). macOS synthesis latency is
addressed by phase 02's pipelining, not here; `say` process spawn is
unavoidable.

## VieNeu — no code change

`managed_vieneu` already reuses a managed process + shared HTTP client
(`tts.rs:70-80`). Neural synthesis latency is absorbed by phases 01-02.
Explicitly out of scope: server warm-up, streaming transport (deferred by the
umbrella plan `260718-2348-managed-vieneu-runtime`).

## Tests

- Windows: extend the existing ignored smoke test
  (`tts.rs:864-888`) or add a second `#[cfg(target_os = "windows")]
  #[tokio::test] #[ignore]` that synthesizes the same text twice with the
  same voice and asserts the second call does not re-enumerate voices
  (observable via the `configured_voice_id` fast path — e.g. expose a
  `#[cfg(test)] fn cached_voice_id() -> Option<String>` helper). Real
  synthesis stays behind `#[ignore]` per repo convention, but per the
  red-team note the cross-thread WinRT agility assumption is only exercised
  by actually RUNNING this test on a Windows box — phase 04's Windows
  manual follow-up must run it once (`cargo test -- --ignored`), not just
  compile it.
- Voice-switch logic is extracted into a pure helper
  `fn voice_needs_reload(configured: &str, requested: &str) -> bool` with
  cross-platform unit tests (empty/whitespace/exact-match cases).
- macOS: no changes in this phase (UUID temp files stay); existing
  `decode_wav_pcm16` tests stay green.
- Cross-platform guard: `cargo check`/`cargo test` on Windows CI is the
  compile gate for the `cfg`-gated cache (matches how the existing windows
  module is validated).

## Acceptance

- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings` pass on macOS; Windows CI green.
- Manual (Windows machine): start a local session, speak ≥5 sentences —
  synthesis per sentence visibly no longer pays activation cost (subjective
  latency drop), and switching voices in Settings takes effect on the next
  sentence without restarting the session or the app.
- Manual (macOS): unchanged behavior — temp files remain per-call UUID WAVs
  (this phase makes no macOS change).
