import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { currentMonitor, getCurrentWindow } from "@tauri-apps/api/window";
import {
  ArrowClockwiseRegular as RefreshCwIcon,
  ArrowLeftRegular as ArrowLeftIcon,
  ArrowDownloadRegular as DownloadIcon,
  BotRegular as BotIcon,
  CheckmarkRegular as CheckIcon,
  CopyRegular as CopyIcon,
  DeleteRegular as Trash2Icon,
  DismissRegular as XIcon,
  DocumentTextRegular as FileTextIcon,
  DragRegular as MoveIcon,
  HeadphonesRegular as HeadphonesIcon,
  KeyRegular as KeyRoundIcon,
  MicRegular as MicIcon,
  PanelLeftContractRegular as PanelLeftCloseIcon,
  PanelLeftExpandRegular as PanelLeftOpenIcon,
  PauseRegular as PauseIcon,
  PlayRegular as PlayIcon,
  PulseRegular as ActivityIcon,
  SaveRegular as SaveIcon,
  ScanTextRegular as ScanTextIcon,
  SendRegular as SendIcon,
  SettingsRegular as Settings2Icon,
  Speaker2Regular as Volume2Icon,
  StopRegular as SquareIcon,
  WarningRegular as AlertTriangleIcon,
  type FluentIcon,
} from "@fluentui/react-icons";
import { startTransition, useEffect, useMemo, useRef, useState } from "react";
import { AppNavigation, type SettingsSection } from "../components/shell/AppNavigation";
import { ResponsiveSettingsPanel } from "../components/shell/ResponsiveSettingsPanel";
import { SessionCommandBar } from "../components/session/SessionCommandBar";
import {
  LocalLlmSettings,
  defaultLocalTranslationConfig,
  validateLocalTranslationDraft,
} from "../components/settings/LocalLlmSettings";
import {
  captureLookHelp,
  exportTranscript,
  forceTranslateBoundary,
  getVieNeuRuntimeStatus,
  getLocalTranslationConfig,
  getAppStatus,
  getTranscriptSnapshot,
  lookHelpStatus,
  closeOverlayWindow,
  closeLookHelpWindow,
  deleteLlmProfile,
  downloadWhisperModel,
  listAudioDevices,
  listLocalTtsVoices,
  listWhisperModels,
  listLlmProfiles,
  openLookHelpWindow,
  openOverlayWindow,
  openScreenRecordingSettings,
  overlayStatus,
  pauseSession,
  playTestTone,
  previewLocalTts,
  installVieNeuRuntime,
  cancelVieNeuRuntimeInstall,
  restartVieNeuRuntime,
  resumeSession,
  runMeetingSummaryAgent,
  saveLlmProfile,
  saveLocalTranslationConfig,
  saveTranslationApiKey,
  startLocalMonitor,
  startSession,
  stopTestTone,
  stopLocalMonitor,
  stopSession,
  setOverlayPaused,
  testLlmProfile,
  testLocalTranslationConfig,
  testTranslationApiKey,
  translationCredentialStatus,
  updateLookHelpConfig,
  updateLookHelpGeometry,
  updateOverlayConfig,
  updateOverlayGeometry,
} from "../api";
import {
  MEETING_SUMMARY_CUSTOM_PROMPT_MAX_CHARS,
  buildMeetingSummaryConfig,
  deriveConversationItems,
  deriveSourceSignalState,
  deriveTranslationActivity,
  meetingSummaryCustomPromptLength,
  meetingSummaryPromptPresetDescription,
  meetingSummaryPromptPresets,
  mergeTranscriptDelta,
  renderTranscript,
  selectMeetingSummaryPromptPreset,
  validateMeetingSummaryCustomPrompt,
  validateLlmProfileDraft,
} from "../transcript";
import {
  sourceLanguageOptionsForProvider,
  targetLanguageOptionsForProvider,
} from "../languages";
import type {
  AppErrorPayload,
  ApiKeySource,
  AudioDeviceInfo,
  AudioDevices,
  AudioOutputChannel,
  AudioLevelEvent,
  ConversationDisplayItem,
  Language,
  LookHelpConfig,
  LookHelpStatus,
  LookHelpUpdate,
  LlmProviderKind,
  LlmProviderProfile,
  LlmProviderProfileDraft,
  LocalTranslationConfigDraft,
  LocalPipelineStage,
  LocalTranslationTestResult,
  LocalVoice,
  VieNeuRuntimeProgress,
  VieNeuRuntimeStatus,
  ManualBoundaryEvent,
  ManualBoundaryReason,
  MeetingSummaryConfig,
  MeetingSummaryPromptPreset,
  MeetingSummaryResult,
  MeetingSummaryStatusEvent,
  OverlayConfig,
  OverlayStatus,
  OverlayTranslationUpdate,
  SessionConfig,
  SessionStatus,
  SourceSignalSnapshot,
  SourceSignalState,
  TranscriptScope,
  TranslationProvider,
  TranslationActivityState,
  TranslatedAudioLevelEvent,
  TranscriptItem,
  WhisperModelDownloadProgress,
  WhisperModelOption,
} from "../types";

type CompatibleIconProps = React.ComponentProps<FluentIcon> & { size?: number };

function withCompatibleSize(Icon: FluentIcon) {
  return function CompatibleIcon({ size, ...props }: CompatibleIconProps) {
    return <Icon {...props} fontSize={size ?? props.fontSize} />;
  };
}

const Activity = withCompatibleSize(ActivityIcon);
const ArrowLeft = withCompatibleSize(ArrowLeftIcon);
const AlertTriangle = withCompatibleSize(AlertTriangleIcon);
const Bot = withCompatibleSize(BotIcon);
const Check = withCompatibleSize(CheckIcon);
const Copy = withCompatibleSize(CopyIcon);
const Download = withCompatibleSize(DownloadIcon);
const FileText = withCompatibleSize(FileTextIcon);
const Headphones = withCompatibleSize(HeadphonesIcon);
const KeyRound = withCompatibleSize(KeyRoundIcon);
const Mic = withCompatibleSize(MicIcon);
const Move = withCompatibleSize(MoveIcon);
const PanelLeftClose = withCompatibleSize(PanelLeftCloseIcon);
const PanelLeftOpen = withCompatibleSize(PanelLeftOpenIcon);
const Pause = withCompatibleSize(PauseIcon);
const Play = withCompatibleSize(PlayIcon);
const RefreshCw = withCompatibleSize(RefreshCwIcon);
const Save = withCompatibleSize(SaveIcon);
const ScanText = withCompatibleSize(ScanTextIcon);
const Send = withCompatibleSize(SendIcon);
const Settings2 = withCompatibleSize(Settings2Icon);
const Square = withCompatibleSize(SquareIcon);
const Trash2 = withCompatibleSize(Trash2Icon);
const Volume2 = withCompatibleSize(Volume2Icon);
const X = withCompatibleSize(XIcon);

const channelOptions: Array<{ value: AudioOutputChannel; label: string }> = [
  { value: "all", label: "Both ears" },
  { value: "left", label: "Left ear" },
  { value: "right", label: "Right ear" },
];
const translationProviders: Array<{ value: TranslationProvider; label: string; title: string }> = [
  {
    value: "google_live_translate",
    label: "Google",
    title: "Google Live Translation",
  },
  {
    value: "openai_realtime",
    label: "OpenAI",
    title: "OpenAI Realtime Translation",
  },
  {
    value: "local_whisper",
    label: "Local",
    title: "Local Whisper",
  },
];
const providerKinds: Array<{ value: LlmProviderKind; label: string; title: string }> = [
  { value: "openai", label: "OpenAI", title: "OpenAI chat completions" },
  { value: "openai_compatible", label: "Compatible", title: "OpenAI-compatible endpoint" },
  { value: "ollama", label: "Ollama", title: "Local Ollama via /v1" },
  { value: "adk_litellm", label: "ADK", title: "ADK/LiteLLM model naming" },
];
const transcriptScopeOptions: Array<{ value: TranscriptScope; label: string }> = [
  { value: "both", label: "Both" },
  { value: "source", label: "Source" },
  { value: "translated", label: "Translated" },
];
const summaryPromptPresetOptions = meetingSummaryPromptPresets.map(({ id, label }) => ({
  value: id,
  label,
}));
const routingStorageKey = "baka-trans-routing-profile-v1";
const activeSessionStatuses: SessionStatus[] = ["listening", "translating", "speaking"];
const deviceAutoRefreshIntervalMs = 5000;
const isWindows = navigator.userAgent.toLowerCase().includes("windows");
const defaultLookHelpSystemPrompt =
  "You are Look & Help, a compact assistant for the visible screen region. Explain, summarize, or help with the provided OCR text. Be concise, practical, and do not invent details that are not present.";

function defaultLookHelpConfig(providerProfileId = ""): LookHelpConfig {
  return {
    providerProfileId,
    systemPrompt: defaultLookHelpSystemPrompt,
    promptPanelVisible: false,
    captureIntervalMs: 900,
    minimumConfidence: 0.45,
    opacity: 0.78,
    maxOcrInputChars: 6000,
  };
}

function beginOverlayDrag(event: React.MouseEvent<HTMLElement>) {
  if (event.button !== 0) {
    return;
  }
  void getCurrentWindow().startDragging();
}

function beginOverlayActionsDrag(event: React.MouseEvent<HTMLElement>) {
  const target = event.target;
  if (target instanceof Element && target.closest("button,input,select,textarea,a")) {
    event.stopPropagation();
    return;
  }
  beginOverlayDrag(event);
}

interface AudioLineRow {
  id: string;
  label: string;
  detail: string;
  state: string;
  leftLevel: number;
  rightLevel: number;
  disabled?: boolean;
}

interface RoutingProfile {
  inputDeviceId: string;
  outputDeviceId: string;
  translationOutputChannel: AudioOutputChannel;
  monitorOutputDeviceId: string;
  monitorOutputChannel: AudioOutputChannel;
  monitorOriginalAudio: boolean;
}

const emptyProfileDraft: LlmProviderProfileDraft = {
  name: "Local notes",
  kind: "ollama",
  model: "llama3.2",
  baseUrl: "http://localhost:11434/v1",
  timeoutSeconds: 60,
  maxOutputTokens: 1200,
  temperature: 0.2,
  enabled: true,
};

interface MainAppProps {
  experience?: "cloud" | "local";
  onRequestModeChange?: () => void;
}

export default function MainApp({
  experience = "cloud",
  onRequestModeChange,
}: MainAppProps) {
  const [devices, setDevices] = useState<AudioDevices>({ inputs: [], outputs: [] });
  const [translationProvider, setTranslationProvider] =
    useState<TranslationProvider>(
      experience === "local" ? "local_whisper" : "google_live_translate",
    );
  const [sourceLanguage, setSourceLanguage] = useState<Language>(
    experience === "local" ? "ja" : "auto",
  );
  const [targetLanguage, setTargetLanguage] = useState<Language>("vi");
  const [inputDeviceId, setInputDeviceId] = useState("");
  const [outputDeviceId, setOutputDeviceId] = useState("");
  const [translationOutputChannel, setTranslationOutputChannel] =
    useState<AudioOutputChannel>("all");
  const [monitorOutputDeviceId, setMonitorOutputDeviceId] = useState("");
  const [monitorOutputChannel, setMonitorOutputChannel] = useState<AudioOutputChannel>("all");
  const [monitorOriginalAudio, setMonitorOriginalAudio] = useState(false);
  const [fallbackEnabled, setFallbackEnabled] = useState(false);
  const [status, setStatus] = useState<SessionStatus>("idle");
  const [apiKeyStored, setApiKeyStored] = useState(false);
  const [apiKeySource, setApiKeySource] = useState<ApiKeySource | null>(null);
  const [apiKeyFingerprint, setApiKeyFingerprint] = useState("");
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [error, setError] = useState<AppErrorPayload | null>(null);
  const [keyTestMessage, setKeyTestMessage] = useState("");
  const [boundaryFeedback, setBoundaryFeedback] = useState("");
  const [sourceLevel, setSourceLevel] = useState<SourceSignalSnapshot | null>(null);
  const [translatedLevel, setTranslatedLevel] = useState({ peak: 0, rms: 0, sampleCount: 0 });
  const [transcript, setTranscript] = useState<TranscriptItem[]>([]);
  const [nowMs, setNowMs] = useState(() => Date.now());
  const [hasNewTranslations, setHasNewTranslations] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(() =>
    window.matchMedia("(min-width: 1280px)").matches,
  );
  const [activeSettings, setActiveSettings] = useState<SettingsSection>("audio");
  const [busy, setBusy] = useState(false);
  const [testingKey, setTestingKey] = useState(false);
  const [testingTone, setTestingTone] = useState<"translation" | "monitor" | null>(null);
  const [localMonitorActive, setLocalMonitorActive] = useState(false);
  const [refreshingDevices, setRefreshingDevices] = useState(false);
  const [autoRefreshDevices, setAutoRefreshDevices] = useState(true);
  const [lastRefreshedAt, setLastRefreshedAt] = useState<number | null>(null);
  const [llmProfiles, setLlmProfiles] = useState<LlmProviderProfile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState("");
  const [profileDraft, setProfileDraft] =
    useState<LlmProviderProfileDraft>(emptyProfileDraft);
  const [profileKeyDraft, setProfileKeyDraft] = useState("");
  const [testingProfile, setTestingProfile] = useState(false);
  const [profileTestMessage, setProfileTestMessage] = useState("");
  const [summaryConfig, setSummaryConfig] = useState<MeetingSummaryConfig>(
    buildMeetingSummaryConfig(""),
  );
  const [meetingNotes, setMeetingNotes] = useState<MeetingSummaryResult | null>(null);
  const [summaryStatus, setSummaryStatus] = useState("");
  const [summaryRunning, setSummaryRunning] = useState(false);
  const [localConfigDraft, setLocalConfigDraft] = useState<LocalTranslationConfigDraft>(
    defaultLocalTranslationConfig,
  );
  const [localConfigDirty, setLocalConfigDirty] = useState(false);
  const [localConfigSaving, setLocalConfigSaving] = useState(false);
  const [localConfigTesting, setLocalConfigTesting] = useState(false);
  const [localConfigTest, setLocalConfigTest] = useState<LocalTranslationTestResult | null>(null);
  const [localVoices, setLocalVoices] = useState<LocalVoice[]>([]);
  const [localVoicesLoading, setLocalVoicesLoading] = useState(false);
  const [localVoicePreviewing, setLocalVoicePreviewing] = useState(false);
  const [whisperModels, setWhisperModels] = useState<WhisperModelOption[]>([]);
  const [selectedWhisperModelId, setSelectedWhisperModelId] = useState("");
  const [whisperDownload, setWhisperDownload] =
    useState<WhisperModelDownloadProgress | null>(null);
  const [whisperDownloading, setWhisperDownloading] = useState(false);
  const [vieneuRuntime, setVieNeuRuntime] = useState<VieNeuRuntimeStatus | null>(null);
  const [vieneuProgress, setVieNeuProgress] = useState<VieNeuRuntimeProgress | null>(null);
  const [vieneuBusy, setVieNeuBusy] = useState(false);
  const [localPipelineStage, setLocalPipelineStage] =
    useState<LocalPipelineStage>("listening");
  const conversationFeedRef = useRef<HTMLDivElement | null>(null);
  const feedAtBottomRef = useRef(true);
  const transcriptEndRef = useRef<HTMLDivElement | null>(null);
  const deviceRefreshInFlightRef = useRef(false);
  const routingProfileRef = useRef<RoutingProfile>({
    inputDeviceId: "",
    outputDeviceId: "",
    translationOutputChannel: "all",
    monitorOutputDeviceId: "",
    monitorOutputChannel: "all",
    monitorOriginalAudio: false,
  });
  const settingsTriggerRef = useRef<HTMLButtonElement>(null);
  const cloudLanguagesRef = useRef<{ source: Language; target: Language }>({
    source: "auto",
    target: "vi",
  });

  const selectedInput = devices.inputs.find((device) => device.id === inputDeviceId);
  const selectedOutput = devices.outputs.find((device) => device.id === outputDeviceId);
  const selectedMonitorOutput = devices.outputs.find(
    (device) => device.id === monitorOutputDeviceId,
  );
  const outputMonitorConflict =
    hasOutputMonitorConflict(
      selectedOutput,
      translationOutputChannel,
      monitorOriginalAudio ? selectedMonitorOutput : undefined,
      monitorOutputChannel,
    );
  const effectiveMonitorOriginalAudio = !isWindows && monitorOriginalAudio && !outputMonitorConflict;
  const routingWarnings = getRoutingWarnings(
    selectedOutput,
    translationOutputChannel,
    effectiveMonitorOriginalAudio ? selectedMonitorOutput : undefined,
    monitorOutputChannel,
  );
  const targetLanguageOptions = useMemo(
    () => targetLanguageOptionsForProvider(translationProvider),
    [translationProvider],
  );
  const sourceOptions = useMemo(
    () => sourceLanguageOptionsForProvider(translationProvider),
    [translationProvider],
  );
  const signalSessionStatus: SessionStatus = localMonitorActive ? "listening" : status;
  const sourceSignalState = deriveSourceSignalState(
    sourceLevel,
    inputDeviceId,
    signalSessionStatus,
    nowMs,
  );
  const visibleSourceLevel =
    sourceLevel && sourceLevel.inputDeviceId === inputDeviceId
      ? sourceLevel
      : { peak: 0, rms: 0 };
  const conversationItems = useMemo(() => deriveConversationItems(transcript), [transcript]);
  const latestConversationItem = conversationItems[conversationItems.length - 1];
  const translationActivity = deriveTranslationActivity(
    status,
    latestConversationItem,
    sourceSignalState,
    translatedLevel.peak,
  );

  const localProviderConfigured =
    !localConfigDirty && validateLocalTranslationDraft(localConfigDraft, vieneuRuntime).length === 0;
  const localProviderHealthy = Boolean(localConfigTest?.ok && localProviderConfigured);
  const providerReady =
    translationProvider === "local_whisper"
      ? localProviderConfigured && outputDeviceId.length > 0
      : apiKeyStored && outputDeviceId.length > 0;
  const canStart =
    (status === "idle" || status === "error") &&
    !localMonitorActive &&
    inputDeviceId.length > 0 &&
    providerReady &&
    testingTone === null &&
    (!monitorOriginalAudio || monitorOutputDeviceId.length > 0);
  const sessionActive = activeSessionStatuses.includes(status);
  const canForceBoundary =
    (translationProvider === "openai_realtime" ||
      translationProvider === "local_whisper") &&
    sessionActive;
  const canPause = canForceBoundary;
  const canResume = status === "paused";
  const canStop = status !== "idle" && status !== "stopping";
  const testToneActive = testingTone !== null;
  const translationToneActive = testingTone === "translation";
  const monitorToneActive = testingTone === "monitor";
  const canTestAudio = status === "idle" && !busy && !localMonitorActive && !testToneActive;
  const canToggleLocalMonitor =
    status === "idle" &&
    !busy &&
    !testToneActive &&
    (localMonitorActive || (inputDeviceId.length > 0 && outputDeviceId.length > 0));
  const inputSignalPercent = Math.round(visibleSourceLevel.peak * 100);
  const inputSignalRmsPercent = Math.round(visibleSourceLevel.rms * 100);
  const inputSignalLabel =
    sourceSignalState === "receiving"
      ? `${inputSignalPercent}% peak`
      : sourceSignalState === "silent"
        ? "Silent stream"
        : sourceSignalState === "stale"
          ? "No recent audio"
          : localMonitorActive
            ? "Monitoring"
            : status === "idle"
              ? "Start to listen"
              : "Waiting";
  const translationOutLevel = localMonitorActive
    ? inputSignalPercent
    : Math.round(translatedLevel.peak * 100);
  const monitorOutLevel = effectiveMonitorOriginalAudio && sessionActive ? inputSignalPercent : 0;
  const audioLineRows: AudioLineRow[] = [
    {
      id: "input",
      label: "Input",
      detail: selectedInput?.name ?? "No input",
      state:
        inputSignalPercent > 3
          ? "signal"
          : localMonitorActive
            ? "listening"
            : sessionActive
              ? "listening"
              : "idle",
      leftLevel: inputSignalPercent,
      rightLevel: inputSignalPercent,
      disabled: !selectedInput,
    },
    {
      id: "translation",
      label: "Translated out",
      detail:
        formatRoute(selectedOutput, translationOutputChannel),
      state: localMonitorActive
        ? translationOutLevel > 3
          ? "signal"
          : "armed"
        : status === "speaking"
          ? "speaking"
          : sessionActive
            ? "armed"
            : "idle",
      leftLevel: channelLevel(translationOutputChannel, "left", translationOutLevel),
      rightLevel: channelLevel(translationOutputChannel, "right", translationOutLevel),
      disabled: !selectedOutput,
    },
    {
      id: "monitor",
      label: "Original out",
      detail: effectiveMonitorOriginalAudio ? formatRoute(selectedMonitorOutput, monitorOutputChannel) : "Off",
      state: effectiveMonitorOriginalAudio ? (monitorOutLevel > 3 ? "signal" : "armed") : "off",
      leftLevel: channelLevel(monitorOutputChannel, "left", monitorOutLevel),
      rightLevel: channelLevel(monitorOutputChannel, "right", monitorOutLevel),
      disabled: !effectiveMonitorOriginalAudio || !selectedMonitorOutput,
    },
  ];
  const readinessLabel =
    status === "idle" ? (canStart ? "Ready" : "Setup needed") : labelStatus(status);
  const sessionPanelHealthy =
    (status === "idle" && canStart) || status === "starting" || sessionActive;
  const selectedProfile = llmProfiles.find((profile) => profile.id === selectedProfileId);
  const profileDraftErrors = validateLlmProfileDraft(profileDraft);
  const summaryPromptValidation = validateMeetingSummaryCustomPrompt(
    summaryConfig.promptPreset,
    summaryConfig.customSystemPrompt,
  );
  const summaryPromptDescription = meetingSummaryPromptPresetDescription(summaryConfig.promptPreset);
  const summaryPromptLength = meetingSummaryCustomPromptLength(summaryConfig.customSystemPrompt);
  const canRunSummary =
    Boolean(selectedProfileId) &&
    !summaryRunning &&
    !summaryPromptValidation &&
    transcript.some((item) => item.sourceText.trim() || item.translatedText.trim());

  const config: SessionConfig = useMemo(
    () => ({
      translationProvider,
      sourceLanguage,
      targetLanguage,
      translationStyle: "technical_meeting_safe",
      inputDeviceId,
      outputDeviceId,
      translationOutputChannel,
      monitorOutputDeviceId,
      monitorOutputChannel,
      monitorOriginalAudio: effectiveMonitorOriginalAudio,
      voiceId: "marin",
      fallbackEnabled,
    }),
    [
      effectiveMonitorOriginalAudio,
      fallbackEnabled,
      inputDeviceId,
      monitorOutputChannel,
      monitorOutputDeviceId,
      outputDeviceId,
      sourceLanguage,
      targetLanguage,
      translationOutputChannel,
      translationProvider,
    ],
  );

  useEffect(() => {
    const desktop = window.matchMedia("(min-width: 1280px)");
    const syncPanel = (event: MediaQueryListEvent) => {
      if (event.matches && activeSettings !== "live") {
        setSettingsOpen(true);
      } else if (!event.matches) {
        setSettingsOpen(false);
      }
    };
    desktop.addEventListener("change", syncPanel);
    return () => desktop.removeEventListener("change", syncPanel);
  }, [activeSettings]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    void hydrate();

    const unlisten = Promise.all([
      listen<SessionStatus>("session-status", (event) => {
        setStatus(event.payload);
        if (event.payload === "idle") {
          setSourceLevel(null);
          setTranslatedLevel({ peak: 0, rms: 0, sampleCount: 0 });
          setLocalPipelineStage("listening");
        }
      }),
      listen<TranscriptItem>("transcript-update", (event) => {
        setTranscript((items) => mergeTranscriptDelta(items, event.payload));
      }),
      listen<ManualBoundaryEvent>("manual-boundary-status", (event) => {
        setBoundaryFeedback(event.payload.message);
      }),
      listen<AudioLevelEvent>("audio-level", (event) => {
        const receivedAtMs = Date.now();
        startTransition(() => {
          setNowMs(receivedAtMs);
          setSourceLevel({
            inputDeviceId: event.payload.inputDeviceId,
            peak: Math.max(0, Math.min(1, event.payload.peak)),
            rms: Math.max(0, Math.min(1, event.payload.rms)),
            receivedAtMs,
          });
        });
      }),
      listen<TranslatedAudioLevelEvent>("translated-audio-level", (event) => {
        startTransition(() => {
          setTranslatedLevel({
            peak: Math.max(0, Math.min(1, event.payload.peak)),
            rms: Math.max(0, Math.min(1, event.payload.rms)),
            sampleCount: event.payload.sampleCount,
          });
        });
      }),
      listen<LocalPipelineStage>("local-pipeline-stage", (event) => {
        setLocalPipelineStage(event.payload);
      }),
      listen<WhisperModelDownloadProgress>("whisper-model-download-progress", (event) => {
        setWhisperDownload(event.payload);
      }),
      listen<VieNeuRuntimeProgress>("vieneu-runtime-progress", (event) => {
        setVieNeuProgress(event.payload);
        setVieNeuRuntime((current) =>
          current
            ? {
                ...current,
                phase: event.payload.phase,
                running:
                  event.payload.phase === "ready"
                    ? true
                    : event.payload.phase === "starting" || event.payload.phase === "recovering"
                      ? false
                      : current.running,
                installedBytes:
                  event.payload.phase === "installed"
                    ? event.payload.totalBytes
                    : current.installedBytes,
                message: event.payload.message,
              }
            : current,
        );
      }),
      listen<AppErrorPayload>("app-error", (event) => setError(event.payload)),
      listen<MeetingSummaryStatusEvent>("summary-agent-status", (event) => {
        setSummaryStatus(event.payload.message);
        setSummaryRunning(event.payload.status === "running");
      }),
      listen<MeetingSummaryResult>("meeting-summary-update", (event) => {
        setMeetingNotes(event.payload);
        setSummaryRunning(false);
      }),
    ]);

    return () => {
      void unlisten.then((callbacks) => callbacks.forEach((callback) => callback()));
    };
  }, []);

  useEffect(() => {
    if (!targetLanguageOptions.some((option) => option.value === targetLanguage)) {
      setTargetLanguage(targetLanguageOptions[0]?.value ?? "vi");
    }
  }, [targetLanguage, targetLanguageOptions]);

  useEffect(() => {
    if (translationProvider !== "local_whisper") {
      cloudLanguagesRef.current = { source: sourceLanguage, target: targetLanguage };
    }
  }, [sourceLanguage, targetLanguage, translationProvider]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    void refreshTranslationCredentialStatus(translationProvider);
  }, [translationProvider]);

  useEffect(() => {
    if (!isTauri() || !autoRefreshDevices) {
      return;
    }

    const interval = window.setInterval(() => {
      void refreshAudioDevices({ silent: true });
    }, deviceAutoRefreshIntervalMs);
    return () => window.clearInterval(interval);
  }, [autoRefreshDevices]);

  useEffect(() => {
    setSourceLevel(null);
    setNowMs(Date.now());
  }, [inputDeviceId]);

  useEffect(() => {
    if (!isSourceSignalStatusActive(signalSessionStatus)) {
      setNowMs(Date.now());
      return;
    }

    const interval = window.setInterval(() => setNowMs(Date.now()), 500);
    return () => window.clearInterval(interval);
  }, [signalSessionStatus]);

  useEffect(() => {
    if (feedAtBottomRef.current) {
      transcriptEndRef.current?.scrollIntoView({ block: "end" });
      setHasNewTranslations(false);
    } else if (conversationItems.length > 0) {
      setHasNewTranslations(true);
    }
  }, [conversationItems]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!event.metaKey || event.key !== "Enter" || !canForceBoundary || busy) {
        return;
      }
      event.preventDefault();
      void requestBoundary("keyboard_shortcut");
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [busy, canForceBoundary]);

  async function hydrate() {
    setBusy(true);
    try {
      const [deviceList, appStatus, transcriptSnapshot, profiles, localConfig, models, runtime] =
        await Promise.all([
        listAudioDevices(),
        getAppStatus(),
        getTranscriptSnapshot(),
        listLlmProfiles(),
        getLocalTranslationConfig(),
        experience === "local" ? listWhisperModels() : Promise.resolve([]),
        experience === "local" ? getVieNeuRuntimeStatus() : Promise.resolve(null),
      ]);
      let voices: LocalVoice[] = [];
      let voiceLoadError: AppErrorPayload | null = null;
      if (
        experience === "local" &&
        (localConfig.ttsProvider === "system" || runtime?.modelInstalled)
      ) {
        try {
          voices = await listLocalTtsVoices(localConfig.ttsProvider);
        } catch (cause) {
          voiceLoadError = normalizeError(cause);
        }
      }
      setDevices(deviceList);
      setStatus(appStatus.sessionStatus);
      setTranscript(transcriptSnapshot);
      applyAppStatus(appStatus);
      await refreshTranslationCredentialStatus(translationProvider);
      applyProfiles(profiles);
      const { schemaVersion: _schemaVersion, ...localDraft } = localConfig;
      const preferredVoice =
        voices.find((voice) => voice.language.toLowerCase().startsWith("vi")) ?? voices[0];
      const migratedLocalDraft =
        localDraft.voiceId || !preferredVoice
          ? localDraft
          : { ...localDraft, voiceId: preferredVoice.id };
      setLocalVoices(voices);
      setVieNeuRuntime(runtime);
      setWhisperModels(models);
      setSelectedWhisperModelId(
        models.find((model) => model.recommended)?.id ?? models[0]?.id ?? "",
      );
      setLocalConfigDraft(migratedLocalDraft);
      setLocalConfigDirty(migratedLocalDraft !== localDraft);
      setLocalConfigTest(null);
      setKeyTestMessage("");
      const storedRouting = readRoutingProfile();
      applyRoutingProfile(resolveRoutingProfile(deviceList, storedRouting), true);
      setLastRefreshedAt(Date.now());
      if (voiceLoadError) {
        setError(voiceLoadError);
      }
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
  }

  async function refreshAudioDevices({ silent = false }: { silent?: boolean } = {}) {
    if (deviceRefreshInFlightRef.current) {
      return;
    }

    deviceRefreshInFlightRef.current = true;
    if (!silent) {
      setRefreshingDevices(true);
    }
    try {
      const deviceList = await listAudioDevices();
      setDevices(deviceList);
      applyRoutingProfile(resolveRoutingProfile(deviceList, routingProfileRef.current), true);
      setLastRefreshedAt(Date.now());
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      deviceRefreshInFlightRef.current = false;
      if (!silent) {
        setRefreshingDevices(false);
      }
    }
  }

  function applyRoutingProfile(profile: RoutingProfile, shouldPersist = false) {
    const previousProfile = routingProfileRef.current;
    const routingProfileChanged =
      previousProfile.inputDeviceId !== profile.inputDeviceId ||
      previousProfile.outputDeviceId !== profile.outputDeviceId ||
      previousProfile.translationOutputChannel !== profile.translationOutputChannel ||
      previousProfile.monitorOutputDeviceId !== profile.monitorOutputDeviceId ||
      previousProfile.monitorOutputChannel !== profile.monitorOutputChannel ||
      previousProfile.monitorOriginalAudio !== profile.monitorOriginalAudio;

    routingProfileRef.current = profile;
    setInputDeviceId(profile.inputDeviceId);
    setOutputDeviceId(profile.outputDeviceId);
    setTranslationOutputChannel(profile.translationOutputChannel);
    setMonitorOutputDeviceId(profile.monitorOutputDeviceId);
    setMonitorOutputChannel(profile.monitorOutputChannel);
    setMonitorOriginalAudio(profile.monitorOriginalAudio);
    if (shouldPersist && routingProfileChanged) {
      persistRoutingProfile(profile);
    }
  }

  function saveRoutingProfile(profile: RoutingProfile) {
    routingProfileRef.current = profile;
    persistRoutingProfile(profile);
  }

  function applyProfiles(profiles: LlmProviderProfile[]) {
    setLlmProfiles(profiles);
    const nextSelected =
      profiles.find((profile) => profile.id === selectedProfileId)?.id ??
      profiles.find((profile) => profile.enabled)?.id ??
      profiles[0]?.id ??
      "";
    setSelectedProfileId(nextSelected);
    setSummaryConfig((current) => ({ ...current, providerProfileId: nextSelected }));
    if (nextSelected) {
      const profile = profiles.find((item) => item.id === nextSelected);
      if (profile) {
        setProfileDraft(profileToDraft(profile));
      }
    }
  }

  async function refreshTranslationCredentialStatus(provider: TranslationProvider) {
    try {
      const credential = await translationCredentialStatus(provider);
      setApiKeyStored(credential.hasApiKey);
      setApiKeySource(credential.apiKeySource ?? null);
      setApiKeyFingerprint(credential.apiKeyFingerprint ?? "");
    } catch (cause) {
      setApiKeyStored(false);
      setApiKeySource(null);
      setApiKeyFingerprint("");
      setError(normalizeError(cause));
    }
  }

  function selectTranslationProvider(provider: TranslationProvider) {
    if (provider === translationProvider) {
      return;
    }
    if (provider === "local_whisper") {
      cloudLanguagesRef.current = { source: sourceLanguage, target: targetLanguage };
      setSourceLanguage("ja");
      setTargetLanguage("vi");
    } else if (translationProvider === "local_whisper") {
      setSourceLanguage(cloudLanguagesRef.current.source);
      setTargetLanguage(cloudLanguagesRef.current.target);
    }
    setTranslationProvider(provider);
    setApiKeyDraft("");
    setKeyTestMessage("");
  }

  async function saveLocalConfig() {
    setLocalConfigSaving(true);
    setError(null);
    try {
      const saved = await saveLocalTranslationConfig(localConfigDraft);
      const { schemaVersion: _schemaVersion, ...draft } = saved;
      setLocalConfigDraft(draft);
      setLocalConfigDirty(false);
      setLocalConfigTest(null);
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setLocalConfigSaving(false);
    }
  }

  async function testLocalConfig() {
    setLocalConfigTesting(true);
    setError(null);
    try {
      setLocalConfigTest(await testLocalTranslationConfig(localConfigDraft));
    } catch (cause) {
      setLocalConfigTest(null);
      setError(normalizeError(cause));
    } finally {
      setLocalConfigTesting(false);
    }
  }

  async function previewLocalVoice() {
    setLocalVoicePreviewing(true);
    setError(null);
    try {
      await previewLocalTts(localConfigDraft, outputDeviceId, translationOutputChannel);
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setLocalVoicePreviewing(false);
    }
  }

  async function refreshLocalVoices() {
    setLocalVoicesLoading(true);
    setError(null);
    try {
      const voices = await listLocalTtsVoices(localConfigDraft.ttsProvider);
      setLocalVoices(voices);
      if (!voices.some((voice) => voice.id === localConfigDraft.voiceId)) {
        const preferredVoice =
          voices.find((voice) => voice.language.toLowerCase().startsWith("vi")) ?? voices[0];
        setLocalConfigDraft((current) => ({
          ...current,
          voiceId: preferredVoice?.id ?? "",
        }));
        setLocalConfigDirty(true);
        setLocalConfigTest(null);
      }
    } catch (cause) {
      setLocalVoices([]);
      setError(normalizeError(cause));
    } finally {
      setLocalVoicesLoading(false);
    }
  }

  async function installManagedVieNeu() {
    if (vieneuBusy) return;
    setVieNeuBusy(true);
    setError(null);
    try {
      const runtime = await installVieNeuRuntime();
      setVieNeuRuntime(runtime);
      setVieNeuProgress(null);
      if (localConfigDraft.ttsProvider === "vieneu" && runtime.modelInstalled) {
        await refreshLocalVoices();
      }
    } catch (cause) {
      setError(normalizeError(cause));
      setVieNeuRuntime(await getVieNeuRuntimeStatus().catch(() => vieneuRuntime));
    } finally {
      setVieNeuBusy(false);
    }
  }

  async function cancelManagedVieNeu() {
    await cancelVieNeuRuntimeInstall().catch((cause) => setError(normalizeError(cause)));
  }

  async function restartManagedVieNeu() {
    if (vieneuBusy) return;
    setVieNeuBusy(true);
    setError(null);
    try {
      const runtime = await restartVieNeuRuntime();
      setVieNeuRuntime(runtime);
      setVieNeuProgress(null);
      if (localConfigDraft.ttsProvider === "vieneu") {
        await refreshLocalVoices();
      }
    } catch (cause) {
      setError(normalizeError(cause));
      setVieNeuRuntime(await getVieNeuRuntimeStatus().catch(() => vieneuRuntime));
    } finally {
      setVieNeuBusy(false);
    }
  }

  async function downloadSelectedWhisperModel() {
    if (!selectedWhisperModelId || whisperDownloading) {
      return;
    }
    const selectedModel = whisperModels.find((model) => model.id === selectedWhisperModelId);
    setWhisperDownloading(true);
    setWhisperDownload({
      modelId: selectedWhisperModelId,
      fileName: selectedModel?.fileName ?? selectedWhisperModelId,
      downloadedBytes: 0,
      percent: 0,
      status: "downloading",
      message: "Preparing download…",
    });
    setError(null);
    try {
      const modelPath = await downloadWhisperModel(selectedWhisperModelId);
      setLocalConfigDraft((current) => ({ ...current, modelPath }));
      setLocalConfigDirty(true);
      setLocalConfigTest(null);
    } catch (cause) {
      const normalized = normalizeError(cause);
      setWhisperDownload((current) => ({
        modelId: current?.modelId ?? selectedWhisperModelId,
        fileName: current?.fileName ?? selectedModel?.fileName ?? selectedWhisperModelId,
        downloadedBytes: current?.downloadedBytes ?? 0,
        totalBytes: current?.totalBytes,
        percent: current?.percent,
        status: "error",
        message: normalized.message,
      }));
      setError(normalized);
    } finally {
      setWhisperDownloading(false);
    }
  }

  async function saveKey() {
    setBusy(true);
    setError(null);
    try {
      await saveTranslationApiKey(translationProvider, apiKeyDraft);
      const credential = await translationCredentialStatus(translationProvider);
      setApiKeyStored(credential.hasApiKey);
      setApiKeySource(credential.apiKeySource ?? null);
      setApiKeyFingerprint(credential.apiKeyFingerprint ?? "");
      setKeyTestMessage("");
      if (credential.hasApiKey) {
        setApiKeyDraft("");
      } else {
        setError({
          code: "missing_api_key",
          message: "The API key was saved, but the app could not read it back.",
        });
      }
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
  }

  async function saveSummaryProfile() {
    const errors = validateLlmProfileDraft(profileDraft);
    if (errors.length > 0) {
      setError({ code: "invalid_llm_profile", message: errors[0] });
      return;
    }
    setBusy(true);
    setError(null);
    setProfileTestMessage("");
    try {
      const submittedApiKey = profileKeyDraft.trim();
      const saved = await saveLlmProfile({
        id: profileDraft.id,
        name: profileDraft.name.trim(),
        kind: profileDraft.kind,
        model: profileDraft.model.trim(),
        baseUrl: profileDraft.baseUrl?.trim() || undefined,
        apiKey: submittedApiKey || undefined,
        timeoutSeconds: profileDraft.timeoutSeconds,
        maxOutputTokens: profileDraft.maxOutputTokens,
        temperature: profileDraft.temperature,
        enabled: profileDraft.enabled,
      });
      const profiles = await listLlmProfiles();
      const persisted = profiles.find((profile) => profile.id === saved.id);
      if (!persisted) {
        throw {
          code: "llm_profile_readback_error",
          message: "The profile was saved, but the app could not read it back.",
        };
      }
      setLlmProfiles(profiles);
      setSelectedProfileId(persisted.id);
      setSummaryConfig((current) => ({ ...current, providerProfileId: persisted.id }));
      setProfileDraft(profileToDraft(persisted));
      if (submittedApiKey && !persisted.hasApiKey) {
        throw {
          code: "missing_llm_api_key",
          message: "The LLM API key was saved, but the app could not read it back.",
        };
      }
      if (persisted.hasApiKey) {
        setProfileKeyDraft("");
      }
      setProfileTestMessage(
        submittedApiKey
          ? `Profile settings and LLM key saved ${persisted.apiKeyFingerprint ?? ""}.`
          : persisted.hasApiKey
            ? `Profile settings saved. LLM key ${persisted.apiKeyFingerprint ?? ""} remains stored.`
            : "Profile settings saved.",
      );
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
  }

  function updateProfileDraft(patch: Partial<LlmProviderProfileDraft>) {
    setProfileDraft((current) => ({ ...current, ...patch }));
  }

  async function deleteSelectedProfile() {
    if (!selectedProfileId) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await deleteLlmProfile(selectedProfileId);
      const profiles = await listLlmProfiles();
      setProfileDraft(emptyProfileDraft);
      setProfileKeyDraft("");
      applyProfiles(profiles);
      setProfileTestMessage("Profile deleted.");
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
  }

  async function testSelectedProfile() {
    if (!selectedProfileId) {
      return;
    }
    setTestingProfile(true);
    setError(null);
    setProfileTestMessage("");
    try {
      const result = await testLlmProfile(selectedProfileId);
      setProfileTestMessage(`${result.message} ${result.baseUrl}`);
      const profiles = await listLlmProfiles();
      applyProfiles(profiles);
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setTestingProfile(false);
    }
  }

  async function runSummary() {
    if (!selectedProfileId || summaryPromptValidation) {
      return;
    }
    setSummaryRunning(true);
    setSummaryStatus("Starting summary");
    setError(null);
    try {
      const result = await runMeetingSummaryAgent({
        ...summaryConfig,
        providerProfileId: selectedProfileId,
      });
      setMeetingNotes(result);
      setSummaryStatus("Meeting notes ready");
    } catch (cause) {
      setSummaryRunning(false);
      setSummaryStatus("Summary failed");
      setError(normalizeError(cause));
    }
  }

  function applyAppStatus(appStatus: Awaited<ReturnType<typeof getAppStatus>>) {
    setStatus(appStatus.sessionStatus);
  }

  async function runCommand(command: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await command();
    } catch (cause) {
      const normalized = normalizeError(cause);
      if (normalized.code === "missing_api_key") {
        setApiKeyStored(false);
      }
      setError(normalized);
    } finally {
      setBusy(false);
    }
  }

  async function testStoredKey() {
    setTestingKey(true);
    setError(null);
    setKeyTestMessage("");
    try {
      const result = await testTranslationApiKey(translationProvider);
      setApiKeyStored(true);
      setApiKeySource(result.source);
      setApiKeyFingerprint(result.fingerprint);
      setKeyTestMessage(result.message);
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setTestingKey(false);
    }
  }

  async function requestBoundary(reason: ManualBoundaryReason) {
    setBusy(true);
    setError(null);
    try {
      await forceTranslateBoundary(reason);
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
  }

  async function testTone(
    kind: "translation" | "monitor",
    deviceId: string,
    outputChannel: AudioOutputChannel,
  ) {
    setTestingTone(kind);
    setError(null);
    try {
      if (testingTone === kind) {
        await stopTestTone();
        setTestingTone(null);
        return;
      }
      if (testingTone !== null) {
        await stopTestTone();
      }
      setTestingTone(kind);
      await playTestTone(deviceId, outputChannel);
    } catch (cause) {
      setTestingTone(null);
      setError(normalizeError(cause));
    }
  }

  async function toggleLocalMonitor() {
    setBusy(true);
    setError(null);
    try {
      if (localMonitorActive) {
        await stopLocalMonitor();
        setLocalMonitorActive(false);
        setSourceLevel(null);
        return;
      }
      await startLocalMonitor(inputDeviceId, outputDeviceId, translationOutputChannel);
      setLocalMonitorActive(true);
    } catch (cause) {
      setLocalMonitorActive(false);
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
  }

  async function showTransparentOverlay() {
    setBusy(true);
    setError(null);
    try {
      await openOverlayWindow({
        sourceLanguage,
        targetLanguage,
        captureIntervalMs: 800,
        minimumConfidence: 0.45,
        opacity: 0.72,
        geminiModel: "models/gemini-2.5-flash",
      });
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
  }

  async function showLookHelpOverlay() {
    setBusy(true);
    setError(null);
    try {
      await openLookHelpWindow(defaultLookHelpConfig(selectedProfileId));
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
  }

  async function doExport(format: "text" | "markdown") {
    const localContent = renderTranscript(transcript, format, meetingNotes);
    try {
      const backend = await exportTranscript(format);
      const content =
        meetingNotes && backend.content ? renderTranscript(transcript, format, meetingNotes) : backend.content;
      downloadText(backend.fileName, content || localContent);
    } catch {
      downloadText(`baka-trans-transcript.${format === "markdown" ? "md" : "txt"}`, localContent);
    }
  }

  function updateInputDevice(deviceId: string) {
    setInputDeviceId(deviceId);
    saveRoutingProfile({ ...routingProfileRef.current, inputDeviceId: deviceId });
  }

  function updateOutputDevice(deviceId: string) {
    setOutputDeviceId(deviceId);
    saveRoutingProfile({ ...routingProfileRef.current, outputDeviceId: deviceId });
  }

  function updateTranslationOutputChannel(outputChannel: AudioOutputChannel) {
    setTranslationOutputChannel(outputChannel);
    saveRoutingProfile({
      ...routingProfileRef.current,
      translationOutputChannel: outputChannel,
    });
  }

  function updateMonitorOutputDevice(deviceId: string) {
    setMonitorOutputDeviceId(deviceId);
    saveRoutingProfile({ ...routingProfileRef.current, monitorOutputDeviceId: deviceId });
  }

  function updateMonitorOutputChannel(outputChannel: AudioOutputChannel) {
    setMonitorOutputChannel(outputChannel);
    saveRoutingProfile({
      ...routingProfileRef.current,
      monitorOutputChannel: outputChannel,
    });
  }

  function updateMonitorEnabled(enabled: boolean) {
    setMonitorOriginalAudio(enabled);
    saveRoutingProfile({ ...routingProfileRef.current, monitorOriginalAudio: enabled });
  }

  function handleConversationScroll() {
    const node = conversationFeedRef.current;
    if (!node) {
      return;
    }

    const isNearBottom = node.scrollHeight - node.scrollTop - node.clientHeight < 72;
    feedAtBottomRef.current = isNearBottom;
    if (isNearBottom) {
      setHasNewTranslations(false);
    }
  }

  function jumpToLatestTranslation() {
    transcriptEndRef.current?.scrollIntoView({ block: "end", behavior: "smooth" });
    feedAtBottomRef.current = true;
    setHasNewTranslations(false);
  }

  return (
    <main className={`app-shell experience-${experience}`}>
      <header className="topbar">
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true">
            <Activity size={22} />
          </div>
          <div>
            <h1>Baka Trans</h1>
            <div className="status-row">
              <span className={`status-dot status-${status}`} />
              <span>{labelStatus(status)}</span>
              {translationProvider === "local_whisper" ? (
                <span className={`status-chip ${localProviderConfigured ? "ok" : "warn"}`}>
                  {localProviderConfigured ? <Check size={14} /> : <Settings2 size={14} />}
                  {localProviderHealthy
                    ? "Local ready"
                    : localProviderConfigured
                      ? "Local configured"
                      : "Local setup needed"}
                </span>
              ) : apiKeyStored ? (
                <span className="status-chip ok">
                  <Check size={14} /> Key stored
                </span>
              ) : (
                <span className="status-chip warn">
                  <KeyRound size={14} /> Key needed
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="top-actions">
          {onRequestModeChange ? (
            <button
              className="icon-button mode-change-button"
              onClick={onRequestModeChange}
              disabled={status !== "idle" && status !== "error"}
              title={
                status === "idle" || status === "error"
                  ? "Change translation mode"
                  : "Stop the session before changing mode"
              }
              aria-label="Change translation mode"
              type="button"
            >
              <ArrowLeft size={18} />
              <span>Change mode</span>
            </button>
          ) : null}
          <button
            className="icon-button"
            onClick={() => {
              if (settingsOpen) {
                setSettingsOpen(false);
                setActiveSettings("live");
              } else {
                setActiveSettings("audio");
                setSettingsOpen(true);
              }
            }}
            ref={settingsTriggerRef}
            title={settingsOpen ? "Hide settings" : "Show settings"}
            aria-label={settingsOpen ? "Hide settings" : "Show settings"}
            aria-controls="session-settings"
            aria-expanded={settingsOpen}
            type="button"
          >
            {settingsOpen ? <PanelLeftClose size={18} /> : <PanelLeftOpen size={18} />}
          </button>
          <button
            className="icon-button overlay-launch-button"
            onClick={showTransparentOverlay}
            disabled={busy}
            title="Look through"
            aria-label="Open look through OCR overlay"
          >
            <ScanText size={18} />
            <span>Look through</span>
          </button>
          <button
            className="icon-button overlay-launch-button"
            onClick={showLookHelpOverlay}
            disabled={busy}
            title="Look & Help"
            aria-label="Open Look and Help OCR assistant overlay"
          >
            <Bot size={18} />
            <span>Look & Help</span>
          </button>
          <button
            className="icon-button"
            onClick={() => void refreshAudioDevices()}
            disabled={busy || refreshingDevices}
            title="Refresh devices"
            aria-label="Refresh devices"
          >
            <RefreshCw className={refreshingDevices ? "spin-icon" : ""} size={18} />
          </button>
          <button
            className="icon-button"
            onClick={() => void doExport("text")}
            disabled={transcript.length === 0}
            title="Export text"
            aria-label="Export text"
          >
            <Download size={18} />
          </button>
          <button
            className="icon-button"
            onClick={() => void doExport("markdown")}
            disabled={transcript.length === 0}
            title="Export Markdown"
            aria-label="Export Markdown"
          >
            <span className="button-text">MD</span>
          </button>
        </div>
      </header>

      <SessionCommandBar
        sourceLanguage={sourceLanguage}
        targetLanguage={targetLanguage}
        sourceOptions={sourceOptions}
        targetOptions={targetLanguageOptions}
        onSourceChange={(value) => setSourceLanguage(value as Language)}
        onTargetChange={(value) => setTargetLanguage(value as Language)}
        fallbackEnabled={fallbackEnabled}
        onFallbackChange={setFallbackEnabled}
        canStart={canStart}
        canPause={canPause}
        canResume={canResume}
        canStop={canStop}
        canTranslateNow={canForceBoundary}
        busy={busy}
        paused={status === "paused"}
        readinessLabel={readinessLabel}
        boundaryFeedback={boundaryFeedback}
        onStart={() => void runCommand(() => startSession(config))}
        onPause={() => void runCommand(pauseSession)}
        onResume={() => void runCommand(resumeSession)}
        onStop={() => void runCommand(stopSession)}
        onTranslateNow={() => void requestBoundary("user_button")}
      />

      <div className="application-layout">
        <AppNavigation
          activeSection={settingsOpen ? activeSettings : "live"}
          experience={experience}
          onSelect={(section) => {
            setActiveSettings(section);
            setSettingsOpen(section !== "live");
          }}
        />
        <div className={`workspace-layout${settingsOpen ? "" : " settings-collapsed"}`}>
        <ResponsiveSettingsPanel
          open={settingsOpen}
          section={activeSettings}
          onClose={() => {
            setSettingsOpen(false);
            setActiveSettings("live");
          }}
          returnFocusRef={settingsTriggerRef}
        >
          <section className="control-grid">
            <div className="panel controls-panel">
              <div className="panel-header">
                <h2>Session</h2>
                <span className={`panel-state ${sessionPanelHealthy ? "ok" : ""}`}>{readinessLabel}</span>
              </div>
              <div className="field-grid two">
                <SelectField
                  label="Source"
                  value={sourceLanguage}
                  onChange={(value) => setSourceLanguage(value as Language)}
                  options={sourceOptions}
                />
                <SelectField
                  label="Target"
                  value={targetLanguage}
                  onChange={(value) => setTargetLanguage(value as Language)}
                  options={targetLanguageOptions}
                />
              </div>

              {sourceLanguage === targetLanguage && sourceLanguage !== "auto" ? (
                <InlineWarning text="Source and target languages match." />
              ) : null}

              <div className="button-row">
                <button
                  className="primary"
                  onClick={() => runCommand(() => startSession(config))}
                  disabled={!canStart || busy}
                >
                  <Play size={18} /> Start
                </button>
                <button onClick={() => runCommand(pauseSession)} disabled={!canPause || busy}>
                  <Pause size={18} /> Pause
                </button>
                <button onClick={() => runCommand(resumeSession)} disabled={!canResume || busy}>
                  <Play size={18} /> Resume
                </button>
                <button onClick={() => runCommand(stopSession)} disabled={!canStop || busy}>
                  <Square size={18} /> Stop
                </button>
                <button
                  className="boundary-button"
                  onClick={() => requestBoundary("user_button")}
                  disabled={!canForceBoundary || busy}
                  title="Translate now"
                >
                  <Send size={18} /> Translate now
                </button>
              </div>

              {boundaryFeedback ? <div className="boundary-feedback">{boundaryFeedback}</div> : null}

              <label className="toggle-row">
                <input
                  type="checkbox"
                  checked={fallbackEnabled}
                  onChange={(event) => setFallbackEnabled(event.currentTarget.checked)}
                />
                <span>Fallback chain</span>
              </label>
            </div>

            <div className="panel devices-panel">
              <div className="panel-header">
                <h2>Audio routing</h2>
                <div className="panel-actions">
                  <label
                    className="auto-refresh-control"
                    title="Auto refresh audio devices every 5 seconds"
                  >
                    <input
                      type="checkbox"
                      checked={autoRefreshDevices}
                      onChange={(event) => setAutoRefreshDevices(event.currentTarget.checked)}
                    />
                    <RefreshCw size={14} />
                    <span>Auto</span>
                  </label>
                  <button
                    className="icon-button tight"
                    onClick={() => void refreshAudioDevices()}
                    disabled={refreshingDevices}
                    title="Refresh audio devices"
                    aria-label="Refresh audio devices"
                  >
                    <RefreshCw className={refreshingDevices ? "spin-icon" : ""} size={16} />
                  </button>
                </div>
              </div>
              <DeviceSelect
                icon={<Mic size={17} />}
                label="Meeting source"
                description={
                  isWindows
                    ? "Choose Teams audio (system output). Teams can stay on your normal speaker or headset."
                    : "Mac input captured for translation. Route Teams speaker audio here."
                }
                devices={devices.inputs}
                value={inputDeviceId}
                onChange={updateInputDevice}
                disabled={localMonitorActive}
              />
              <DeviceSelect
                icon={<Headphones size={17} />}
                label="Translated audio"
                description="Headphones, speakers, or a virtual device used as a Teams microphone."
                devices={devices.outputs}
                value={outputDeviceId}
                onChange={updateOutputDevice}
                disabled={localMonitorActive}
              />
              <SelectField
                label="Translated channel"
                value={translationOutputChannel}
                onChange={(value) => updateTranslationOutputChannel(value as AudioOutputChannel)}
                options={channelOptions}
                disabled={localMonitorActive}
              />
              {!isWindows ? (
                <>
                  <label className="toggle-row no-margin">
                    <input
                      type="checkbox"
                      checked={monitorOriginalAudio}
                      onChange={(event) => updateMonitorEnabled(event.currentTarget.checked)}
                    />
                    <span>Original audio monitor</span>
                  </label>
                  <DeviceSelect
                    icon={<Volume2 size={17} />}
                    label="Monitor output"
                    description="Optional original meeting audio playback."
                    devices={devices.outputs}
                    value={monitorOutputDeviceId}
                    onChange={updateMonitorOutputDevice}
                    disabled={!monitorOriginalAudio}
                  />
                  <SelectField
                    label="Original channel"
                    value={monitorOutputChannel}
                    onChange={(value) => updateMonitorOutputChannel(value as AudioOutputChannel)}
                    options={channelOptions}
                    disabled={!monitorOriginalAudio}
                  />
                </>
              ) : null}
              <div className="signal-card">
                <div>
                  <span>Input signal</span>
                  <strong>{inputSignalLabel}</strong>
                </div>
                <div
                  className="meter"
                  aria-label="Input signal peak"
                  aria-valuemax={100}
                  aria-valuemin={0}
                  aria-valuenow={inputSignalPercent}
                  role="progressbar"
                >
                  <div style={{ width: `${inputSignalPercent}%` }} />
                  <span style={{ width: `${inputSignalRmsPercent}%` }} />
                </div>
              </div>
              <AudioLineMonitor rows={audioLineRows} />
              <div className="meter-row">
                <Volume2 size={17} />
                <button
                  className={translationToneActive ? "small-button danger" : "small-button"}
                  onClick={() => testTone("translation", outputDeviceId, translationOutputChannel)}
                  disabled={
                    !translationToneActive && (!outputDeviceId || !canTestAudio)
                  }
                >
                  {translationToneActive ? "Stop translated" : "Test translated"}
                </button>
              </div>
              <div className="monitor-test-row">
                <Mic size={17} />
                <button
                  className={localMonitorActive ? "small-button danger" : "small-button"}
                  onClick={toggleLocalMonitor}
                  disabled={!canToggleLocalMonitor}
                >
                  {localMonitorActive ? "Stop mic monitor" : "Monitor mic to output"}
                </button>
                <span>
                  {localMonitorActive
                    ? "Live mic is routed to translated audio output."
                    : "No translation call; tests mic, device, and left/right routing."}
                </span>
              </div>
              {!isWindows ? <div className="button-row compact">
                <button
                  className={monitorToneActive ? "small-button danger" : "small-button"}
                  onClick={() => testTone("monitor", monitorOutputDeviceId, monitorOutputChannel)}
                  disabled={
                    !monitorToneActive &&
                    (!effectiveMonitorOriginalAudio || !monitorOutputDeviceId || !canTestAudio)
                  }
                >
                  {monitorToneActive ? "Stop original" : "Test original"}
                </button>
                {lastRefreshedAt ? (
                  <span className="refresh-note">
                    {refreshingDevices ? "Refreshing" : "Refreshed"} {formatClock(lastRefreshedAt)}
                    {autoRefreshDevices ? " | Auto 5s" : ""}
                  </span>
                ) : null}
              </div> : null}
              <RoutingSummary
                input={selectedInput}
                output={selectedOutput}
                outputChannel={translationOutputChannel}
                monitor={effectiveMonitorOriginalAudio ? selectedMonitorOutput : undefined}
                monitorChannel={monitorOutputChannel}
              />
              {routingWarnings.map((warning) => (
                <InlineWarning text={warning} key={warning} />
              ))}
            </div>

            <div className="panel key-panel">
              <div className="panel-header">
                <h2>Translation provider</h2>
              </div>
              {experience === "cloud" ? (
                <div className="segmented-control" aria-label="Translation provider">
                  {translationProviders
                    .filter((provider) => provider.value !== "local_whisper")
                    .map((provider) => (
                  <button
                    type="button"
                    className={translationProvider === provider.value ? "active" : ""}
                    title={provider.title}
                    key={provider.value}
                    onClick={() => selectTranslationProvider(provider.value)}
                  >
                    {provider.label}
                  </button>
                    ))}
                </div>
              ) : null}
              {translationProvider === "local_whisper" ? (
                <div className="local-provider-callout">
                  <strong>No cloud key required</strong>
                  <span>
                    Local mode transcribes the selected source with Whisper, translates with
                    TranslateGemma, and plays the selected target through your local voice output.
                  </span>
                  <button
                    className="small-button"
                    onClick={() => {
                      setActiveSettings("local_llm");
                      setSettingsOpen(true);
                    }}
                  >
                    <Settings2 size={14} /> Configure Local LLM
                  </button>
                </div>
              ) : (
                <>
                  <div className="key-row">
                    <input
                      type="password"
                      value={apiKeyDraft}
                      placeholder={
                        apiKeyStored
                          ? "Saved translation key. Paste only to replace."
                          : `${labelTranslationProvider(translationProvider)} API key`
                      }
                      onChange={(event) => setApiKeyDraft(event.currentTarget.value)}
                    />
                    <button onClick={saveKey} disabled={apiKeyDraft.trim().length === 0 || busy}>
                      <Save size={17} /> {apiKeyStored ? "Replace" : "Save"}
                    </button>
                  </div>
                  <div className={`key-status ${apiKeyStored ? "ok" : "warn"}`}>
                    <KeyRound size={14} />
                    <span>
                      {apiKeyStored
                        ? `Using ${labelApiKeySource(apiKeySource)} key ${apiKeyFingerprint || ""} for ${labelTranslationProvider(translationProvider)}`
                        : `No ${labelTranslationProvider(translationProvider)} key available`}
                    </span>
                  </div>
                  <div className="key-test-row">
                    <button
                      className="small-button"
                      onClick={testStoredKey}
                      disabled={!apiKeyStored || busy || testingKey}
                    >
                      <Check size={14} /> {testingKey ? "Testing" : "Test key"}
                    </button>
                    {keyTestMessage ? <span>{keyTestMessage}</span> : null}
                  </div>
                  <div className="setup-list">
                    {isWindows ? (
                      <>
                        <div>
                          <strong>1. Keep Teams</strong>
                          <span>Leave Teams on your normal speaker or headset.</span>
                        </div>
                        <div>
                          <strong>2. Select</strong>
                          <span>Choose Teams audio (system output) as the meeting source.</span>
                        </div>
                        <div>
                          <strong>3. Start</strong>
                          <span>Choose where translated audio should play, then start translation.</span>
                        </div>
                      </>
                    ) : (
                      <>
                        <div>
                          <strong>1. Route</strong>
                          <span>Set Teams speaker output to BlackHole 2ch or a multi-output device that includes it.</span>
                        </div>
                        <div>
                          <strong>2. Select</strong>
                          <span>Choose BlackHole as source and headphones as translation output.</span>
                        </div>
                        <div>
                          <strong>3. Split</strong>
                          <span>For left/right ears, use one headphone device with opposite channels.</span>
                        </div>
                        <div>
                          <strong>4. Teams</strong>
                          <span>To speak translated audio into Teams, route translated output to a virtual device and select it as the Teams microphone.</span>
                        </div>
                      </>
                    )}
                  </div>
                </>
              )}
            </div>

            <LocalLlmSettings
              draft={localConfigDraft}
              dirty={localConfigDirty}
              saving={localConfigSaving}
              testing={localConfigTesting}
              testResult={localConfigTest}
              voices={localVoices}
              voicesLoading={localVoicesLoading}
              previewing={localVoicePreviewing}
              whisperModels={whisperModels}
              selectedWhisperModelId={selectedWhisperModelId}
              whisperDownload={whisperDownload}
              whisperDownloading={whisperDownloading}
              vieneuRuntime={vieneuRuntime}
              vieneuProgress={vieneuProgress}
              vieneuBusy={vieneuBusy}
              previewDisabled={
                !outputDeviceId || status !== "idle" || testingTone !== null || localMonitorActive
              }
              onChange={(draft) => {
                if (draft.ttsProvider !== localConfigDraft.ttsProvider) {
                  setLocalVoices([]);
                }
                setLocalConfigDraft(draft);
                setLocalConfigDirty(true);
                setLocalConfigTest(null);
              }}
              onSave={() => void saveLocalConfig()}
              onTest={() => void testLocalConfig()}
              onPreview={() => void previewLocalVoice()}
              onRefreshVoices={() => void refreshLocalVoices()}
              onWhisperModelSelect={setSelectedWhisperModelId}
              onWhisperDownload={() => void downloadSelectedWhisperModel()}
              onVieNeuInstall={() => void installManagedVieNeu()}
              onVieNeuCancel={() => void cancelManagedVieNeu()}
              onVieNeuRestart={() => void restartManagedVieNeu()}
            />

            <div className="panel summary-config-panel">
              <div className="panel-header">
                <h2>Summary Agent</h2>
                <span className={`panel-state ${selectedProfile ? "ok" : ""}`}>
                  {selectedProfile ? "Configured" : "No profile"}
                </span>
              </div>

              <div className="field-grid">
                <SelectField
                  label="Profile"
                  value={selectedProfileId}
                  onChange={(value) => {
                    const profile = llmProfiles.find((item) => item.id === value);
                    setSelectedProfileId(value);
                    setSummaryConfig((current) => ({ ...current, providerProfileId: value }));
                    setProfileDraft(profile ? profileToDraft(profile) : emptyProfileDraft);
                    setProfileKeyDraft("");
                    setProfileTestMessage("");
                  }}
                  options={[
                    { value: "", label: "New profile" },
                    ...llmProfiles.map((profile) => ({
                      value: profile.id,
                      label: profile.name,
                    })),
                  ]}
                />

                <div className="segmented-control" aria-label="Provider kind">
                  {providerKinds.map((kind) => (
                    <button
                      type="button"
                      className={profileDraft.kind === kind.value ? "active" : ""}
                      title={kind.title}
                      key={kind.value}
                      onClick={() =>
                        setProfileDraft((current) => ({
                          ...current,
                          kind: kind.value,
                          baseUrl: defaultBaseUrlForKind(kind.value, current.baseUrl),
                        }))
                      }
                    >
                      {kind.label}
                    </button>
                  ))}
                </div>

                <div className="field-grid two">
                  <label className="field">
                    <span>Name</span>
                    <input
                      value={profileDraft.name}
                      onChange={(event) =>
                        updateProfileDraft({ name: event.currentTarget.value })
                      }
                    />
                  </label>
                  <label className="field">
                    <span>Model</span>
                    <input
                      value={profileDraft.model}
                      placeholder="gpt-4.1-mini or llama3.2"
                      onChange={(event) =>
                        updateProfileDraft({ model: event.currentTarget.value })
                      }
                    />
                  </label>
                </div>

                <label className="field">
                  <span>Base URL</span>
                  <input
                    value={profileDraft.baseUrl ?? ""}
                    placeholder={profileDraft.kind === "ollama" ? "http://localhost:11434/v1" : ""}
                    onChange={(event) =>
                      updateProfileDraft({ baseUrl: event.currentTarget.value })
                    }
                  />
                </label>

                <div className="field-grid two profile-tuning-grid">
                  <label className="field">
                    <span>Timeout (s)</span>
                    <input
                      type="number"
                      min="5"
                      max="300"
                      step="1"
                      value={profileNumberValue(profileDraft.timeoutSeconds)}
                      onChange={(event) =>
                        updateProfileDraft({
                          timeoutSeconds: parseProfileNumberInput(event.currentTarget.value, true),
                        })
                      }
                    />
                  </label>
                  <label className="field">
                    <span>Max tokens</span>
                    <input
                      type="number"
                      min="128"
                      max="16384"
                      step="1"
                      value={profileNumberValue(profileDraft.maxOutputTokens)}
                      onChange={(event) =>
                        updateProfileDraft({
                          maxOutputTokens: parseProfileNumberInput(
                            event.currentTarget.value,
                            true,
                          ),
                        })
                      }
                    />
                  </label>
                  <label className="field">
                    <span>Temperature</span>
                    <input
                      type="number"
                      min="0"
                      max="2"
                      step="0.1"
                      value={profileNumberValue(profileDraft.temperature)}
                      onChange={(event) =>
                        updateProfileDraft({
                          temperature: parseProfileNumberInput(event.currentTarget.value),
                        })
                      }
                    />
                  </label>
                  <label className="toggle-row profile-enabled-toggle no-margin">
                    <input
                      type="checkbox"
                      checked={profileDraft.enabled ?? true}
                      onChange={(event) =>
                        updateProfileDraft({ enabled: event.currentTarget.checked })
                      }
                    />
                    <span>Enabled</span>
                  </label>
                </div>

                <label className="field profile-key-field">
                  <span>LLM API key</span>
                  <input
                    type="password"
                    value={profileKeyDraft}
                    placeholder={
                      selectedProfile?.hasApiKey
                        ? `Saved LLM key ${selectedProfile.apiKeyFingerprint ?? ""}`
                        : profileDraft.kind === "ollama"
                          ? "Ollama API key, optional for local"
                          : "LLM provider API key"
                    }
                    onChange={(event) => setProfileKeyDraft(event.currentTarget.value)}
                  />
                  <small>Saved with the profile and stored separately from profile settings.</small>
                  <div className={`key-status ${selectedProfile?.hasApiKey ? "ok" : ""}`}>
                    {selectedProfile?.hasApiKey
                      ? `Saved LLM key ${selectedProfile.apiKeyFingerprint ?? ""}`
                      : profileDraft.kind === "ollama"
                        ? "No LLM key saved. Optional for local Ollama."
                        : "No LLM key saved for this profile."}
                  </div>
                </label>

                <div className="profile-actions">
                  <button
                    className="small-button"
                    onClick={saveSummaryProfile}
                    disabled={busy || profileDraftErrors.length > 0}
                  >
                    <Save size={14} /> Save profile
                  </button>
                  <button
                    className="small-button"
                    onClick={testSelectedProfile}
                    disabled={!selectedProfileId || busy || testingProfile}
                  >
                    <Check size={14} /> {testingProfile ? "Testing" : "Test profile"}
                  </button>
                  <button
                    className="small-button danger"
                    onClick={deleteSelectedProfile}
                    disabled={!selectedProfileId || busy}
                    title="Delete profile"
                  >
                    <Trash2 size={14} /> Delete
                  </button>
                  <button
                    className="small-button"
                    onClick={() => {
                      setSelectedProfileId("");
                      setProfileDraft(emptyProfileDraft);
                      setProfileKeyDraft("");
                      setProfileTestMessage("");
                    }}
                  >
                    <Settings2 size={14} /> New
                  </button>
                </div>
                {profileDraftErrors[0] ? <InlineWarning text={profileDraftErrors[0]} /> : null}
                {profileTestMessage ? <div className="key-test-row">{profileTestMessage}</div> : null}
              </div>

              <div className="summary-options">
                <div className="field-grid two">
                  <SelectField
                    label="Transcript"
                    value={summaryConfig.transcriptScope}
                    onChange={(value) =>
                      setSummaryConfig((current) => ({
                        ...current,
                        transcriptScope: value as TranscriptScope,
                      }))
                    }
                    options={transcriptScopeOptions}
                  />
                  <label className="field">
                    <span>Language</span>
                    <input
                      value={summaryConfig.outputLanguage}
                      onChange={(event) => {
                        const outputLanguage = event.currentTarget.value;
                        setSummaryConfig((current) => ({
                          ...current,
                          outputLanguage,
                        }));
                      }}
                    />
                  </label>
                </div>
                <div className="summary-prompt-control">
                  <SelectField
                    label="Summary style"
                    value={summaryConfig.promptPreset}
                    onChange={(value) =>
                      setSummaryConfig((current) =>
                        selectMeetingSummaryPromptPreset(
                          current,
                          value as MeetingSummaryPromptPreset,
                        ),
                      )
                    }
                    options={summaryPromptPresetOptions}
                  />
                  <p className="summary-prompt-description">{summaryPromptDescription}</p>
                  {summaryConfig.promptPreset === "custom" ? (
                    <div className="summary-custom-prompt">
                      <label htmlFor="summary-custom-system-prompt">Custom system prompt</label>
                      <textarea
                        id="summary-custom-system-prompt"
                        value={summaryConfig.customSystemPrompt}
                        placeholder="Describe the tone, emphasis, or organization you want."
                        aria-invalid={Boolean(summaryPromptValidation)}
                        aria-describedby="summary-custom-prompt-meta"
                        onChange={(event) => {
                          const customSystemPrompt = event.currentTarget.value;
                          setSummaryConfig((current) => ({
                            ...current,
                            customSystemPrompt,
                          }));
                        }}
                      />
                      <div id="summary-custom-prompt-meta" className="summary-prompt-meta">
                        <div
                          className="summary-prompt-validation"
                          role={summaryPromptValidation ? "alert" : undefined}
                        >
                          {summaryPromptValidation ?? " "}
                        </div>
                        <div
                          className={`summary-prompt-count${summaryPromptValidation ? " invalid" : ""}`}
                        >
                          {summaryPromptLength.toLocaleString()}/
                          {MEETING_SUMMARY_CUSTOM_PROMPT_MAX_CHARS.toLocaleString()}
                        </div>
                      </div>
                    </div>
                  ) : null}
                </div>
                <div className="section-toggles">
                  {(
                    [
                      ["summary", "Summary"],
                      ["decisions", "Decisions"],
                      ["actionItems", "Actions"],
                      ["blockers", "Blockers"],
                      ["importantPoints", "Points"],
                    ] as const
                  ).map(([key, label]) => (
                    <label key={key}>
                      <input
                        type="checkbox"
                        checked={summaryConfig.sections[key]}
                        onChange={(event) => {
                          const checked = event.currentTarget.checked;
                          setSummaryConfig((current) => ({
                            ...current,
                            sections: {
                              ...current.sections,
                              [key]: checked,
                            },
                          }));
                        }}
                      />
                      <span>{label}</span>
                    </label>
                  ))}
                </div>
                <button className="primary run-summary-button" onClick={runSummary} disabled={!canRunSummary}>
                  <Bot size={17} /> {summaryRunning ? "Running" : "Run summary"}
                </button>
                {summaryStatus ? <div className="summary-status">{summaryStatus}</div> : null}
              </div>
            </div>
          </section>
        </ResponsiveSettingsPanel>

        <section className="translation-column" aria-label="Live translation">
          {error ? (
            <section className="error-bar">
              <AlertTriangle size={18} />
              <span>{error.message}</span>
            </section>
          ) : null}

          {meetingNotes ? (
            <section className="notes-panel">
              <div className="panel-header">
                <h2>Meeting Notes</h2>
                <span className="panel-state ok">{meetingNotes.model}</span>
              </div>
              {meetingNotes.summary ? <p className="notes-summary">{meetingNotes.summary}</p> : null}
              <NotesList title="Decisions" values={meetingNotes.decisions} />
              <ActionItemsList items={meetingNotes.actionItems} />
              <NotesList title="Blockers" values={meetingNotes.blockers} />
              <NotesList title="Important points" values={meetingNotes.importantPoints} />
            </section>
          ) : null}

          <section className="conversation-panel">
            {experience === "local" ? (
              <LocalPipelineRail stage={localPipelineStage} sessionStatus={status} />
            ) : null}
            <LiveStatusRail
              sourceSignalState={sourceSignalState}
              sourceLevelPercent={inputSignalPercent}
              sessionStatus={status}
              translationActivity={translationActivity}
              playbackLevelPercent={localMonitorActive ? 0 : translationOutLevel}
            />
            <div
              className="conversation-feed"
              onScroll={handleConversationScroll}
              ref={conversationFeedRef}
            >
              {conversationItems.length === 0 ? (
                <ConversationEmptyState
                  status={status}
                  sourceSignalState={sourceSignalState}
                  hasInput={Boolean(inputDeviceId)}
                  hasOutput={
                    Boolean(outputDeviceId)
                  }
                  onOpenAudioSettings={() => {
                    setActiveSettings("audio");
                    setSettingsOpen(true);
                  }}
                />
              ) : (
                conversationItems.map((item) => (
                  <UtteranceCard item={item} key={item.id} />
                ))
              )}
              <div ref={transcriptEndRef} />
            </div>
            {hasNewTranslations ? (
              <button
                className="new-translation-button"
                onClick={jumpToLatestTranslation}
                type="button"
              >
                <FileText size={15} /> New translation
              </button>
            ) : null}
          </section>
        </section>
        </div>
      </div>
    </main>
  );
}

export function TransparentOverlayWindow() {
  const [status, setStatus] = useState<OverlayStatus | null>(null);
  const [translation, setTranslation] = useState<OverlayTranslationUpdate | null>(null);
  const [opacity, setOpacity] = useState(0.72);
  const [copied, setCopied] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [statusMessage, setStatusMessage] = useState("Starting overlay");
  const config: OverlayConfig = status?.config ?? {
    sourceLanguage: "auto",
    targetLanguage: "vi",
    captureIntervalMs: 800,
    minimumConfidence: 0.45,
    opacity,
    geminiModel: "models/gemini-2.5-flash",
  };

  useEffect(() => {
    document.documentElement.classList.add("overlay-html");
    document.body.classList.add("overlay-body");
    return () => {
      document.documentElement.classList.remove("overlay-html");
      document.body.classList.remove("overlay-body");
    };
  }, []);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let disposed = false;
    const appWindow = getCurrentWindow();

    async function reportGeometry() {
      try {
        const [position, size, scaleFactor, monitor] = await Promise.all([
          appWindow.innerPosition(),
          appWindow.innerSize(),
          appWindow.scaleFactor(),
          currentMonitor(),
        ]);
        if (disposed) {
          return;
        }
        await updateOverlayGeometry({
          displayId: monitor?.name ?? undefined,
          x: position.x,
          y: position.y,
          width: size.width,
          height: size.height,
          scaleFactor,
          updatedAtMs: Date.now(),
        });
      } catch {
        setStatusMessage("Waiting for window geometry");
      }
    }

    const unlisten = Promise.all([
      listen<OverlayStatus>("overlay-status-update", (event) => {
        setStatus(event.payload);
        setOpacity(event.payload.config.opacity);
        setStatusMessage(event.payload.message);
      }),
      listen<OverlayTranslationUpdate>("overlay-translation-update", (event) => {
        setTranslation(event.payload);
        setStatusMessage(event.payload.message);
      }),
      appWindow.onMoved(() => void reportGeometry()),
      appWindow.onResized(() => void reportGeometry()),
    ]);

    void overlayStatus()
      .then((payload) => {
        setStatus(payload);
        setOpacity(payload.config.opacity);
        setStatusMessage(payload.message);
      })
      .catch(() => setStatusMessage("Overlay status unavailable"));
    void reportGeometry();
    const interval = window.setInterval(reportGeometry, 1200);

    return () => {
      disposed = true;
      window.clearInterval(interval);
      void unlisten.then((callbacks) => callbacks.forEach((callback) => callback()));
    };
  }, []);

  async function togglePaused() {
    const paused = !(status?.isPaused ?? false);
    setStatus((current) => (current ? { ...current, isPaused: paused } : current));
    await setOverlayPaused(paused);
  }

  async function copyTranslation() {
    if (!translation?.translatedText) {
      return;
    }
    await navigator.clipboard.writeText(translation.translatedText);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  }

  function updateTransparentConfig(nextConfig: OverlayConfig) {
    setStatus((current) => (current ? { ...current, config: nextConfig } : current));
    setOpacity(nextConfig.opacity);
    if (!isTauri()) {
      return;
    }
    void updateOverlayConfig(nextConfig).catch((cause) =>
      setStatusMessage(normalizeError(cause).message),
    );
  }

  const statusKind = status?.status ?? "idle";
  const translatedText = translation?.translatedText.trim();
  const sourceText = translation?.sourceText.trim();

  return (
    <main
      className="overlay-window-root look-through-window-root"
      aria-label="Look Through"
      style={{ "--overlay-opacity": opacity } as React.CSSProperties}
    >
      <header
        className="overlay-titlebar"
        onMouseDown={beginOverlayDrag}
      >
        <div>
          <Move size={15} />
          <strong>Look Through</strong>
        </div>
        <div className="overlay-window-actions" onMouseDown={beginOverlayActionsDrag}>
          <button
            className="icon-button tight"
            onClick={() => setSettingsOpen((current) => !current)}
            title={settingsOpen ? "Hide settings" : "Show settings"}
            aria-label={settingsOpen ? "Hide settings" : "Show settings"}
          >
            <Settings2 size={14} />
          </button>
          <button
            className="icon-button tight"
            onClick={() => void closeOverlayWindow()}
            title="Close overlay"
            aria-label="Close overlay"
          >
            <X size={14} />
          </button>
        </div>
      </header>

      <section className={`overlay-status-strip look-through-status-strip overlay-${statusKind}`}>
        <span className={`status-dot status-${statusKind}`} />
        <span>{overlayStatusLabel(statusKind)}</span>
        <small>{statusMessage}</small>
        <button className="look-through-live-button" onClick={() => void togglePaused()}>
          {status?.isPaused ? <Play size={15} /> : <Pause size={15} />}
          {status?.isPaused ? "Resume" : "Pause"}
        </button>
      </section>

      {settingsOpen ? (
        <section className="look-through-settings" aria-label="Look Through settings">
          <label>
            <span>Opacity</span>
            <input
              type="range"
              min="0.35"
              max="0.92"
              step="0.01"
              value={opacity}
              onChange={(event) => {
                const nextOpacity = Number(event.currentTarget.value);
                updateTransparentConfig({ ...config, opacity: nextOpacity });
              }}
            />
          </label>
        </section>
      ) : null}

      <section className="look-through-workspace">
        <article className="look-through-panel look-through-source-panel">
          <header>
            <strong>Detected screen</strong>
            <small>{sourceText ? `${sourceText.length} characters` : "Live OCR"}</small>
          </header>
          <div className="look-through-panel-body">
            {sourceText ? (
              <p>{sourceText}</p>
            ) : (
              <div className="look-through-panel-empty">
                <ScanText size={22} />
                <strong>
                  {status?.isPaused ? "Realtime detection paused." : "Watching this region."}
                </strong>
                <span>Move or resize the window to detect visible text.</span>
              </div>
            )}
          </div>
        </article>

        <article className="look-through-panel look-through-result-panel" aria-live="polite">
          <header>
            <strong>Translation</strong>
            <button
              className="icon-button tight"
              onClick={copyTranslation}
              disabled={!translatedText}
              title="Copy translation"
              aria-label="Copy translation"
            >
              <Copy size={14} />
            </button>
          </header>
          <div className="look-through-panel-body look-through-result-body">
            {translatedText ? (
              <p>{translatedText}</p>
            ) : statusKind === "permission_needed" ? (
              <div className="look-through-panel-empty">
                <AlertTriangle size={22} />
                <strong>{statusMessage}</strong>
                <button onClick={() => void openScreenRecordingSettings()}>
                  <Settings2 size={15} /> Open Privacy Settings
                </button>
              </div>
            ) : statusKind === "error" ? (
              <div className="look-through-panel-empty">
                <AlertTriangle size={22} />
                <strong>{statusMessage}</strong>
              </div>
            ) : statusKind === "no_text" ? (
              <div className="look-through-panel-empty">
                <ScanText size={22} />
                <strong>No readable text detected.</strong>
                <span>Realtime scanning will continue automatically.</span>
              </div>
            ) : status?.isPaused ? (
              <div className="look-through-panel-empty">
                <Pause size={22} />
                <strong>Realtime detection paused.</strong>
              </div>
            ) : (
              <div className="look-through-panel-empty">
                <ScanText size={22} />
                <strong>
                  {statusKind === "translating"
                    ? "Translating detected text"
                    : "Detecting visible text"}
                </strong>
                <span>Updates appear automatically.</span>
              </div>
            )}
          </div>
        </article>
      </section>

      <footer className="look-through-meta-bar">
        <span>
          {config.sourceLanguage.toUpperCase()} to {config.targetLanguage.toUpperCase()}
        </span>
        <span>
          {copied
            ? "Translation copied"
            : status?.isPaused
              ? "Realtime paused"
              : "Realtime detection"}
        </span>
      </footer>
    </main>
  );
}

export function LookHelpOverlayWindow() {
  const [status, setStatus] = useState<LookHelpStatus | null>(null);
  const [answer, setAnswer] = useState<LookHelpUpdate | null>(null);
  const [profiles, setProfiles] = useState<LlmProviderProfile[]>([]);
  const [opacity, setOpacity] = useState(0.78);
  const [localConfig, setLocalConfig] = useState<LookHelpConfig>(() => defaultLookHelpConfig());
  const [copied, setCopied] = useState(false);
  const [capturePending, setCapturePending] = useState(false);
  const [statusMessage, setStatusMessage] = useState("Starting Look & Help");
  const config = status?.config ?? localConfig;

  useEffect(() => {
    document.documentElement.classList.add("overlay-html");
    document.body.classList.add("overlay-body");
    return () => {
      document.documentElement.classList.remove("overlay-html");
      document.body.classList.remove("overlay-body");
    };
  }, []);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let disposed = false;
    const appWindow = getCurrentWindow();

    async function reportGeometry() {
      try {
        const [position, size, scaleFactor, monitor] = await Promise.all([
          appWindow.innerPosition(),
          appWindow.innerSize(),
          appWindow.scaleFactor(),
          currentMonitor(),
        ]);
        if (disposed) {
          return;
        }
        await updateLookHelpGeometry({
          displayId: monitor?.name ?? undefined,
          x: position.x,
          y: position.y,
          width: size.width,
          height: size.height,
          scaleFactor,
          updatedAtMs: Date.now(),
        });
      } catch {
        setStatusMessage("Waiting for window geometry");
      }
    }

    const unlisten = Promise.all([
      listen<LookHelpStatus>("look-help-status-update", (event) => {
        setStatus(event.payload);
        setOpacity(event.payload.config.opacity);
        setStatusMessage(event.payload.message);
      }),
      listen<LookHelpUpdate>("look-help-answer-update", (event) => {
        setAnswer(event.payload);
        setStatusMessage(event.payload.message);
      }),
      appWindow.onMoved(() => void reportGeometry()),
      appWindow.onResized(() => void reportGeometry()),
    ]);

    void lookHelpStatus()
      .then((payload) => {
        setStatus(payload);
        setOpacity(payload.config.opacity);
        setStatusMessage(payload.message);
      })
      .catch(() => setStatusMessage("Look & Help status unavailable"));
    void listLlmProfiles().then(setProfiles).catch(() => setProfiles([]));
    void reportGeometry();
    const interval = window.setInterval(reportGeometry, 1200);

    return () => {
      disposed = true;
      window.clearInterval(interval);
      void unlisten.then((callbacks) => callbacks.forEach((callback) => callback()));
    };
  }, []);

  function updateHelperConfig(nextConfig: LookHelpConfig) {
    setLocalConfig(nextConfig);
    setStatus((current) => (current ? { ...current, config: nextConfig } : current));
    setOpacity(nextConfig.opacity);
    if (!isTauri()) {
      return;
    }
    void updateLookHelpConfig(nextConfig).catch((cause) =>
      setStatusMessage(normalizeError(cause).message),
    );
  }

  async function captureScreen() {
    if (capturePending) {
      return;
    }
    setCapturePending(true);
    setAnswer(null);
    try {
      await captureLookHelp();
    } catch (cause) {
      setStatusMessage(normalizeError(cause).message);
    } finally {
      setCapturePending(false);
    }
  }

  async function copyAnswer() {
    if (!answer?.answerText) {
      return;
    }
    await navigator.clipboard.writeText(answer.answerText);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  }

  const statusKind = status?.status ?? "idle";
  const answerText = answer?.answerText.trim();
  const sourceText = answer?.sourceText.trim();
  const selectedProfile = profiles.find((profile) => profile.id === config.providerProfileId);
  const isWorking =
    capturePending || statusKind === "scanning" || statusKind === "thinking";
  const captureLabel =
    statusKind === "thinking" ? "Asking LLM" : isWorking ? "Capturing" : "Capture";

  return (
    <main
      className="overlay-window-root look-help-window-root"
      aria-label="Look & Help"
      style={{ "--overlay-opacity": opacity } as React.CSSProperties}
    >
      <header
        className="overlay-titlebar look-help-titlebar"
        onMouseDown={beginOverlayDrag}
      >
        <div>
          <Bot size={15} />
          <strong>Look & Help</strong>
        </div>
        <div className="overlay-window-actions" onMouseDown={beginOverlayActionsDrag}>
          <button
            className="icon-button tight"
            onClick={() =>
              updateHelperConfig({
                ...config,
                promptPanelVisible: !config.promptPanelVisible,
              })
            }
            title={config.promptPanelVisible ? "Hide settings" : "Show settings"}
            aria-label={config.promptPanelVisible ? "Hide settings" : "Show settings"}
          >
            <Settings2 size={14} />
          </button>
          <button
            className="icon-button tight"
            onClick={() => void closeLookHelpWindow()}
            title="Close overlay"
            aria-label="Close overlay"
          >
            <X size={14} />
          </button>
        </div>
      </header>

      <section className={`overlay-status-strip look-help-status-strip overlay-${statusKind}`}>
        <span className={`status-dot status-${statusKind}`} />
        <span>{overlayStatusLabel(statusKind)}</span>
        <small>{statusMessage}</small>
        <button
          className="look-help-capture-button"
          onClick={() => void captureScreen()}
          disabled={isWorking}
        >
          <ScanText size={15} />
          {captureLabel}
        </button>
      </section>

      {config.promptPanelVisible ? (
        <section className="look-help-settings" aria-label="Look & Help settings">
          <label>
            <span>Profile</span>
            <select
              value={config.providerProfileId}
              onChange={(event) =>
                updateHelperConfig({ ...config, providerProfileId: event.currentTarget.value })
              }
            >
              <option value="">Select profile</option>
              {profiles.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Opacity</span>
            <input
              type="range"
              min="0.35"
              max="0.94"
              step="0.01"
              value={opacity}
              onChange={(event) => {
                const nextOpacity = Number(event.currentTarget.value);
                updateHelperConfig({ ...config, opacity: nextOpacity });
              }}
            />
          </label>
        </section>
      ) : null}

      <section className="look-help-workspace">
        <article className="look-help-panel look-help-source-panel">
          <header>
            <strong>Captured screen</strong>
            <small>{sourceText ? `${sourceText.length} characters` : "OCR text"}</small>
          </header>
          <div className="look-help-panel-body">
            {sourceText ? (
              <p>{sourceText}</p>
            ) : (
              <div className="look-help-panel-empty">
                <ScanText size={22} />
                <strong>Position the window over the content you need.</strong>
                <span>Text is read only when you press Capture.</span>
              </div>
            )}
          </div>
        </article>

        <article className="look-help-panel look-help-request-panel">
          <header>
            <strong>Request</strong>
            <small>Sent with captured text</small>
          </header>
          <textarea
            aria-label="Request for the LLM"
            value={config.systemPrompt}
            onChange={(event) =>
              updateHelperConfig({ ...config, systemPrompt: event.currentTarget.value })
            }
          />
        </article>

        <article className="look-help-panel look-help-result-panel" aria-live="polite">
          <header>
            <strong>LLM result</strong>
            <button
              className="icon-button tight"
              onClick={copyAnswer}
              disabled={!answerText}
              title="Copy result"
              aria-label="Copy result"
            >
              <Copy size={14} />
            </button>
          </header>
          <div className="look-help-panel-body look-help-result-body">
            {answerText ? (
              <p>{answerText}</p>
            ) : statusKind === "permission_needed" ? (
              <div className="look-help-panel-empty">
                <AlertTriangle size={22} />
                <strong>{statusMessage}</strong>
                <button onClick={() => void openScreenRecordingSettings()}>
                  <Settings2 size={15} /> Open Privacy Settings
                </button>
              </div>
            ) : statusKind === "error" ? (
              <div className="look-help-panel-empty">
                <AlertTriangle size={22} />
                <strong>{statusMessage}</strong>
              </div>
            ) : statusKind === "no_text" ? (
              <div className="look-help-panel-empty">
                <ScanText size={22} />
                <strong>No readable text found.</strong>
                <span>Adjust the capture area and try again.</span>
              </div>
            ) : isWorking ? (
              <div className="look-help-panel-empty">
                <Bot size={22} />
                <strong>
                  {statusKind === "thinking" ? "Preparing the answer" : "Reading the screen"}
                </strong>
              </div>
            ) : (
              <div className="look-help-panel-empty">
                <Bot size={22} />
                <strong>No answer yet.</strong>
                <span>Capture the screen when the region is aligned.</span>
              </div>
            )}
          </div>
        </article>
      </section>

      <footer className="look-help-meta-bar">
        <span>{selectedProfile?.name ?? "No LLM profile selected"}</span>
        <span>{copied ? "Result copied" : "Manual capture only"}</span>
      </footer>
    </main>
  );
}

function LocalPipelineRail({
  stage,
  sessionStatus,
}: {
  stage: LocalPipelineStage;
  sessionStatus: SessionStatus;
}) {
  const stages: Array<{ value: LocalPipelineStage; label: string; icon: React.ReactNode }> = [
    { value: "listening", label: "Listening", icon: <Mic size={16} /> },
    { value: "transcribing", label: "Whisper", icon: <FileText size={16} /> },
    { value: "translating", label: "Gemma", icon: <Bot size={16} /> },
    { value: "synthesizing", label: "Voice", icon: <Activity size={16} /> },
    { value: "speaking", label: "Speaking", icon: <Volume2 size={16} /> },
  ];
  return (
    <div className="local-pipeline-rail" aria-label="Local translation pipeline">
      <span className="visually-hidden" role="status" aria-live="polite">
        Local pipeline: {stages.find((item) => item.value === stage)?.label ?? "Listening"}
      </span>
      {stages.map((item) => {
        const active = sessionStatus !== "idle" && stage === item.value;
        return (
          <div
            className={`local-pipeline-step${active ? " active" : ""}`}
            aria-current={active ? "step" : undefined}
            key={item.value}
          >
            <span aria-hidden="true">{item.icon}</span>
            <strong>{item.label}</strong>
          </div>
        );
      })}
    </div>
  );
}

function LiveStatusRail({
  sourceSignalState,
  sourceLevelPercent,
  sessionStatus,
  translationActivity,
  playbackLevelPercent,
}: {
  sourceSignalState: SourceSignalState;
  sourceLevelPercent: number;
  sessionStatus: SessionStatus;
  translationActivity: TranslationActivityState;
  playbackLevelPercent: number;
}) {
  const source = sourceSignalMeta(sourceSignalState);
  const activity = translationActivityMeta(translationActivity);
  const playbackActive = playbackLevelPercent > 3;
  return (
    <div className="live-status-rail" aria-label="Live translation status">
      <RailChip
        icon={<Mic size={17} />}
        label="Source"
        value={source.label}
        tone={source.tone}
        meterValue={sourceSignalState === "receiving" ? sourceLevelPercent : undefined}
      />
      <RailChip
        icon={<Activity size={17} />}
        label="Session"
        value={labelStatus(sessionStatus)}
        tone={sessionTone(sessionStatus)}
      />
      <RailChip
        icon={translationActivity === "translating" ? <RefreshCw size={17} /> : <Bot size={17} />}
        label="Translation"
        value={activity.label}
        tone={activity.tone}
      />
      <RailChip
        icon={<Volume2 size={17} />}
        label="Playback"
        value={playbackActive ? "Playing translated" : "Output ready"}
        tone={playbackActive ? "ok" : "neutral"}
        meterValue={playbackLevelPercent}
      />
    </div>
  );
}

function RailChip({
  icon,
  label,
  value,
  tone,
  meterValue,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  tone: "ok" | "info" | "warn" | "danger" | "neutral";
  meterValue?: number;
}) {
  const normalized = Math.max(0, Math.min(100, Math.round(meterValue ?? 0)));
  return (
    <div className={`rail-chip ${tone}`}>
      <span className="rail-icon" aria-hidden="true">
        {icon}
      </span>
      <div className="rail-copy">
        <span>{label}</span>
        <strong>{value}</strong>
      </div>
      {meterValue === undefined ? null : (
        <div
          className="rail-meter"
          aria-label={`${label} level`}
          aria-valuemax={100}
          aria-valuemin={0}
          aria-valuenow={normalized}
          role="progressbar"
        >
          <i style={{ width: `${normalized}%` }} />
        </div>
      )}
    </div>
  );
}

function ConversationEmptyState({
  status,
  sourceSignalState,
  hasInput,
  hasOutput,
  onOpenAudioSettings,
}: {
  status: SessionStatus;
  sourceSignalState: SourceSignalState;
  hasInput: boolean;
  hasOutput: boolean;
  onOpenAudioSettings: () => void;
}) {
  const copy = emptyConversationCopy(status, sourceSignalState, hasInput, hasOutput);
  const needsSetup = !hasInput || !hasOutput;
  return (
    <div className="empty-state conversation-empty">
      <FileText size={28} />
      <strong>{copy.title}</strong>
      <p>{copy.body}</p>
      {needsSetup ? (
        <button className="empty-state-action" onClick={onOpenAudioSettings} type="button">
          <Settings2 size={16} /> Open audio settings
        </button>
      ) : null}
    </div>
  );
}

function UtteranceCard({ item }: { item: ConversationDisplayItem }) {
  const sentencePairs =
    item.sentencePairs.length > 0
      ? item.sentencePairs
      : [
          {
            sourceText: item.sourceText,
            translatedText: item.translatedText,
          },
        ];

  return (
    <article
      aria-live={item.status === "final" ? "polite" : "off"}
      className={`utterance-card ${item.status}`}
    >
      <header className="utterance-meta">
        <span className="speaker-chip">
          <Mic size={14} />
          {item.speakerDisplayLabel}
        </span>
        <span>{formatClock(item.timestampMs)}</span>
        <span className={`utterance-state state-${item.status}`}>
          {labelTranscriptStatus(item)}
        </span>
        {item.latencyMs ? <span>{item.latencyMs} ms</span> : null}
        {item.speakerConfidence ? (
          <span>{Math.round(item.speakerConfidence * 100)}% speaker confidence</span>
        ) : null}
      </header>
      <div className="sentence-pair-list">
        <div className="sentence-pair-heading" aria-hidden="true">
          <span>Original</span>
          <span>Translation</span>
        </div>
        {sentencePairs.map((pair, index) => (
          <div className="sentence-pair-row" key={`${item.id}-${index}`}>
            <p className="utterance-source">
              {pair.sourceText ||
                (item.status === "partial" ? "Listening..." : "No source text")}
            </p>
            <div
              className={`translation-line ${
                item.hasPendingTranslation ? "pending" : item.status === "error" ? "error" : ""
              }`}
            >
              {item.status === "error" ? (
                <p>
                  <AlertTriangle size={15} />
                  {item.errorMessage || pair.translatedText || "Translation failed for this utterance."}
                </p>
              ) : pair.translatedText ? (
                <p>{pair.translatedText}</p>
              ) : (
                <p className="translation-placeholder">Translating</p>
              )}
            </div>
          </div>
        ))}
      </div>
    </article>
  );
}

function NotesList({ title, values }: { title: string; values: string[] }) {
  if (values.length === 0) {
    return null;
  }
  return (
    <div className="notes-list">
      <strong>{title}</strong>
      <ul>
        {values.map((value) => (
          <li key={value}>{value}</li>
        ))}
      </ul>
    </div>
  );
}

function ActionItemsList({ items }: { items: MeetingSummaryResult["actionItems"] }) {
  if (items.length === 0) {
    return null;
  }
  return (
    <div className="notes-list">
      <strong>Action items</strong>
      <ul>
        {items.map((item) => (
          <li key={`${item.text}-${item.owner ?? ""}`}>
            {item.text}
            {item.owner ? <span> Owner: {item.owner}.</span> : null}
            {item.dueDate ? <span> Due: {item.dueDate}.</span> : null}
          </li>
        ))}
      </ul>
    </div>
  );
}

function SelectField({
  label,
  value,
  onChange,
  options,
  disabled = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
  disabled?: boolean;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <select
        value={value}
        onChange={(event) => onChange(event.currentTarget.value)}
        disabled={disabled}
      >
        {options.map((option) => (
          <option value={option.value} key={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function DeviceSelect({
  icon,
  label,
  description,
  devices,
  value,
  onChange,
  disabled = false,
}: {
  icon: React.ReactNode;
  label: string;
  description?: string;
  devices: AudioDeviceInfo[];
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}) {
  return (
    <label className="device-field">
      <span>
        {icon}
        {label}
      </span>
      {description ? <small>{description}</small> : null}
      <select
        value={value}
        onChange={(event) => onChange(event.currentTarget.value)}
        disabled={disabled}
      >
        <option value="">No device</option>
        {devices.map((device) => (
          <option value={device.id} key={device.id}>
            {formatDeviceOption(device)}
            {device.isDefault ? " (default)" : ""}
          </option>
        ))}
      </select>
    </label>
  );
}

function RoutingSummary({
  input,
  output,
  outputChannel,
  monitor,
  monitorChannel,
}: {
  input?: AudioDeviceInfo;
  output?: AudioDeviceInfo;
  outputChannel: AudioOutputChannel;
  monitor?: AudioDeviceInfo;
  monitorChannel: AudioOutputChannel;
}) {
  const splitActive =
    sameDeviceId(output, monitor) &&
    ((outputChannel === "left" && monitorChannel === "right") ||
      (outputChannel === "right" && monitorChannel === "left"));

  return (
    <div className="routing-summary">
      <div>
        <strong>Meeting</strong>
        <span>{input?.name ?? "No input"}</span>
      </div>
      <div>
        <strong>Translation</strong>
        <span>{formatRoute(output, outputChannel)}</span>
      </div>
      <div>
        <strong>Original</strong>
        <span>{monitor ? formatRoute(monitor, monitorChannel) : "Off"}</span>
      </div>
      <div>
        <strong>Mode</strong>
        <span>{splitActive ? "Split ears active" : "Mono route"}</span>
      </div>
    </div>
  );
}

function AudioLineMonitor({ rows }: { rows: AudioLineRow[] }) {
  return (
    <div className="line-monitor" aria-label="Audio line monitor">
      {rows.map((row) => (
        <div className={`line-row ${row.disabled ? "disabled" : ""}`} key={row.id}>
          <div className="line-meta">
            <strong>{row.label}</strong>
            <span>{row.detail}</span>
          </div>
          <span className={`line-state state-${row.state}`}>{row.state}</span>
          <ChannelLane side="L" value={row.leftLevel} />
          <ChannelLane side="R" value={row.rightLevel} />
        </div>
      ))}
    </div>
  );
}

function ChannelLane({ side, value }: { side: "L" | "R"; value: number }) {
  const normalized = Math.max(0, Math.min(100, Math.round(value)));
  return (
    <div className={`channel-lane ${normalized > 3 ? "active" : ""}`}>
      <span>{side}</span>
      <div>
        <i style={{ width: `${normalized}%` }} />
      </div>
    </div>
  );
}

function InlineWarning({ text }: { text: string }) {
  return (
    <div className="inline-warning">
      <AlertTriangle size={15} />
      <span>{text}</span>
    </div>
  );
}

function selectDefaultDevice(devices: AudioDeviceInfo[]) {
  return devices.find((device) => device.isDefault)?.id ?? devices[0]?.id ?? "";
}

function resolveStoredDevice(devices: AudioDeviceInfo[], storedId?: string) {
  if (!storedId) {
    return selectDefaultDevice(devices);
  }

  const exact = devices.find((device) => device.id === storedId);
  if (exact) {
    return exact.id;
  }

  const storedName = storedId.split(":").slice(2).join(":");
  const sameName = devices.find(
    (device) => normalizeDeviceName(device.name) === normalizeDeviceName(storedName),
  );
  return sameName?.id ?? selectDefaultDevice(devices);
}

function resolveRoutingProfile(
  deviceList: AudioDevices,
  storedRouting?: RoutingProfile | null,
): RoutingProfile {
  return {
    inputDeviceId: resolveStoredDevice(deviceList.inputs, storedRouting?.inputDeviceId),
    outputDeviceId: resolveStoredDevice(deviceList.outputs, storedRouting?.outputDeviceId),
    translationOutputChannel: storedRouting?.translationOutputChannel ?? "all",
    monitorOutputDeviceId: resolveStoredDevice(
      deviceList.outputs,
      storedRouting?.monitorOutputDeviceId,
    ),
    monitorOutputChannel: storedRouting?.monitorOutputChannel ?? "all",
    monitorOriginalAudio: storedRouting?.monitorOriginalAudio ?? false,
  };
}

function labelStatus(status: SessionStatus) {
  return status.replace(/_/g, " ");
}

function overlayStatusLabel(status: OverlayStatus["status"]) {
  return status.replace(/_/g, " ");
}

function labelTranscriptStatus(item: ConversationDisplayItem) {
  if (item.status === "error") {
    return "Needs attention";
  }
  if (item.hasPendingTranslation) {
    return "Translating";
  }
  return item.status;
}

function sourceSignalMeta(state: SourceSignalState): {
  label: string;
  tone: "ok" | "info" | "warn" | "danger" | "neutral";
} {
  switch (state) {
    case "receiving":
      return { label: "Receiving audio", tone: "ok" };
    case "silent":
      return { label: "Source silent", tone: "info" };
    case "stale":
      return { label: "No recent audio", tone: "warn" };
    case "error":
      return { label: "Capture error", tone: "danger" };
    case "waiting":
      return { label: "Waiting", tone: "neutral" };
  }
}

function translationActivityMeta(state: TranslationActivityState): {
  label: string;
  tone: "ok" | "info" | "warn" | "danger" | "neutral";
} {
  switch (state) {
    case "listening":
      return { label: "Listening", tone: "info" };
    case "translating":
      return { label: "Translating", tone: "info" };
    case "needs_attention":
      return { label: "Needs attention", tone: "warn" };
    case "ready":
      return { label: "Ready", tone: "ok" };
  }
}

function sessionTone(status: SessionStatus): "ok" | "info" | "warn" | "danger" | "neutral" {
  switch (status) {
    case "listening":
    case "translating":
    case "speaking":
      return "ok";
    case "starting":
    case "stopping":
    case "paused":
      return "warn";
    case "error":
      return "danger";
    case "idle":
      return "neutral";
  }
}

function emptyConversationCopy(
  status: SessionStatus,
  sourceSignalState: SourceSignalState,
  hasInput: boolean,
  hasOutput: boolean,
) {
  if (!hasInput || !hasOutput) {
    return {
      title: "Setup needed",
      body: "Choose a meeting source and translated output before starting a session.",
    };
  }
  if (status === "idle") {
    return {
      title: "Ready to listen",
      body: "Start a session and each source line will appear with its translation underneath.",
    };
  }
  if (sourceSignalState === "stale") {
    return {
      title: "No recent source audio",
      body: "Check the selected input or Teams routing if speech should be arriving.",
    };
  }
  if (sourceSignalState === "silent") {
    return {
      title: "Source is connected",
      body: "The stream is healthy and quiet. Speech will appear here when it starts.",
    };
  }
  return {
    title: "Listening for speech",
    body: "Source audio is being monitored. Pause briefly after speaking or press Translate now to commit the utterance.",
  };
}

function isSourceSignalStatusActive(status: SessionStatus) {
  return status === "starting" || activeSessionStatuses.includes(status);
}

function labelTranslationProvider(provider: TranslationProvider) {
  switch (provider) {
    case "google_live_translate":
      return "Google Live Translation";
    case "openai_realtime":
      return "OpenAI Realtime Translation";
    case "local_whisper":
      return "Local Whisper";
  }
}

function normalizeDeviceName(name: string) {
  return name.trim().toLowerCase().replace(/\s+/g, " ");
}

function getRoutingWarnings(
  output?: AudioDeviceInfo,
  outputChannel: AudioOutputChannel = "all",
  monitor?: AudioDeviceInfo,
  monitorChannel: AudioOutputChannel = "all",
) {
  const warnings: string[] = [];
  if (!isStereoCapable(output) && outputChannel !== "all") {
    warnings.push("Translated left/right routing needs a stereo output device.");
  }
  if (!isStereoCapable(monitor) && monitorChannel !== "all") {
    warnings.push("Original left/right routing needs a stereo output device.");
  }
  return warnings;
}

function sameDeviceId(left?: AudioDeviceInfo, right?: AudioDeviceInfo) {
  return Boolean(left && right && left.id.length > 0 && left.id === right.id);
}

function hasOutputMonitorConflict(
  output?: AudioDeviceInfo,
  outputChannel: AudioOutputChannel = "all",
  monitor?: AudioDeviceInfo,
  monitorChannel: AudioOutputChannel = "all",
) {
  if (!sameDeviceId(output, monitor)) {
    return false;
  }

  return !(
    (outputChannel === "left" && monitorChannel === "right") ||
    (outputChannel === "right" && monitorChannel === "left")
  );
}

function channelLevel(
  outputChannel: AudioOutputChannel,
  side: "left" | "right",
  value: number,
) {
  if (outputChannel === "all") {
    return value;
  }
  return outputChannel === side ? value : 0;
}

function readRoutingProfile(): RoutingProfile | null {
  try {
    const raw = window.localStorage.getItem(routingStorageKey);
    return raw ? (JSON.parse(raw) as RoutingProfile) : null;
  } catch {
    return null;
  }
}

function persistRoutingProfile(profile: RoutingProfile) {
  window.localStorage.setItem(routingStorageKey, JSON.stringify(profile));
}

function isStereoCapable(device?: AudioDeviceInfo) {
  return (device?.maxChannels ?? 2) >= 2;
}

function formatDeviceOption(device: AudioDeviceInfo) {
  const channelInfo = device.maxChannels ? `${device.maxChannels}ch` : "channels ?";
  const sampleInfo =
    device.minSampleRate && device.maxSampleRate
      ? device.minSampleRate === device.maxSampleRate
        ? `${device.maxSampleRate}Hz`
        : `${device.minSampleRate}-${device.maxSampleRate}Hz`
      : "sample rate ?";
  return `${device.name} - ${channelInfo}, ${sampleInfo}`;
}

function formatRoute(device: AudioDeviceInfo | undefined, outputChannel: AudioOutputChannel) {
  if (!device) {
    return "No output";
  }
  return `${device.name} (${channelLabel(outputChannel)})`;
}

function channelLabel(outputChannel: AudioOutputChannel) {
  switch (outputChannel) {
    case "left":
      return "left";
    case "right":
      return "right";
    case "all":
      return "both ears";
  }
}

function labelApiKeySource(source: ApiKeySource | null) {
  switch (source) {
    case "environment":
      return "env";
    case "keychain":
      return isWindows ? "Windows Credential Manager" : "Keychain";
    case "memory":
      return "memory";
    default:
      return "unknown";
  }
}

function profileToDraft(profile: LlmProviderProfile): LlmProviderProfileDraft {
  return {
    id: profile.id,
    name: profile.name,
    kind: profile.kind,
    model: profile.model,
    baseUrl: profile.baseUrl,
    timeoutSeconds: profile.timeoutSeconds,
    maxOutputTokens: profile.maxOutputTokens,
    temperature: profile.temperature,
    enabled: profile.enabled,
  };
}

function profileNumberValue(value?: number) {
  return Number.isFinite(value) ? String(value) : "";
}

function parseProfileNumberInput(value: string, integer = false) {
  const trimmed = value.trim();
  if (!trimmed) {
    return undefined;
  }
  const parsed = Number(trimmed);
  if (!Number.isFinite(parsed)) {
    return undefined;
  }
  return integer ? Math.trunc(parsed) : parsed;
}

function defaultBaseUrlForKind(kind: LlmProviderKind, current?: string) {
  if (kind === "openai") {
    return "https://api.openai.com/v1";
  }
  if (kind === "ollama") {
    return "http://localhost:11434/v1";
  }
  return current ?? "";
}

function formatClock(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function normalizeError(cause: unknown): AppErrorPayload {
  if (typeof cause === "object" && cause && "message" in cause) {
    const maybe = cause as Partial<AppErrorPayload>;
    return {
      code: maybe.code ?? "app_error",
      message: maybe.message ?? "Unexpected application error.",
    };
  }
  return {
    code: "app_error",
    message: String(cause),
  };
}

function downloadText(fileName: string, content: string) {
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.click();
  URL.revokeObjectURL(url);
}
