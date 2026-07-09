# Phase 10 - OpenAI Realtime Supported Language Selector

Status: planned
Depends on: phase 03, phase 04

## Goal

Expand the language selectors so users can choose the languages supported by OpenAI Realtime Translation instead of the current MVP-only list.

## Context

- Current UI options live in `src/App.tsx` as `languages`, with `targetLanguages` derived by removing `auto`.
- Current frontend type is `export type Language = "auto" | "en" | "ja" | "vi"` in `src/types.ts`.
- Current backend type is the Rust `Language` enum plus `realtime_code()` in `src-tauri/src/models.rs`.
- `src-tauri/src/ai.rs` already sends `config.target_language.realtime_code()` to `session.audio.output.language` when creating the Realtime Translation client secret.
- Official OpenAI docs/cookbook state that `gpt-realtime-translate` detects 70+ input languages and supports these 13 output languages: Spanish, Portuguese, French, Japanese, Russian, Chinese, German, Korean, Hindi, Indonesian, Vietnamese, Italian, and English.

## Implementation Tasks

- Add a single language metadata source with code, display label, and support flags:
  - `supportsSource`
  - `supportsTarget`
  - `isAuto`
- Use this target output set: `es`, `pt`, `fr`, `ja`, `ru`, `zh`, `de`, `ko`, `hi`, `id`, `vi`, `it`, `en`.
- Use OpenAI's documented input-language list for the source selector, plus `auto`.
- Keep `auto` out of the target selector.
- Update `src/types.ts` so `Language` can represent the expanded codes without duplicating long unions in multiple files.
- Update `src/App.tsx` selectors to use `sourceLanguageOptions` and `targetLanguageOptions` instead of deriving target options from the whole source list.
- Update `src-tauri/src/models.rs` to accept the expanded language codes and validate target support before starting a session.
- Keep the Realtime Translation request unchanged except for passing the selected target code.
- Preserve the same-language warning for explicit source selections, but skip it when source is `auto`.
- Add or update tests for:
  - target options include exactly the 13 supported output languages
  - target options exclude `auto`
  - source options include `auto`
  - backend rejects unsupported target codes with a clear error

## Input Language Labels

Use the official cookbook source-language list as labels for the source selector: Arabic, Afrikaans, Azerbaijani, Belarusian, Bengali, Bosnian, Bulgarian, Catalan, Chinese, Croatian, Czech, Danish, Dutch, Dzongkha, English, Esperanto, Estonian, Basque, Persian / Farsi, Finnish, Filipino, French, Galician, German, Greek, Gujarati, Haitian Creole, Hawaiian, Hebrew, Hindi, Hungarian, Armenian, Indonesian, Italian, Japanese, Javanese, Georgian, Kazakh, Korean, Kurdish, Latin, Latvian, Lithuanian, Macedonian, Malay, Malayalam, Maori, Mongolian, Burmese / Myanmar, Nepali, Norwegian, Nynorsk, Polish, Portuguese, Punjabi, Romanian, Russian, Serbian, Shona, Slovak, Slovenian, Albanian, Spanish, Swahili, Swedish, Tagalog, Telugu, Thai, Turkish, Ukrainian, Uzbek, Vietnamese, Welsh, and Yoruba.

## Verification

- Run the frontend/unit test suite after updating language metadata.
- Run Rust tests or `cargo check` for `src-tauri`.
- Manually confirm the UI shows `Auto` only in Source and all 13 output languages in Target.
- Start a session with one newly added target language code far from the old MVP list, such as `es` or `de`, and confirm the backend sends that code to the Realtime Translation session.

## Exit Criteria

- Users can select every OpenAI-supported Realtime Translation target language.
- Users can label/select source languages from the OpenAI-documented input set while keeping automatic detection available.
- Unsupported target codes cannot reach the OpenAI Realtime Translation request.
