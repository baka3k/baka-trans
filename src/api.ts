import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatus,
  ApiKeyTestResult,
  AudioOutputChannel,
  AudioDevices,
  ExportedTranscript,
  LlmProviderProfile,
  LlmProviderProfileDraft,
  LlmProviderTestResult,
  ManualBoundaryReason,
  MeetingSummaryConfig,
  MeetingSummaryResult,
  OverlayConfig,
  OverlayGeometry,
  OverlayStatus,
  SessionConfig,
  TranslationCredentialStatus,
  TranslationProvider,
} from "./types";

export function getAppStatus() {
  return invoke<AppStatus>("get_app_status");
}

export function listAudioDevices() {
  return invoke<AudioDevices>("list_audio_devices");
}

export function startSession(config: SessionConfig) {
  return invoke<void>("start_session", { config });
}

export function pauseSession() {
  return invoke<void>("pause_session");
}

export function resumeSession() {
  return invoke<void>("resume_session");
}

export function stopSession() {
  return invoke<void>("stop_session");
}

export function forceTranslateBoundary(reason: ManualBoundaryReason) {
  return invoke<void>("force_translate_boundary", {
    request: {
      reason,
      requestedAtMs: Date.now(),
    },
  });
}

export function saveApiKey(apiKey: string) {
  return invoke<void>("save_api_key", { apiKey });
}

export function saveTranslationApiKey(provider: TranslationProvider, apiKey: string) {
  return invoke<void>("save_translation_api_key", { provider, apiKey });
}

export function hasApiKey() {
  return invoke<boolean>("has_api_key");
}

export function hasTranslationApiKey(provider: TranslationProvider) {
  return invoke<boolean>("has_translation_api_key", { provider });
}

export function testApiKey() {
  return invoke<ApiKeyTestResult>("test_api_key");
}

export function translationCredentialStatus(provider: TranslationProvider) {
  return invoke<TranslationCredentialStatus>("translation_credential_status", { provider });
}

export function testTranslationApiKey(provider: TranslationProvider) {
  return invoke<ApiKeyTestResult>("test_translation_api_key", { provider });
}

export function listLlmProfiles() {
  return invoke<LlmProviderProfile[]>("list_llm_profiles");
}

export function saveLlmProfile(draft: LlmProviderProfileDraft) {
  return invoke<LlmProviderProfile>("save_llm_profile", { draft });
}

export function deleteLlmProfile(profileId: string) {
  return invoke<void>("delete_llm_profile", { profileId });
}

export function testLlmProfile(profileId: string) {
  return invoke<LlmProviderTestResult>("test_llm_profile", { profileId });
}

export function runMeetingSummaryAgent(config: MeetingSummaryConfig) {
  return invoke<MeetingSummaryResult>("run_meeting_summary_agent", { config });
}

export function exportTranscript(format: "text" | "markdown") {
  return invoke<ExportedTranscript>("export_transcript", {
    request: { format },
  });
}

export function playTestTone(outputDeviceId: string, outputChannel: AudioOutputChannel) {
  return invoke<void>("play_test_tone", { outputDeviceId, outputChannel });
}

export function stopTestTone() {
  return invoke<void>("stop_test_tone");
}

export function startLocalMonitor(
  inputDeviceId: string,
  outputDeviceId: string,
  outputChannel: AudioOutputChannel,
) {
  return invoke<void>("start_local_monitor", {
    inputDeviceId,
    outputDeviceId,
    outputChannel,
  });
}

export function stopLocalMonitor() {
  return invoke<void>("stop_local_monitor");
}

export function openOverlayWindow(config: OverlayConfig) {
  return invoke<void>("open_overlay_window", { config });
}

export function closeOverlayWindow() {
  return invoke<void>("close_overlay_window");
}

export function overlayStatus() {
  return invoke<OverlayStatus>("overlay_status");
}

export function updateOverlayGeometry(geometry: OverlayGeometry) {
  return invoke<void>("update_overlay_geometry", { geometry });
}

export function setOverlayPaused(paused: boolean) {
  return invoke<void>("set_overlay_paused", { paused });
}

export function openScreenRecordingSettings() {
  return invoke<void>("open_screen_recording_settings");
}
