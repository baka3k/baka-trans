# OpenAI Realtime Language Selector — 2026-07-09
## Context
Phase 10 requested replacing the MVP language list with OpenAI Realtime Translation source and target language coverage from `plans/realtime-meeting-translation-macos/phase-10-openai-realtime-language-selector.md`.

## Change
Added frontend language metadata and derived selector options in `src/languages.ts:1`, wired `src/App.tsx:46` and `src/App.tsx:746` to use source/target option lists, and derived the frontend `Language` type from that metadata in `src/types.ts:1`. Expanded backend language deserialization and target validation in `src-tauri/src/models.rs:40`, then enforced the target-language guard before session start in `src-tauri/src/session.rs:83`.

## Impact
Impact level: medium. Users can now choose the documented Realtime Translation output languages and a broader source-language list while `auto` remains source-only. Backend validation prevents unsupported target codes from reaching the Realtime request.

## Decision
The frontend keeps one metadata source with display labels and support flags, while the backend keeps explicit enum variants for serde validation and a clear `unsupported_target_language` error. The Realtime request path stays unchanged except for receiving the expanded, validated target code.

## References
- plan: ./plans/realtime-meeting-translation-macos/phase-10-openai-realtime-language-selector.md
- commit: ba85f34ba986cc4390d45817c0a8ac27738da6b2
