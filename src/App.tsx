import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  AlertTriangle,
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
  Square,
  Volume2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  exportTranscript,
  forceTranslateBoundary,
  getAppStatus,
  hasApiKey,
  listAudioDevices,
  pauseSession,
  playTestTone,
  resumeSession,
  saveApiKey,
  startSession,
  stopSession,
} from "./api";
import { mergeTranscriptDelta, renderTranscript } from "./transcript";
import type {
  AppErrorPayload,
  AudioDeviceInfo,
  AudioDevices,
  AudioOutputChannel,
  AudioLevelEvent,
  Language,
  ManualBoundaryEvent,
  ManualBoundaryReason,
  SessionConfig,
  SessionStatus,
  TranscriptItem,
  TranslationStyle,
} from "./types";

const languages: Array<{ value: Language; label: string }> = [
  { value: "auto", label: "Auto" },
  { value: "en", label: "English" },
  { value: "ja", label: "Japanese" },
  { value: "vi", label: "Vietnamese" },
];

const targetLanguages = languages.filter((language) => language.value !== "auto");

const styles: Array<{ value: TranslationStyle; label: string }> = [
  { value: "literal", label: "Literal" },
  { value: "natural", label: "Natural" },
  { value: "technical_meeting_safe", label: "Technical" },
];

const voices = ["marin", "cedar", "coral", "verse", "alloy", "nova"];
const channelOptions: Array<{ value: AudioOutputChannel; label: string }> = [
  { value: "all", label: "Both ears" },
  { value: "left", label: "Left ear" },
  { value: "right", label: "Right ear" },
];
const routingStorageKey = "baka-trans-routing-profile-v1";
const activeSessionStatuses: SessionStatus[] = ["listening", "translating", "speaking"];

interface RoutingProfile {
  inputDeviceId: string;
  outputDeviceId: string;
  translationOutputChannel: AudioOutputChannel;
  monitorOutputDeviceId: string;
  monitorOutputChannel: AudioOutputChannel;
  monitorOriginalAudio: boolean;
}

export default function App() {
  const [devices, setDevices] = useState<AudioDevices>({ inputs: [], outputs: [] });
  const [sourceLanguage, setSourceLanguage] = useState<Language>("auto");
  const [targetLanguage, setTargetLanguage] = useState<Language>("en");
  const [translationStyle, setTranslationStyle] =
    useState<TranslationStyle>("technical_meeting_safe");
  const [inputDeviceId, setInputDeviceId] = useState("");
  const [outputDeviceId, setOutputDeviceId] = useState("");
  const [translationOutputChannel, setTranslationOutputChannel] =
    useState<AudioOutputChannel>("all");
  const [monitorOutputDeviceId, setMonitorOutputDeviceId] = useState("");
  const [monitorOutputChannel, setMonitorOutputChannel] = useState<AudioOutputChannel>("all");
  const [monitorOriginalAudio, setMonitorOriginalAudio] = useState(false);
  const [voiceId, setVoiceId] = useState("marin");
  const [fallbackEnabled, setFallbackEnabled] = useState(false);
  const [status, setStatus] = useState<SessionStatus>("idle");
  const [apiKeyStored, setApiKeyStored] = useState(false);
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [error, setError] = useState<AppErrorPayload | null>(null);
  const [boundaryFeedback, setBoundaryFeedback] = useState("");
  const [level, setLevel] = useState({ peak: 0, rms: 0 });
  const [transcript, setTranscript] = useState<TranscriptItem[]>([]);
  const [busy, setBusy] = useState(false);
  const [testingTone, setTestingTone] = useState<"translation" | "monitor" | null>(null);
  const [lastRefreshedAt, setLastRefreshedAt] = useState<number | null>(null);
  const transcriptEndRef = useRef<HTMLDivElement | null>(null);

  const selectedInput = devices.inputs.find((device) => device.id === inputDeviceId);
  const selectedOutput = devices.outputs.find((device) => device.id === outputDeviceId);
  const selectedMonitorOutput = devices.outputs.find(
    (device) => device.id === monitorOutputDeviceId,
  );
  const routingWarnings = getRoutingWarnings(
    selectedInput,
    selectedOutput,
    translationOutputChannel,
    monitorOriginalAudio ? selectedMonitorOutput : undefined,
    monitorOutputChannel,
  );

  const canStart =
    status === "idle" &&
    inputDeviceId.length > 0 &&
    outputDeviceId.length > 0 &&
    (!monitorOriginalAudio || monitorOutputDeviceId.length > 0) &&
    apiKeyStored;
  const canForceBoundary = activeSessionStatuses.includes(status);
  const canPause = canForceBoundary;
  const canResume = status === "paused";
  const canStop = status !== "idle" && status !== "stopping";
  const canTestAudio = status === "idle" && !busy;
  const inputSignalPercent = Math.round(level.peak * 100);
  const inputSignalLabel =
    inputSignalPercent > 3 ? `${inputSignalPercent}% peak` : status === "idle" ? "Idle" : "No signal";
  const readinessLabel = canStart
    ? "Ready"
    : status === "idle"
      ? "Setup needed"
      : labelStatus(status);

  const config: SessionConfig = useMemo(
    () => ({
      sourceLanguage,
      targetLanguage,
      translationStyle,
      inputDeviceId,
      outputDeviceId,
      translationOutputChannel,
      monitorOutputDeviceId,
      monitorOutputChannel,
      monitorOriginalAudio,
      voiceId,
      fallbackEnabled,
    }),
    [
      fallbackEnabled,
      inputDeviceId,
      monitorOriginalAudio,
      monitorOutputChannel,
      monitorOutputDeviceId,
      outputDeviceId,
      sourceLanguage,
      targetLanguage,
      translationOutputChannel,
      translationStyle,
      voiceId,
    ],
  );

  useEffect(() => {
    void hydrate();

    const unlisten = Promise.all([
      listen<SessionStatus>("session-status", (event) => {
        setStatus(event.payload);
        if (event.payload === "idle") {
          setLevel({ peak: 0, rms: 0 });
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
      listen<AppErrorPayload>("app-error", (event) => setError(event.payload)),
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
      const [deviceList, appStatus, stored] = await Promise.all([
        listAudioDevices(),
        getAppStatus(),
        hasApiKey(),
      ]);
      setDevices(deviceList);
      setStatus(appStatus.sessionStatus);
      setApiKeyStored(stored);
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

  async function saveKey() {
    setBusy(true);
    setError(null);
    try {
      await saveApiKey(apiKeyDraft);
      const stored = await hasApiKey();
      setApiKeyStored(stored);
      if (stored) {
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

  async function doExport(format: "text" | "markdown") {
    const localContent = renderTranscript(transcript, format);
    try {
      const backend = await exportTranscript(format);
      downloadText(backend.fileName, backend.content || localContent);
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
              options={languages}
            />
            <SelectField
              label="Target"
              value={targetLanguage}
              onChange={(value) => setTargetLanguage(value as Language)}
              options={targetLanguages}
            />
            <SelectField
              label="Style"
              value={translationStyle}
              onChange={(value) => setTranslationStyle(value as TranslationStyle)}
              options={styles}
            />
            <SelectField
              label="Voice"
              value={voiceId}
              onChange={setVoiceId}
              options={voices.map((voice) => ({ value: voice, label: voice }))}
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
            description="Captured and sent for translation."
            devices={devices.inputs}
            value={inputDeviceId}
            onChange={updateInputDevice}
          />
          <DeviceSelect
            icon={<Headphones size={17} />}
            label="Translated audio"
            description="Private translated speech playback."
            devices={devices.outputs}
            value={outputDeviceId}
            onChange={updateOutputDevice}
          />
          <SelectField
            label="Translated channel"
            value={translationOutputChannel}
            onChange={(value) => updateTranslationOutputChannel(value as AudioOutputChannel)}
            options={channelOptions}
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
          <div className="button-row compact">
            <button
              className="small-button"
              onClick={() =>
                monitorOutputDeviceId &&
                testTone("monitor", monitorOutputDeviceId, monitorOutputChannel)
              }
              disabled={
                !monitorOriginalAudio ||
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
            monitor={monitorOriginalAudio ? selectedMonitorOutput : undefined}
            monitorChannel={monitorOutputChannel}
          />
          {routingWarnings.map((warning) => (
            <InlineWarning text={warning} key={warning} />
          ))}
        </div>

        <div className="panel key-panel">
          <div className="panel-header">
            <h2>Settings</h2>
          </div>
          <div className="key-row">
            <input
              type="password"
              value={apiKeyDraft}
              placeholder={apiKeyStored ? "Saved in Keychain" : "OpenAI API key"}
              onChange={(event) => setApiKeyDraft(event.currentTarget.value)}
            />
            <button onClick={saveKey} disabled={apiKeyDraft.trim().length === 0 || busy}>
              <Save size={17} /> Save
            </button>
          </div>
          <div className="setup-list">
            <div>
              <strong>1. Route</strong>
              <span>Send meeting audio to BlackHole 2ch.</span>
            </div>
            <div>
              <strong>2. Select</strong>
              <span>Choose BlackHole as source and headphones as translation output.</span>
            </div>
            <div>
              <strong>3. Split</strong>
              <span>For left/right ears, use one headphone device with opposite channels.</span>
            </div>
          </div>
        </div>
      </section>

      {error ? (
        <section className="error-bar">
          <AlertTriangle size={18} />
          <span>{error.message}</span>
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
    </main>
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
    sameDeviceName(output, monitor) &&
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
  input?: AudioDeviceInfo,
  output?: AudioDeviceInfo,
  outputChannel: AudioOutputChannel = "all",
  monitor?: AudioDeviceInfo,
  monitorChannel: AudioOutputChannel = "all",
) {
  const warnings: string[] = [];
  if (sameDeviceName(input, output)) {
    warnings.push("Meeting source and translated output look identical.");
  }
  if (sameDeviceName(input, monitor)) {
    warnings.push("Meeting source and original monitor look identical.");
  }
  if (sameDeviceName(output, monitor)) {
    const splitActive =
      (outputChannel === "left" && monitorChannel === "right") ||
      (outputChannel === "right" && monitorChannel === "left");
    if (!splitActive) {
      warnings.push("Translated output and original monitor share a device. Use opposite ears or separate outputs.");
    }
  }
  if (!isStereoCapable(output) && outputChannel !== "all") {
    warnings.push("Translated left/right routing needs a stereo output device.");
  }
  if (!isStereoCapable(monitor) && monitorChannel !== "all") {
    warnings.push("Original left/right routing needs a stereo output device.");
  }
  return warnings;
}

function sameDeviceName(left?: AudioDeviceInfo, right?: AudioDeviceInfo) {
  return Boolean(
    left &&
      right &&
      normalizeDeviceName(left.name).length > 0 &&
      normalizeDeviceName(left.name) === normalizeDeviceName(right.name),
  );
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
