import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatus,
  ApiKeyTestResult,
  AudioOutputChannel,
  AudioDevices,
  ExportedTranscript,
  HyMtModelStatus,
  LookHelpConfig,
  LookHelpStatus,
  LocalTranslationConfig,
  LocalTranslationConfigDraft,
  LocalTranslationTestResult,
  LocalTtsProvider,
  LocalVoice,
  VieNeuRuntimeStatus,
  WhisperModelOption,
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
  TranslationEngineTestResult,
  TranslationProvider,
  TranscriptItem,
} from "./types";

export function getAppStatus() {
  return invoke<AppStatus>("get_app_status");
}

export function getTranscriptSnapshot() {
  return invoke<TranscriptItem[]>("get_transcript_snapshot");
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

export function getLocalTranslationConfig() {
  return invoke<LocalTranslationConfig>("get_local_translation_config");
}

export function saveLocalTranslationConfig(draft: LocalTranslationConfigDraft) {
  return invoke<LocalTranslationConfig>("save_local_translation_config", { draft });
}

export function testLocalTranslationConfig(draft: LocalTranslationConfigDraft) {
  return invoke<LocalTranslationTestResult>("test_local_translation_config", { draft });
}

export function testTranslationEngine(draft: LocalTranslationConfigDraft) {
  return invoke<TranslationEngineTestResult>("test_translation_engine", { draft });
}

export function listWhisperModels() {
  return invoke<WhisperModelOption[]>("list_whisper_models");
}

export function downloadWhisperModel(modelId: string) {
  return invoke<string>("download_whisper_model", { modelId });
}

export function getWhisperModelDir() {
  return invoke<string>("get_whisper_model_dir");
}

export function listLocalTtsVoices(provider: LocalTtsProvider) {
  return invoke<LocalVoice[]>("list_local_tts_voices", { provider });
}

export function getVieNeuRuntimeStatus() {
  return invoke<VieNeuRuntimeStatus>("get_vieneu_runtime_status");
}

export function installVieNeuRuntime() {
  return invoke<VieNeuRuntimeStatus>("install_vieneu_runtime");
}

export function cancelVieNeuRuntimeInstall() {
  return invoke<void>("cancel_vieneu_runtime_install");
}

export function restartVieNeuRuntime() {
  return invoke<VieNeuRuntimeStatus>("restart_vieneu_runtime");
}

export function getHyMtModelStatus() {
  return invoke<HyMtModelStatus>("get_hy_mt_model_status");
}

export function installHyMtModel() {
  return invoke<HyMtModelStatus>("install_hy_mt_model");
}

export function cancelHyMtModelInstall() {
  return invoke<void>("cancel_hy_mt_model_install");
}

export function previewLocalTts(
  draft: LocalTranslationConfigDraft,
  outputDeviceId: string,
  outputChannel: AudioOutputChannel,
) {
  return invoke<void>("preview_local_tts", { draft, outputDeviceId, outputChannel });
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

export function openLookHelpWindow(config: LookHelpConfig) {
  return invoke<void>("open_look_help_window", { config });
}

export function closeOverlayWindow() {
  return invoke<void>("close_overlay_window");
}

export function closeLookHelpWindow() {
  return invoke<void>("close_look_help_window");
}

export function overlayStatus() {
  return invoke<OverlayStatus>("overlay_status");
}

export function lookHelpStatus() {
  return invoke<LookHelpStatus>("look_help_status");
}

export function captureLookHelp() {
  return invoke<void>("capture_look_help");
}

export function updateOverlayGeometry(geometry: OverlayGeometry) {
  return invoke<void>("update_overlay_geometry", { geometry });
}

export function updateOverlayConfig(config: OverlayConfig) {
  return invoke<void>("update_overlay_config", { config });
}

export function updateLookHelpGeometry(geometry: OverlayGeometry) {
  return invoke<void>("update_look_help_geometry", { geometry });
}

export function updateLookHelpConfig(config: LookHelpConfig) {
  return invoke<void>("update_look_help_config", { config });
}

export function setOverlayPaused(paused: boolean) {
  return invoke<void>("set_overlay_paused", { paused });
}

export function setLookHelpPaused(paused: boolean) {
  return invoke<void>("set_look_help_paused", { paused });
}

export function openScreenRecordingSettings() {
  return invoke<void>("open_screen_recording_settings");
}
