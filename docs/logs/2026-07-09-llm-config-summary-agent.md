# LLM Config Summary Agent — 2026-07-09
## Context
Implemented Phase 09 from `plans/realtime-meeting-translation-macos/phase-09-llm-config-summary-agent.md`, which asked for LLM provider profiles and a manual meeting-summary agent separate from the OpenAI Realtime translation key path.
## Change
Added Rust DTOs for LLM provider profiles, summary configuration, status events, action items, and structured meeting notes (`src-tauri/src/models.rs:207`). Added an OpenAI-compatible LLM client and profile store that keeps metadata in app config while storing per-profile secrets under namespaced Keychain users (`src-tauri/src/llm.rs:12`, `src-tauri/src/llm.rs:86`, `src-tauri/src/llm.rs:227`). Added `MeetingSummaryAgent` to select transcript text, chunk long meetings, preserve source transcript item IDs, request JSON output, validate/merge structured notes, and emit progress/results (`src-tauri/src/summary_agent.rs:36`, `src-tauri/src/summary_agent.rs:157`).

Registered Tauri commands for profile CRUD/testing and `run_meeting_summary_agent`, while keeping old API-key commands as wrappers and adding explicit translation-key command names (`src-tauri/src/commands.rs:57`, `src-tauri/src/commands.rs:99`). Updated the React UI to separate Translation OpenAI key setup from Summary Agent provider configuration, profile testing, summary options, and meeting-note display/export (`src/App.tsx:940`, `src/App.tsx:1005`, `src/App.tsx:1210`). Added frontend helpers and tests for profile validation, default summary config, and exporting meeting notes with transcripts (`src/transcript.ts:62`, `src/transcript.test.ts:39`).
## Impact
Impact level: medium. Users can configure OpenAI-compatible, OpenAI, Ollama, or ADK/LiteLLM-shaped summary profiles without changing the realtime translation key. Manual meeting notes now run over transcript state and produce structured summary, decisions, action items, blockers, and important points. Main residual risks are provider-specific response quirks and the lack of a live manual OpenAI/Ollama run in this session.
## Decision
Kept the first runtime in Rust through OpenAI-compatible chat completions for packaging simplicity, while representing ADK/LiteLLM in the profile schema for a future sidecar adapter. Summary uses explicit profile secrets instead of reusing the translation OpenAI key by default. Because realtime transcript events currently store partial deltas, the agent prefers final items when present and falls back to non-error transcript text so notes are usable before finalization semantics are expanded.
## References
- plan: `./plans/realtime-meeting-translation-macos/phase-09-llm-config-summary-agent.md`
- verification: `npm test`, `npm run build`, `cargo fmt --check`, `cargo check`, `cargo test`
- commit: `389e081`
