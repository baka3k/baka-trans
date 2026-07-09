import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  AlertTriangle,
  Bot,
  Check,
  Download,
  FileText,
  Headphones,
  KeyRound,
  Mic,
  Pause,
  Play,
  RefreshCw,
  Save,
  Send,
  Settings2,
  Square,
  Trash2,
  Volume2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  exportTranscript,
  forceTranslateBoundary,
  getAppStatus,
  deleteLlmProfile,
  listAudioDevices,
  listLlmProfiles,
  pauseSession,
  playTestTone,
  resumeSession,
  runMeetingSummaryAgent,
  saveLlmProfile,
  saveTranslationApiKey,
  startLocalMonitor,
  startSession,
  stopLocalMonitor,
  stopSession,
  testLlmProfile,
  testTranslationApiKey,
} from "./api";
import {
  buildMeetingSummaryConfig,
  mergeTranscriptDelta,
  renderTranscript,
  validateLlmProfileDraft,
} from "./transcript";
import { sourceLanguageOptions, targetLanguageOptions } from "./languages";
import type {
  AppErrorPayload,
  ApiKeySource,
  AudioDeviceInfo,
  AudioDevices,
  AudioOutputChannel,
  AudioLevelEvent,
  Language,
  LlmProviderKind,
  LlmProviderProfile,
  LlmProviderProfileDraft,
  ManualBoundaryEvent,
  ManualBoundaryReason,
  MeetingSummaryConfig,
  MeetingSummaryResult,
  MeetingSummaryStatusEvent,
  SessionConfig,
  SessionStatus,
  TranscriptScope,
  TranslatedAudioLevelEvent,
  TranscriptItem,
} from "./types";

const channelOptions: Array<{ value: AudioOutputChannel; label: string }> = [
  { value: "all", label: "Both ears" },
  { value: "left", label: "Left ear" },
  { value: "right", label: "Right ear" },
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
const routingStorageKey = "baka-trans-routing-profile-v1";
const activeSessionStatuses: SessionStatus[] = ["listening", "translating", "speaking"];

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

export default function App() {
  const [devices, setDevices] = useState<AudioDevices>({ inputs: [], outputs: [] });
  const [sourceLanguage, setSourceLanguage] = useState<Language>("auto");
  const [targetLanguage, setTargetLanguage] = useState<Language>("en");
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
  const [level, setLevel] = useState({ peak: 0, rms: 0 });
  const [translatedLevel, setTranslatedLevel] = useState({ peak: 0, rms: 0, sampleCount: 0 });
  const [transcript, setTranscript] = useState<TranscriptItem[]>([]);
  const [busy, setBusy] = useState(false);
  const [testingKey, setTestingKey] = useState(false);
  const [testingTone, setTestingTone] = useState<"translation" | "monitor" | null>(null);
  const [localMonitorActive, setLocalMonitorActive] = useState(false);
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
  const transcriptEndRef = useRef<HTMLDivElement | null>(null);

  const selectedInput = devices.inputs.find((device) => device.id === inputDeviceId);
  const selectedOutput = devices.outputs.find((device) => device.id === outputDeviceId);
  const selectedMonitorOutput = devices.outputs.find(
    (device) => device.id === monitorOutputDeviceId,
  );
  const outputMonitorConflict = hasOutputMonitorConflict(
    selectedOutput,
    translationOutputChannel,
    monitorOriginalAudio ? selectedMonitorOutput : undefined,
    monitorOutputChannel,
  );
  const effectiveMonitorOriginalAudio = monitorOriginalAudio && !outputMonitorConflict;
  const routingWarnings = getRoutingWarnings(
    selectedOutput,
    translationOutputChannel,
    effectiveMonitorOriginalAudio ? selectedMonitorOutput : undefined,
    monitorOutputChannel,
  );

  const canStart =
    (status === "idle" || status === "error") &&
    !localMonitorActive &&
    inputDeviceId.length > 0 &&
    outputDeviceId.length > 0 &&
    (!monitorOriginalAudio || monitorOutputDeviceId.length > 0);
  const canForceBoundary = activeSessionStatuses.includes(status);
  const canPause = canForceBoundary;
  const canResume = status === "paused";
  const canStop = status !== "idle" && status !== "stopping";
  const canTestAudio = status === "idle" && !busy && !localMonitorActive;
  const canToggleLocalMonitor =
    status === "idle" &&
    !busy &&
    (localMonitorActive || (inputDeviceId.length > 0 && outputDeviceId.length > 0));
  const inputSignalPercent = Math.round(level.peak * 100);
  const inputSignalLabel =
    inputSignalPercent > 3
      ? `${inputSignalPercent}% peak`
      : localMonitorActive
        ? "Monitoring"
      : status === "idle"
        ? "Idle"
        : "No signal";
  const translationOutLevel = localMonitorActive
    ? inputSignalPercent
    : Math.round(translatedLevel.peak * 100);
  const monitorOutLevel = effectiveMonitorOriginalAudio && canForceBoundary ? inputSignalPercent : 0;
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
            : canForceBoundary
              ? "listening"
              : "idle",
      leftLevel: inputSignalPercent,
      rightLevel: inputSignalPercent,
      disabled: !selectedInput,
    },
    {
      id: "translation",
      label: "Translated out",
      detail: formatRoute(selectedOutput, translationOutputChannel),
      state: localMonitorActive
        ? translationOutLevel > 3
          ? "signal"
          : "armed"
        : status === "speaking"
          ? "speaking"
          : canForceBoundary
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
  const readinessLabel = canStart
    ? "Ready"
    : status === "idle"
      ? "Setup needed"
      : labelStatus(status);
  const selectedProfile = llmProfiles.find((profile) => profile.id === selectedProfileId);
  const profileDraftErrors = validateLlmProfileDraft(profileDraft);
  const canRunSummary =
    Boolean(selectedProfileId) &&
    !summaryRunning &&
    transcript.some((item) => item.sourceText.trim() || item.translatedText.trim());

  const config: SessionConfig = useMemo(
    () => ({
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
    ],
  );

  useEffect(() => {
    void hydrate();

    const unlisten = Promise.all([
      listen<SessionStatus>("session-status", (event) => {
        setStatus(event.payload);
        if (event.payload === "idle") {
          setLevel({ peak: 0, rms: 0 });
          setTranslatedLevel({ peak: 0, rms: 0, sampleCount: 0 });
        }
      }),
      listen<TranscriptItem>("transcript-update", (event) => {
        setTranscript((items) => mergeTranscriptDelta(items, event.payload));
      }),
      listen<ManualBoundaryEvent>("manual-boundary-status", (event) => {
        setBoundaryFeedback(event.payload.message);
      }),
      listen<AudioLevelEvent>("audio-level", (event) => {
        setLevel({
          peak: Math.max(0, Math.min(1, event.payload.peak)),
          rms: Math.max(0, Math.min(1, event.payload.rms)),
        });
      }),
      listen<TranslatedAudioLevelEvent>("translated-audio-level", (event) => {
        setTranslatedLevel({
          peak: Math.max(0, Math.min(1, event.payload.peak)),
          rms: Math.max(0, Math.min(1, event.payload.rms)),
          sampleCount: event.payload.sampleCount,
        });
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
    transcriptEndRef.current?.scrollIntoView({ block: "end" });
  }, [transcript]);

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
      const [deviceList, appStatus, profiles] = await Promise.all([
        listAudioDevices(),
        getAppStatus(),
        listLlmProfiles(),
      ]);
      setDevices(deviceList);
      setStatus(appStatus.sessionStatus);
      applyAppStatus(appStatus);
      applyProfiles(profiles);
      setKeyTestMessage("");
      const storedRouting = readRoutingProfile();
      setInputDeviceId(resolveStoredDevice(deviceList.inputs, storedRouting?.inputDeviceId));
      setOutputDeviceId(resolveStoredDevice(deviceList.outputs, storedRouting?.outputDeviceId));
      setTranslationOutputChannel(storedRouting?.translationOutputChannel ?? "all");
      setMonitorOutputDeviceId(
        resolveStoredDevice(deviceList.outputs, storedRouting?.monitorOutputDeviceId),
      );
      setMonitorOutputChannel(storedRouting?.monitorOutputChannel ?? "all");
      setMonitorOriginalAudio(storedRouting?.monitorOriginalAudio ?? false);
      setLastRefreshedAt(Date.now());
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
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

  async function saveKey() {
    setBusy(true);
    setError(null);
    try {
      await saveTranslationApiKey(apiKeyDraft);
      const appStatus = await getAppStatus();
      applyAppStatus(appStatus);
      setKeyTestMessage("");
      if (appStatus.hasApiKey) {
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
      const saved = await saveLlmProfile({
        ...profileDraft,
        apiKey: profileKeyDraft.trim() || undefined,
      });
      const profiles = await listLlmProfiles();
      setLlmProfiles(profiles);
      setSelectedProfileId(saved.id);
      setSummaryConfig((current) => ({ ...current, providerProfileId: saved.id }));
      setProfileDraft(profileToDraft(saved));
      setProfileKeyDraft("");
      setProfileTestMessage("Profile saved.");
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
    }
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
    if (!selectedProfileId) {
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
    setApiKeyStored(appStatus.hasApiKey);
    setApiKeySource(appStatus.apiKeySource ?? null);
    setApiKeyFingerprint(appStatus.apiKeyFingerprint ?? "");
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
      const result = await testTranslationApiKey();
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
      await playTestTone(deviceId, outputChannel);
    } catch (cause) {
      setError(normalizeError(cause));
    } finally {
      setTestingTone(null);
    }
  }

  async function toggleLocalMonitor() {
    setBusy(true);
    setError(null);
    try {
      if (localMonitorActive) {
        await stopLocalMonitor();
        setLocalMonitorActive(false);
        setLevel({ peak: 0, rms: 0 });
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
    persistRoutingProfile({
      inputDeviceId: deviceId,
      outputDeviceId,
      translationOutputChannel,
      monitorOutputDeviceId,
      monitorOutputChannel,
      monitorOriginalAudio,
    });
  }

  function updateOutputDevice(deviceId: string) {
    setOutputDeviceId(deviceId);
    persistRoutingProfile({
      inputDeviceId,
      outputDeviceId: deviceId,
      translationOutputChannel,
      monitorOutputDeviceId,
      monitorOutputChannel,
      monitorOriginalAudio,
    });
  }

  function updateTranslationOutputChannel(outputChannel: AudioOutputChannel) {
    setTranslationOutputChannel(outputChannel);
    persistRoutingProfile({
      inputDeviceId,
      outputDeviceId,
      translationOutputChannel: outputChannel,
      monitorOutputDeviceId,
      monitorOutputChannel,
      monitorOriginalAudio,
    });
  }

  function updateMonitorOutputDevice(deviceId: string) {
    setMonitorOutputDeviceId(deviceId);
    persistRoutingProfile({
      inputDeviceId,
      outputDeviceId,
      translationOutputChannel,
      monitorOutputDeviceId: deviceId,
      monitorOutputChannel,
      monitorOriginalAudio,
    });
  }

  function updateMonitorOutputChannel(outputChannel: AudioOutputChannel) {
    setMonitorOutputChannel(outputChannel);
    persistRoutingProfile({
      inputDeviceId,
      outputDeviceId,
      translationOutputChannel,
      monitorOutputDeviceId,
      monitorOutputChannel: outputChannel,
      monitorOriginalAudio,
    });
  }

  function updateMonitorEnabled(enabled: boolean) {
    setMonitorOriginalAudio(enabled);
    persistRoutingProfile({
      inputDeviceId,
      outputDeviceId,
      translationOutputChannel,
      monitorOutputDeviceId,
      monitorOutputChannel,
      monitorOriginalAudio: enabled,
    });
  }

  return (
    <main className="app-shell">
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
              {apiKeyStored ? (
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
          <button
            className="icon-button"
            onClick={hydrate}
            disabled={busy}
            title="Refresh devices"
            aria-label="Refresh devices"
          >
            <RefreshCw size={18} />
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

      <div className="workspace-layout">
        <aside className="settings-column" aria-label="Session settings">
          <section className="control-grid">
            <div className="panel controls-panel">
              <div className="panel-header">
                <h2>Session</h2>
                <span className={`panel-state ${canStart ? "ok" : ""}`}>{readinessLabel}</span>
              </div>
              <div className="field-grid two">
                <SelectField
                  label="Source"
                  value={sourceLanguage}
                  onChange={(value) => setSourceLanguage(value as Language)}
                  options={sourceLanguageOptions}
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
              </div>
              <DeviceSelect
                icon={<Mic size={17} />}
                label="Meeting source"
                description="Mac input captured for translation. Route Teams speaker audio here."
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
                  <span style={{ width: `${Math.round(level.rms * 100)}%` }} />
                </div>
              </div>
              <AudioLineMonitor rows={audioLineRows} />
              <div className="meter-row">
                <Volume2 size={17} />
                <button
                  className="small-button"
                  onClick={() =>
                    outputDeviceId &&
                    testTone("translation", outputDeviceId, translationOutputChannel)
                  }
                  disabled={!outputDeviceId || !canTestAudio || testingTone === "translation"}
                >
                  {testingTone === "translation" ? "Testing" : "Test translated"}
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
                    : "No OpenAI call; tests mic, device, and left/right routing."}
                </span>
              </div>
              <div className="button-row compact">
                <button
                  className="small-button"
                  onClick={() =>
                    monitorOutputDeviceId &&
                    testTone("monitor", monitorOutputDeviceId, monitorOutputChannel)
                  }
                  disabled={
                    !effectiveMonitorOriginalAudio ||
                    !monitorOutputDeviceId ||
                    !canTestAudio ||
                    testingTone === "monitor"
                  }
                >
                  {testingTone === "monitor" ? "Testing" : "Test original"}
                </button>
                {lastRefreshedAt ? (
                  <span className="refresh-note">Refreshed {formatClock(lastRefreshedAt)}</span>
                ) : null}
              </div>
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
                <h2>Translation OpenAI key</h2>
              </div>
              <div className="key-row">
                <input
                  type="password"
                  value={apiKeyDraft}
                  placeholder={
                    apiKeyStored
                      ? "Saved translation key. Paste only to replace."
                      : "OpenAI Realtime API key"
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
                    ? `Using ${labelApiKeySource(apiKeySource)} key ${apiKeyFingerprint || ""} for realtime translation`
                    : "No translation OpenAI key available"}
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
              </div>
            </div>

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
                        setProfileDraft((current) => ({
                          ...current,
                          name: event.currentTarget.value,
                        }))
                      }
                    />
                  </label>
                  <label className="field">
                    <span>Model</span>
                    <input
                      value={profileDraft.model}
                      placeholder="gpt-4.1-mini or llama3.2"
                      onChange={(event) =>
                        setProfileDraft((current) => ({
                          ...current,
                          model: event.currentTarget.value,
                        }))
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
                      setProfileDraft((current) => ({
                        ...current,
                        baseUrl: event.currentTarget.value,
                      }))
                    }
                  />
                </label>

                <div className="key-row">
                  <input
                    type="password"
                    value={profileKeyDraft}
                    placeholder={
                      selectedProfile?.hasApiKey
                        ? `Saved key ${selectedProfile.apiKeyFingerprint ?? ""}`
                        : profileDraft.kind === "ollama"
                          ? "Optional placeholder key"
                          : "Summary provider API key"
                    }
                    onChange={(event) => setProfileKeyDraft(event.currentTarget.value)}
                  />
                  <button
                    onClick={saveSummaryProfile}
                    disabled={busy || profileDraftErrors.length > 0}
                  >
                    <Save size={17} /> Save
                  </button>
                </div>

                <div className="profile-actions">
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
                      onChange={(event) =>
                        setSummaryConfig((current) => ({
                          ...current,
                          outputLanguage: event.currentTarget.value,
                        }))
                      }
                    />
                  </label>
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
                        onChange={(event) =>
                          setSummaryConfig((current) => ({
                            ...current,
                            sections: {
                              ...current.sections,
                              [key]: event.currentTarget.checked,
                            },
                          }))
                        }
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
        </aside>

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

          <section className="transcript-panel">
            <div className="transcript-head">
              <span>Original speech</span>
              <span>Translated speech</span>
            </div>
            <div className="transcript-list">
              {transcript.length === 0 ? (
                <div className="empty-state">
                  <FileText size={28} />
                  <strong>No transcript yet</strong>
                  <p>Start a session to see original speech and translation side by side.</p>
                </div>
              ) : (
                transcript.map((item) => (
                  <article className={`transcript-row ${item.status}`} key={item.id}>
                    <p>{item.sourceText}</p>
                    <p>{item.translatedText}</p>
                  </article>
                ))
              )}
              <div ref={transcriptEndRef} />
            </div>
          </section>
        </section>
      </div>
    </main>
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

function labelStatus(status: SessionStatus) {
  return status.replace(/_/g, " ");
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
      return "Keychain";
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
