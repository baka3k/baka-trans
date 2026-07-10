# Translation Provider Abstraction — 2026-07-10
## Context
Phase 11 of `plans/realtime-meeting-translation-macos/plan.md` called for a controlled OpenAI-to-Google migration: provider-aware translation settings, separate credentials, and language validation before adding the Gemini Live audio pipeline.

## Change
Added a `TranslationProvider` model and provider-scoped session validation in `src-tauri/src/models.rs:38` and `src-tauri/src/models.rs:281`. Split translation credential storage by provider in `src-tauri/src/security.rs:5`, with OpenAI using `OPENAI_API_KEY` and Google using `GEMINI_API_KEY`. Added provider-aware UI controls and credential status in `src/App.tsx:80` and `src/App.tsx:987`. Added provider-filtered target language options, including Google regional codes, in `src/languages.ts:11` and `src/languages.ts:142`. Split the AI client layer into a provider-neutral facade in `src-tauri/src/ai.rs:1`, an OpenAI realtime module in `src-tauri/src/ai/openai_realtime.rs:25`, and a Google Live credential probe in `src-tauri/src/ai/google_live.rs:4`.

## Impact
Impact level: medium. The active translation provider is now explicit and Google credentials can be saved/tested independently from OpenAI credentials. OpenAI remains the only runnable realtime audio backend in this phase; Google session start is blocked with a clear phase-12 message until the Live pipeline lands. The module split lowers phase-12 risk by giving the Gemini Live implementation a separate file without changing the public session/command contracts.

## Decision
Used an additive provider abstraction instead of replacing OpenAI in place. This preserves rollback and keeps the existing OpenAI realtime path stable while making Google credentials, labels, and language metadata visible for migration work.

## References
- plan: ./plans/realtime-meeting-translation-macos/phase-11-translation-provider-abstraction-google-credentials.md
- source: src-tauri/src/models.rs:38
- source: src-tauri/src/security.rs:5
- source: src-tauri/src/ai.rs:1
- source: src-tauri/src/ai/google_live.rs:4
- source: src-tauri/src/ai/openai_realtime.rs:25
- source: src/App.tsx:80
- source: src/languages.ts:11
