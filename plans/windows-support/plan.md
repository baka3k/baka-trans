# Windows Support

Status: implemented; hardware/manual release validation pending
Created: 2026-07-12
Source spec: user request in Codex thread, Vietnamese: determine whether the macOS-only project can be changed to run on Windows.
Mode: `hi-plan --fast`
Blocked by: `plans/realtime-meeting-translation-macos`, `plans/transparent-ocr-overlay-macos`, `plans/look-help-overlay-macos` (the required foundations already exist; remaining work is platform parity)
Blocks: none

## Objective

Make Baka Trans build, install, and run as a supported Windows desktop application without regressing macOS. Preserve the shared Tauri/React UI, realtime translation pipelines, transcript/summary behavior, and overlay UX while isolating OS-specific credential, audio-routing, capture/OCR, packaging, and documentation behavior.

## Feasibility Decision

Windows support is feasible and does not require rewriting the application. The shared stack is already portable: React/Vite, Tauri 2, Rust networking, and `cpal` audio all support Windows. The work is a platform port around the current macOS-specific seams, not a new product implementation.

The recommended first milestone is the realtime meeting translator on Windows. Transparent OCR and Look & Help should follow once the normal app, credentials, and Windows audio routing are proven. This limits the first validation surface and avoids mixing WASAPI/virtual-audio issues with Windows screen capture and OCR issues.

## Evidence Bundle

- `README.md:58-78` already states that Windows builds must use a Windows machine/CI and suggests `nsis,msi`, but the repository is not configured to produce those bundles yet.
- `src-tauri/tauri.conf.json:12-31` enables the macOS private API and limits bundle targets to `dmg` and `app`; it also embeds an Apple signing identity.
- `src-tauri/Cargo.toml:20-33` enables Tauri's `macos-private-api` globally and configures `keyring` with only `apple-native`. The Objective-C/Vision dependencies are correctly target-gated to macOS.
- `src-tauri/src/audio.rs:69-122` uses `cpal::default_host()` and generic input/output streams. This is a strong portable base, but Windows device enumeration, sample formats, exclusive-device conflicts, and virtual-cable routing still need runtime validation.
- `src-tauri/src/security.rs:21-99` and `src-tauri/src/llm.rs:466-502` use the `keyring` abstraction, so Windows Credential Manager support can be introduced at the dependency-feature layer without changing the frontend secret flow.
- `src-tauri/src/overlay.rs:442-568` implements capture/OCR and permission handling only under `cfg(target_os = "macos")`; non-macOS builds intentionally return an unsupported-platform error.
- `src-tauri/src/look_help.rs:528-541` already target-gates the native `NSWindow` behavior and supplies a non-macOS fallback, but parity must be checked for overlay exclusion/click-through behavior.
- `src/App.tsx:1174-1392` contains user-visible macOS and BlackHole-specific routing language that must become platform-aware.
- `plans/transparent-ocr-overlay-macos/phase-04-windows-capture-ocr-backend.md` already identifies Windows Graphics Capture plus Windows OCR as the intended overlay backend. This plan absorbs that deferred phase into a full Windows product port.
- Local verification on 2026-07-12: frontend tests could not start because dependencies are not installed (`vitest` not found); `cargo check --manifest-path src-tauri/Cargo.toml` did not complete within 120 seconds. These are environment/inconclusive results, not proof of Windows compatibility.

## Architecture Decision

Keep one cross-platform application and isolate native behavior behind target-specific Rust modules/configuration:

- Shared: React UI, Tauri commands/events, session state, Gemini/OpenAI WebSockets, summaries, transcripts, dedupe/cache logic.
- macOS: Keychain, BlackHole guidance, CoreGraphics/Vision capture/OCR, Apple signing/notarization, DMG/app bundles.
- Windows: Credential Manager, built-in WASAPI loopback capture, native GDI desktop-region capture plus Windows OCR, Authenticode signing hooks, and NSIS bundles.

Do not fork the frontend or maintain a separate Windows repository. Prefer `cfg(target_os)` Rust modules and Tauri platform override config files so shared behavior remains testable once.

## Phases

1. [Cross-platform build and credential foundation](phase-01-build-credentials.md)
2. [Windows audio routing and realtime parity](phase-02-audio-realtime.md)
3. [Windows overlays, packaging, and release validation](phase-03-overlays-packaging.md)

## Acceptance Criteria

- A clean Windows development machine can run `npm ci`, `npm run tauri -- dev`, tests, and a release build.
- Google/OpenAI and LLM-profile secrets persist through Windows Credential Manager and never move into frontend storage or logs.
- Windows users can select working input/output devices, receive source/translated audio, and follow an accurate Teams routing guide.
- Look Through and Look & Help capture text outside their own overlay, handle DPI/multiple monitors, and report actionable Windows permission/consent errors.
- CI builds Windows artifacts; NSIS is the required installer and MSI is optional unless distribution policy requires it.
- Existing macOS build, Keychain, capture/OCR, and release behavior still pass their checks.

## Main Risks

- `cpal` device support does not automatically expose Windows system-output loopback capture, so the Rust backend needs a small Windows-specific WASAPI implementation. This is more implementation work than requiring a virtual cable, but it gives users the simplest setup: no extra driver, no virtual mixer, and no Teams speaker rerouting. VB-CABLE remains an optional fallback for unsupported/problematic devices; VoiceMeeter is out of the default scope because its mixer setup is unnecessarily complex for this product.
- Windows desktop capture and overlay exclusion differ from macOS Screen Recording/TCC. The implementation uses direct GDI region capture plus `WDA_EXCLUDEFROMCAPTURE`, avoiding a capture picker; DPI transforms and mixed-scale monitors still require real hardware tests. Windows Graphics Capture remains the upgrade path if GDI fails on protected or accelerated surfaces.

## Implementation Result (2026-07-12)

- Windows build and Credential Manager dependencies compile successfully.
- Windows outputs are exposed as `Teams audio (system output)` loopback sources through CPAL's WASAPI backend.
- Windows overlay capture uses DPI-scaled GDI region capture, excludes both overlay windows from capture, and recognizes text with `Windows.Media.Ocr`.
- The frontend uses the simple three-step Windows routing workflow and hides the redundant original-audio monitor controls.
- Frontend build/tests, Rust tests, release build, process launch smoke test, and NSIS packaging pass on Windows.
- Remaining release gates require real Teams audio, credential save/load, Windows OCR content, mixed-DPI monitors, install/upgrade/uninstall, and a macOS CI run.
- Windows OCR API availability varies by OS/runtime. A Windows OCR implementation needs a clearly defined minimum Windows version and an actionable unsupported-runtime error.
- MSI builds require the WiX toolchain; NSIS is simpler as the baseline installer.

## Suggested Cook Command

`hi-cook plans/windows-support/plan.md`
