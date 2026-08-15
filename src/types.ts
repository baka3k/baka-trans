import type { LanguageCode } from "./languages";

export type DeviceKind = "input" | "output";
export type AudioOutputChannel = "all" | "left" | "right";
export type ApiKeySource = "environment" | "keychain" | "memory";
export type Language = LanguageCode;
export type TranslationProvider =
  | "openai_realtime"
  | "google_live_translate"
  | "local_whisper_ollama";
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
export type TranscriptUpdateMode = "delta" | "snapshot";
export type SourceSignalState = "waiting" | "receiving" | "silent" | "stale" | "error";
export type TranslationActivityState =
  | "listening"
  | "translating"
  | "ready"
  | "needs_attention";
export type LocalPipelineStage =
  | "listening"
  | "transcribing"
  | "translating"
  | "synthesizing"
  | "speaking";
export type LlmProviderKind = "openai" | "openai_compatible" | "ollama" | "adk_litellm";
export type MeetingSummaryTrigger = "manual" | "end_of_session";
export type MeetingSummaryPromptPreset =
  | "balanced"
  | "professional"
  | "gentle"
  | "detailed"
  | "timeline"
  | "custom";
export type TranscriptScope = "source" | "translated" | "both";
export type MeetingSummaryStatus = "running" | "complete" | "error";
export type OverlayStatusKind =
  | "idle"
  | "permission_needed"
  | "scanning"
  | "translating"
  | "translated"
  | "thinking"
  | "complete"
  | "no_text"
  | "paused"
  | "error";

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

export interface OverlayConfig {
  sourceLanguage: Language;
  targetLanguage: Language;
  captureIntervalMs: number;
  minimumConfidence: number;
  opacity: number;
  geminiModel: string;
}

export interface OverlayGeometry {
  displayId?: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactor: number;
  updatedAtMs: number;
}

export interface OverlayStatus {
  isOpen: boolean;
  isPaused: boolean;
  status: OverlayStatusKind;
  message: string;
  config: OverlayConfig;
  geometry?: OverlayGeometry;
}

export interface OverlayTranslationUpdate {
  sourceText: string;
  translatedText: string;
  status: OverlayStatusKind;
  message: string;
  confidence?: number;
  latencyMs?: number;
  provider: string;
  model: string;
  updatedAtMs: number;
}

export interface LookHelpConfig {
  providerProfileId: string;
  systemPrompt: string;
  promptPanelVisible: boolean;
  captureIntervalMs: number;
  minimumConfidence: number;
  opacity: number;
  maxOcrInputChars: number;
  maxOutputTokens?: number;
}

export interface LookHelpStatus {
  isOpen: boolean;
  isPaused: boolean;
  status: OverlayStatusKind;
  message: string;
  config: LookHelpConfig;
  geometry?: OverlayGeometry;
}

export interface LookHelpUpdate {
  sourceText: string;
  answerText: string;
  status: OverlayStatusKind;
  message: string;
  latencyMs?: number;
  providerProfileId: string;
  model: string;
  promptHash: number;
  updatedAtMs: number;
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

export interface LocalTranslationConfig {
  schemaVersion: number;
  translationEngine: LocalTranslationEngine;
  openaiBaseUrl: string;
  openaiModel: string;
  openaiTimeoutSeconds: number;
  openaiTemperature: number;
  openaiMaxOutputTokens: number;
  baseUrl?: string;
  model?: string;
  timeoutSeconds?: number;
  temperature?: number;
  maxOutputTokens?: number;
  keepAlive?: string;
  modelPath: string;
  language: "ja";
  threads: number;
  useGpu: boolean;
  sampleRateHz: 16000;
  minimumSpeechMs: number;
  silenceToCommitMs: number;
  maximumUtteranceMs: number;
  preRollMs: number;
  speechThreshold: number;
  ttsProvider: LocalTtsProvider;
  vieneuBaseUrl: string;
  vieneuStyle: VieNeuReadingStyle;
  voiceId: string;
  ttsRate: number;
  ttsVolume: number;
  ttsOutputSampleRateHz: 24000;
}

export type LocalTranslationEngine = "huggingface_offline" | "openai_compatible";

export type LocalTtsProvider = "system" | "vieneu";
export type VieNeuReadingStyle = "tu_nhien" | "tin_tuc" | "doc_truyen";

export type VieNeuRuntimePhase =
  | "not_installed"
  | "paused"
  | "downloading"
  | "verifying"
  | "installed"
  | "starting"
  | "ready"
  | "recovering"
  | "repair_needed"
  | "error"
  | "unsupported";

export interface VieNeuRuntimeStatus {
  phase: VieNeuRuntimePhase;
  runtimeAvailable: boolean;
  modelInstalled: boolean;
  running: boolean;
  modelVersion: string;
  installedBytes: number;
  totalBytes: number;
  message: string;
}

export interface VieNeuRuntimeProgress {
  phase: VieNeuRuntimePhase;
  downloadedBytes: number;
  verifiedBytes: number;
  totalBytes: number;
  percent?: number;
  message: string;
}

export type LocalTranslationConfigDraft = Omit<LocalTranslationConfig, "schemaVersion">;

export interface LocalTranslationTestResult {
  ok: boolean;
  message: string;
  model: string;
  endpoint: string;
  whisperModelReadable: boolean;
  whisperModelLoaded: boolean;
  ollamaReachable: boolean;
  ollamaModelAccepted: boolean;
  ttsVoiceAvailable: boolean;
}

export interface WhisperModelOption {
  id: string;
  label: string;
  description: string;
  fileName: string;
  sizeMib: number;
  recommended: boolean;
}

export interface WhisperModelDownloadProgress {
  modelId: string;
  fileName: string;
  downloadedBytes: number;
  totalBytes?: number;
  percent?: number;
  status: "downloading" | "completed" | "error";
  message: string;
}

export interface LocalVoice {
  id: string;
  name: string;
  language: string;
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
  revision?: number;
  updateMode?: TranscriptUpdateMode;
  errorMessage?: string;
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
  sentencePairs: ConversationSentencePair[];
  status: TranscriptStatus;
  latencyMs?: number;
  errorMessage?: string;
  speakerLabel?: string;
  speakerSegmentId?: string;
  speakerConfidence?: number;
  speakerDisplayLabel: string;
  hasPendingTranslation: boolean;
}

export interface ConversationSentencePair {
  sourceText: string;
  translatedText: string;
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
  promptPreset: MeetingSummaryPromptPreset;
  customSystemPrompt: string;
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
