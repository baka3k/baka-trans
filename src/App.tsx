import { listen } from "@tauri-apps/api/event";
import {
  AlertTriangle,
  Check,
  Download,
  Headphones,
  KeyRound,
  Mic,
  Pause,
  Play,
  RefreshCw,
  Save,
  Square,
  Volume2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  exportTranscript,
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
  AudioLevelEvent,
  Language,
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

export default function App() {
  const [devices, setDevices] = useState<AudioDevices>({ inputs: [], outputs: [] });
  const [sourceLanguage, setSourceLanguage] = useState<Language>("auto");
  const [targetLanguage, setTargetLanguage] = useState<Language>("en");
  const [translationStyle, setTranslationStyle] =
    useState<TranslationStyle>("technical_meeting_safe");
  const [inputDeviceId, setInputDeviceId] = useState("");
  const [outputDeviceId, setOutputDeviceId] = useState("");
  const [voiceId, setVoiceId] = useState("marin");
  const [fallbackEnabled, setFallbackEnabled] = useState(false);
  const [status, setStatus] = useState<SessionStatus>("idle");
  const [apiKeyStored, setApiKeyStored] = useState(false);
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [error, setError] = useState<AppErrorPayload | null>(null);
  const [level, setLevel] = useState(0);
  const [transcript, setTranscript] = useState<TranscriptItem[]>([]);
  const [busy, setBusy] = useState(false);
  const transcriptEndRef = useRef<HTMLDivElement | null>(null);

  const selectedInput = devices.inputs.find((device) => device.id === inputDeviceId);
  const selectedOutput = devices.outputs.find((device) => device.id === outputDeviceId);
  const feedbackWarning =
    selectedInput &&
    selectedOutput &&
    normalizeDeviceName(selectedInput.name) === normalizeDeviceName(selectedOutput.name);

  const canStart =
    status === "idle" && inputDeviceId.length > 0 && outputDeviceId.length > 0 && apiKeyStored;
  const canPause = ["listening", "translating", "speaking"].includes(status);
  const canResume = status === "paused";
  const canStop = status !== "idle" && status !== "stopping";

  const config: SessionConfig = useMemo(
    () => ({
      sourceLanguage,
      targetLanguage,
      translationStyle,
      inputDeviceId,
      outputDeviceId,
      voiceId,
      fallbackEnabled,
    }),
    [
      fallbackEnabled,
      inputDeviceId,
      outputDeviceId,
      sourceLanguage,
      targetLanguage,
      translationStyle,
      voiceId,
    ],
  );

  useEffect(() => {
    void hydrate();

    const unlisten = Promise.all([
      listen<SessionStatus>("session-status", (event) => setStatus(event.payload)),
      listen<TranscriptItem>("transcript-update", (event) => {
        setTranscript((items) => mergeTranscriptDelta(items, event.payload));
      }),
      listen<AudioLevelEvent>("audio-level", (event) => {
        setLevel(Math.max(0, Math.min(1, event.payload.peak)));
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
      setInputDeviceId(selectDefaultDevice(deviceList.inputs));
      setOutputDeviceId(selectDefaultDevice(deviceList.outputs));
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
      setApiKeyDraft("");
      setApiKeyStored(true);
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
      setError(normalizeError(cause));
    } finally {
      setBusy(false);
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

  return (
    <main className="app-shell">
      <header className="topbar">
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
        <div className="top-actions">
          <button className="icon-button" onClick={hydrate} disabled={busy} title="Refresh devices">
            <RefreshCw size={18} />
          </button>
          <button
            className="icon-button"
            onClick={() => void doExport("text")}
            disabled={transcript.length === 0}
            title="Export text"
          >
            <Download size={18} />
          </button>
          <button
            className="icon-button"
            onClick={() => void doExport("markdown")}
            disabled={transcript.length === 0}
            title="Export Markdown"
          >
            MD
          </button>
        </div>
      </header>

      <section className="control-grid">
        <div className="panel controls-panel">
          <div className="section-title">Session</div>
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
          </div>

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
          <div className="section-title">Audio</div>
          <DeviceSelect
            icon={<Mic size={17} />}
            label="Input"
            devices={devices.inputs}
            value={inputDeviceId}
            onChange={setInputDeviceId}
          />
          <DeviceSelect
            icon={<Headphones size={17} />}
            label="Output"
            devices={devices.outputs}
            value={outputDeviceId}
            onChange={setOutputDeviceId}
          />
          <div className="meter-row">
            <Volume2 size={17} />
            <div className="meter">
              <div style={{ width: `${Math.round(level * 100)}%` }} />
            </div>
            <button
              className="small-button"
              onClick={() => outputDeviceId && runCommand(() => playTestTone(outputDeviceId))}
              disabled={!outputDeviceId || busy}
            >
              Test
            </button>
          </div>
          {feedbackWarning ? <InlineWarning text="Input and output look identical." /> : null}
        </div>

        <div className="panel key-panel">
          <div className="section-title">Settings</div>
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
          <ol className="checklist">
            <li>Install BlackHole 2ch.</li>
            <li>Set Teams speaker output to BlackHole or a multi-output route.</li>
            <li>Select BlackHole as input here.</li>
            <li>Select headphones as output here.</li>
          </ol>
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
          <span>Source</span>
          <span>Translation</span>
        </div>
        <div className="transcript-list">
          {transcript.length === 0 ? (
            <div className="empty-row">
              <span />
              <span />
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
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <select value={value} onChange={(event) => onChange(event.currentTarget.value)}>
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
  devices,
  value,
  onChange,
}: {
  icon: React.ReactNode;
  label: string;
  devices: AudioDeviceInfo[];
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="device-field">
      <span>
        {icon}
        {label}
      </span>
      <select value={value} onChange={(event) => onChange(event.currentTarget.value)}>
        <option value="">No device</option>
        {devices.map((device) => (
          <option value={device.id} key={device.id}>
            {device.name}
            {device.isDefault ? " (default)" : ""}
          </option>
        ))}
      </select>
    </label>
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

function labelStatus(status: SessionStatus) {
  return status.replace(/_/g, " ");
}

function normalizeDeviceName(name: string) {
  return name.trim().toLowerCase().replace(/\s+/g, " ");
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
