export type DeviceKind = "input" | "output";
export type Language = "auto" | "en" | "ja" | "vi";
export type TranslationStyle = "literal" | "natural" | "technical_meeting_safe";
export type SessionStatus =
  | "idle"
  | "starting"
  | "listening"
  | "translating"
  | "speaking"
  | "paused"
  | "stopping"
  | "error";
export type TranscriptStatus = "partial" | "final" | "error";

export interface AudioDeviceInfo {
  id: string;
  name: string;
  kind: DeviceKind;
  isDefault: boolean;
  minSampleRate?: number;
  maxSampleRate?: number;
  maxChannels?: number;
}

export interface AudioDevices {
  inputs: AudioDeviceInfo[];
  outputs: AudioDeviceInfo[];
}

export interface SessionConfig {
  sourceLanguage: Language;
  targetLanguage: Language;
  translationStyle: TranslationStyle;
  inputDeviceId: string;
  outputDeviceId: string;
  monitorOutputDeviceId: string;
  monitorOriginalAudio: boolean;
  voiceId: string;
  fallbackEnabled: boolean;
}

export interface AppStatus {
  sessionStatus: SessionStatus;
  hasApiKey: boolean;
  transcriptCount: number;
}

export interface TranscriptItem {
  id: string;
  timestampMs: number;
  sourceText: string;
  translatedText: string;
  status: TranscriptStatus;
  latencyMs?: number;
}

export interface AudioLevelEvent {
  inputDeviceId: string;
  rms: number;
  peak: number;
}

export interface AppErrorPayload {
  code: string;
  message: string;
}

export interface ExportedTranscript {
  fileName: string;
  content: string;
}
