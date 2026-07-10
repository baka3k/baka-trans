import type { LanguageCode } from "./languages";

export type DeviceKind = "input" | "output";
export type AudioOutputChannel = "all" | "left" | "right";
export type ApiKeySource = "environment" | "keychain" | "memory";
export type Language = LanguageCode;
export type TranslationProvider = "openai_realtime" | "google_live_translate";
export type TranslationStyle = "literal" | "natural" | "technical_meeting_safe";
export type ManualBoundaryReason = "user_button" | "keyboard_shortcut";
export type ManualBoundaryStatus =
  | "idle"
  | "pending"
  | "committed"
  | "ignored_empty_buffer"
  | "rate_limited"
  | "error";
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
export type SourceSignalState = "waiting" | "receiving" | "silent" | "stale" | "error";
export type TranslationActivityState =
  | "listening"
  | "translating"
  | "ready"
  | "needs_attention";
export type LlmProviderKind = "openai" | "openai_compatible" | "ollama" | "adk_litellm";
export type MeetingSummaryTrigger = "manual" | "end_of_session";
export type TranscriptScope = "source" | "translated" | "both";
export type MeetingSummaryStatus = "running" | "complete" | "error";

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
  translationProvider: TranslationProvider;
  sourceLanguage: Language;
  targetLanguage: Language;
  translationStyle: TranslationStyle;
  inputDeviceId: string;
  outputDeviceId: string;
  translationOutputChannel: AudioOutputChannel;
  monitorOutputDeviceId: string;
  monitorOutputChannel: AudioOutputChannel;
  monitorOriginalAudio: boolean;
  voiceId: string;
  fallbackEnabled: boolean;
}

export interface AppStatus {
  sessionStatus: SessionStatus;
  hasApiKey: boolean;
  apiKeySource?: ApiKeySource;
  apiKeyFingerprint?: string;
  transcriptCount: number;
}

export interface ApiKeyTestResult {
  provider: TranslationProvider;
  source: ApiKeySource;
  fingerprint: string;
  message: string;
}

export interface TranslationCredentialStatus {
  provider: TranslationProvider;
  hasApiKey: boolean;
  apiKeySource?: ApiKeySource;
  apiKeyFingerprint?: string;
}

export interface LlmProviderProfile {
  id: string;
  name: string;
  kind: LlmProviderKind;
  model: string;
  baseUrl?: string;
  hasApiKey: boolean;
  apiKeySource?: string;
  apiKeyFingerprint?: string;
  timeoutSeconds: number;
  maxOutputTokens: number;
  temperature: number;
  enabled: boolean;
}

export interface LlmProviderProfileDraft {
  id?: string;
  name: string;
  kind: LlmProviderKind;
  model: string;
  baseUrl?: string;
  apiKey?: string;
  timeoutSeconds?: number;
  maxOutputTokens?: number;
  temperature?: number;
  enabled?: boolean;
}

export interface LlmProviderTestResult {
  profileId: string;
  ok: boolean;
  message: string;
  model: string;
  baseUrl: string;
}

export interface ManualBoundaryRequest {
  reason: ManualBoundaryReason;
  requestedAtMs: number;
}

export interface ManualBoundaryEvent {
  status: ManualBoundaryStatus;
  message: string;
  committedAtMs?: number;
}

export interface TranscriptItem {
  id: string;
  timestampMs: number;
  sourceText: string;
  translatedText: string;
  status: TranscriptStatus;
  latencyMs?: number;
  speakerLabel?: string;
  speakerSegmentId?: string;
  speakerConfidence?: number;
}

export interface AudioLevelEvent {
  inputDeviceId: string;
  rms: number;
  peak: number;
}

export interface SourceSignalSnapshot extends AudioLevelEvent {
  receivedAtMs: number;
}

export interface ConversationDisplayItem {
  id: string;
  timestampMs: number;
  sourceText: string;
  translatedText: string;
  status: TranscriptStatus;
  latencyMs?: number;
  speakerLabel?: string;
  speakerSegmentId?: string;
  speakerConfidence?: number;
  speakerDisplayLabel: string;
  hasPendingTranslation: boolean;
}

export interface TranslatedAudioLevelEvent {
  sampleCount: number;
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

export interface MeetingSummarySections {
  summary: boolean;
  decisions: boolean;
  actionItems: boolean;
  blockers: boolean;
  importantPoints: boolean;
}

export interface MeetingSummaryConfig {
  providerProfileId: string;
  trigger: MeetingSummaryTrigger;
  transcriptScope: TranscriptScope;
  outputLanguage: string;
  sections: MeetingSummarySections;
  maxTranscriptChars: number;
  rollingMemoryEnabled: boolean;
}

export interface ActionItem {
  text: string;
  owner?: string;
  dueDate?: string;
  sourceItemIds: string[];
}

export interface MeetingSummaryResult {
  id: string;
  createdAtMs: number;
  sourceItemIds: string[];
  summary: string;
  decisions: string[];
  actionItems: ActionItem[];
  blockers: string[];
  importantPoints: string[];
  model: string;
  providerProfileId: string;
  status: MeetingSummaryStatus;
  errorMessage?: string;
}

export interface MeetingSummaryStatusEvent {
  status: MeetingSummaryStatus;
  message: string;
}
