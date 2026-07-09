# Phase 09 - LLM Configuration and Meeting-Summary Agent

Status: planned
Depends on: phase 03, phase 04, phase 05

## Goal

Add a clear LLM configuration experience and a meeting-summary agent that can summarize the current transcript, point out important things to remember, and extract decisions/action items without sending raw meeting data to an LLM through an unstructured one-shot prompt.

The summary capability must be independent from the realtime translation pipeline. Translation keeps its OpenAI Realtime credentials and model assumptions. Summary uses selectable LLM provider profiles that are compatible with OpenAI-style chat APIs and can also support local Ollama through Google ADK/LiteLLM-friendly configuration.

## Context

- `src-tauri/src/ai.rs` is currently focused on OpenAI Realtime Translation and mints Realtime client secrets from one API key.
- `src-tauri/src/security.rs` stores a single OpenAI API key under the Keychain user `openai-api-key`.
- `src/api.ts` exposes `saveApiKey`, `hasApiKey`, and `testApiKey`, but no generic provider profile commands.
- `src/types.ts` and `src-tauri/src/models.rs` mirror all bridge DTOs and must stay aligned.
- `src/App.tsx` holds the current compact settings UI, including the key panel, session controls, routing, and transcript display.
- Google ADK's Ollama guidance recommends `LiteLlm(model="ollama_chat/<model>")` for Ollama-hosted models. It also documents an OpenAI-provider option using `OPENAI_API_BASE=http://localhost:11434/v1` and a dummy `OPENAI_API_KEY`, so the app config should support both concepts.

## Scope Challenge

Question 1: Should summary reuse the translation OpenAI key automatically?
Decision: No. Keep translation and summary configuration visually and logically separate. Offer an explicit "use translation OpenAI key" choice for an OpenAI summary profile later, but default to separate profile setup so Ollama/local users are not forced through the realtime translation key path.

Question 2: Should the first implementation embed Google ADK directly?
Decision: Build the app-side agent contract first and use a Rust OpenAI-compatible client as the default executor for packaging simplicity. Design the config and runtime boundary so a Python ADK sidecar or future ADK adapter can execute the same `MeetingSummaryAgent` contract without changing the UI or stored profile schema.

Question 3: Should the agent run continuously during live translation?
Decision: Start with manual and end-of-session triggers. Optional interval summaries can be added after the first agent path is stable, because continuous summarization increases cost, privacy exposure, and state-race complexity.

## Architecture

```text
React settings UI
  -> Tauri commands:
       list_llm_profiles
       save_llm_profile
       delete_llm_profile
       test_llm_profile
       run_meeting_summary_agent
  -> Rust config/security:
       profile metadata in app config
       profile secrets in Keychain
  -> MeetingSummaryAgent:
       collect finalized transcript items
       chunk/compact transcript
       call model executor
       validate/repair structured output
       emit summary-agent-status and meeting-summary-update
  -> React summary panel/export
```

Agent shape:

```text
MeetingSummaryAgent
  Inputs:
    transcript items
    transcript scope
    output language
    enabled sections
    provider profile
  Internal steps:
    1. select high-quality transcript text
    2. chunk long transcript with item IDs
    3. summarize chunks into rolling memory
    4. extract decisions, action items, blockers, and facts to remember
    5. merge and validate final structured notes
  Outputs:
    MeetingSummaryResult
    source transcript item IDs
    provider/model metadata
```

Default executor:

- Add `src-tauri/src/llm.rs` for OpenAI-compatible chat-completion calls.
- Support `https://api.openai.com/v1`, arbitrary OpenAI-compatible `base_url`, and Ollama's `http://localhost:11434/v1`.
- Normalize base URLs so the request goes to `/chat/completions`.
- Use `Authorization: Bearer <api_key>` when a key is configured; allow a placeholder key for local Ollama profiles.
- Use deterministic temperature by default and request JSON output when the provider supports it, but still parse/repair non-JSON responses.

Optional ADK adapter:

- Keep a feature flag or executor kind for `adk_litellm`.
- Represent ADK model IDs explicitly, for example `ollama_chat/gemma3:latest` or `openai/mistral-small3.1`.
- If implemented, run the ADK agent as a local sidecar process or local HTTP service that receives the same `MeetingSummaryAgent` input and returns the same result schema.
- Do not make live translation depend on the ADK sidecar.

## Data Model

Rust/TypeScript bridge DTOs:

- `LlmProviderKind`: `openai`, `openai_compatible`, `ollama`, `adk_litellm`
- `LlmProviderProfile`
  - `id`
  - `name`
  - `kind`
  - `model`
  - `baseUrl`
  - `hasApiKey`
  - `apiKeySource`
  - `apiKeyFingerprint`
  - `timeoutSeconds`
  - `maxOutputTokens`
  - `temperature`
  - `enabled`
- `LlmProviderProfileDraft`
  - profile fields plus optional `apiKey`
- `LlmProviderTestResult`
  - `profileId`
  - `ok`
  - `message`
  - `model`
  - `baseUrl`
- `MeetingSummaryConfig`
  - `providerProfileId`
  - `trigger`
  - `transcriptScope`
  - `outputLanguage`
  - `sections`
  - `maxTranscriptChars`
  - `rollingMemoryEnabled`
- `MeetingSummaryResult`
  - `id`
  - `createdAtMs`
  - `sourceItemIds`
  - `summary`
  - `decisions`
  - `actionItems`
  - `blockers`
  - `importantPoints`
  - `model`
  - `providerProfileId`
  - `status`
  - `errorMessage`
- `ActionItem`
  - `text`
  - `owner`
  - `dueDate`
  - `sourceItemIds`

Storage:

- Persist non-secret profiles in an app config file or backend-managed JSON state, not in `localStorage`.
- Store each profile secret in Keychain with a namespaced user such as `llm-profile-{profile_id}`.
- Keep the existing translation key path for OpenAI Realtime; rename UI labels and command wrappers so it is clear this key is for translation.

## UI Plan

Redesign the current settings area into compact sections:

1. Translation
   - OpenAI Realtime key status.
   - Save/replace/test translation key.
   - Existing source/target/fallback controls remain tied to translation.

2. Summary Agent
   - Provider selector.
   - Add/edit profile dialog or inline drawer.
   - Provider kind segmented control: OpenAI, OpenAI-compatible, Ollama, ADK/LiteLLM.
   - Model and base URL inputs.
   - API key field with placeholder-key hint for local Ollama only.
   - Test profile button.
   - Summary options: transcript scope, output language, enabled sections.

3. Meeting Notes
   - Run summary button.
   - Status/progress line.
   - Summary, decisions, action items, blockers, important points.
   - Export should include meeting notes when present.

Keep the UI dense and operational. Avoid long help text. Use concise labels, icons, tooltips for unfamiliar provider choices, and stable panel dimensions so the settings column does not jump while tests run.

## Implementation Tasks

1. Split translation key naming and commands.
   - Keep existing storage compatible, but rename frontend labels to "Translation OpenAI key".
   - Consider adding explicit command names such as `save_translation_api_key` while preserving old command wrappers if needed.
   - Update `AppStatus` or add a config status DTO if summary provider status would bloat session status.

2. Add provider profile DTOs and persistence.
   - Update `src/types.ts` and `src-tauri/src/models.rs` together.
   - Add profile CRUD commands in `src-tauri/src/commands.rs` and register them in `src-tauri/src/lib.rs`.
   - Add backend config read/write helpers for profile metadata.
   - Store profile API keys in Keychain under per-profile secret names.

3. Add OpenAI-compatible LLM client.
   - Create `src-tauri/src/llm.rs`.
   - Build chat-completion requests from provider profiles.
   - Normalize OpenAI/Ollama base URLs.
   - Support timeout, model, temperature, and max tokens.
   - Parse JSON responses and return useful provider errors.
   - Add a lightweight test call for `test_llm_profile`.

4. Implement `MeetingSummaryAgent`.
   - Create `src-tauri/src/summary_agent.rs`.
   - Pull finalized transcript items from `AppState`.
   - Select source/translated/both transcript text based on config.
   - Chunk long transcripts and retain source item IDs.
   - Run structured extraction steps instead of one untracked prompt.
   - Validate result sections and emit app errors/status events on failure.

5. Wire frontend config and notes UI.
   - Refactor the current key panel in `src/App.tsx` into Translation and Summary Agent sections.
   - Add profile add/edit/test flows.
   - Add meeting notes state and event listeners.
   - Add a Run Summary action that is disabled when no transcript or no valid provider is selected.
   - Include meeting notes in Markdown export when present.

6. Add tests.
   - TypeScript tests for profile validation UI helpers and transcript-to-agent input helpers.
   - Rust tests for base URL normalization, request payload construction, JSON response parsing, and summary result validation.
   - Manual test with one OpenAI profile and one local Ollama-compatible profile if Ollama is installed.

## Verification

- `npm test`
- `npm run build`
- `cargo fmt --check`
- `cargo check`
- Unit tests:
  - translation key status still works after UI rename
  - profile validation rejects missing model/base URL when required
  - Ollama profile defaults to `http://localhost:11434/v1`
  - OpenAI-compatible client targets `/chat/completions`
  - malformed LLM output returns a structured summary-agent error
  - summary agent preserves source transcript item IDs
- Manual tests:
  - Add/test an OpenAI summary profile without changing the realtime translation key.
  - Add/test an Ollama profile with a local model and placeholder key.
  - Run summary on a transcript and verify summary, decisions, action items, blockers, and important points appear.
  - Start/stop translation while a summary is idle and verify session controls still work.

## Exit Criteria

- Translation configuration and summary-agent configuration are visually separate and understandable.
- The user can add at least one OpenAI-compatible LLM profile and test it.
- The profile schema can represent Ollama through OpenAI-compatible base URL mode and ADK/LiteLLM model naming mode.
- The meeting-summary agent runs over transcript state and returns structured notes, not just raw model text.
- Summary runs do not interfere with live audio capture, realtime translation, playback, manual boundary, or transcript updates.
- Secrets remain backend-controlled and are not persisted in frontend storage.

## Non-Goals

- Full searchable meeting archive.
- Background summarization every few seconds.
- Speaker diarization or owner inference beyond what the transcript text explicitly states.
- Replacing OpenAI Realtime Translation with an LLM text translation loop.
- Requiring Google ADK as the only summary runtime.
