import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatus,
  AudioDevices,
  ExportedTranscript,
  SessionConfig,
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

export function saveApiKey(apiKey: string) {
  return invoke<void>("save_api_key", { apiKey });
}

export function hasApiKey() {
  return invoke<boolean>("has_api_key");
}

export function exportTranscript(format: "text" | "markdown") {
  return invoke<ExportedTranscript>("export_transcript", {
    request: { format },
  });
}

export function playTestTone(outputDeviceId: string) {
  return invoke<void>("play_test_tone", { outputDeviceId });
}

