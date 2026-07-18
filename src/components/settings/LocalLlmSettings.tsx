import type {
  LocalTranslationConfigDraft,
  LocalTranslationTestResult,
  LocalVoice,
} from "../../types";

export const defaultLocalTranslationConfig: LocalTranslationConfigDraft = {
  baseUrl: "http://localhost:11434",
  model: "gemma3:4b",
  timeoutSeconds: 30,
  temperature: 0,
  maxOutputTokens: 256,
  keepAlive: "10m",
  modelPath: "",
  language: "ja",
  threads: 4,
  useGpu: false,
  sampleRateHz: 16000,
  minimumSpeechMs: 300,
  silenceToCommitMs: 700,
  maximumUtteranceMs: 15000,
  preRollMs: 250,
  speechThreshold: 0.015,
  voiceId: "",
  ttsRate: 1,
  ttsVolume: 1,
  ttsOutputSampleRateHz: 24000,
};

interface LocalLlmSettingsProps {
  draft: LocalTranslationConfigDraft;
  dirty: boolean;
  saving: boolean;
  testing: boolean;
  testResult: LocalTranslationTestResult | null;
  voices: LocalVoice[];
  previewing: boolean;
  previewDisabled?: boolean;
  onChange: (draft: LocalTranslationConfigDraft) => void;
  onSave: () => void;
  onTest: () => void;
  onPreview: () => void;
}

export function validateLocalTranslationDraft(draft: LocalTranslationConfigDraft): string[] {
  const errors: string[] = [];
  if (!/^https?:\/\//i.test(draft.baseUrl.trim())) {
    errors.push("Enter an http or https Ollama server URL.");
  }
  if (!draft.model.trim()) {
    errors.push("Choose an installed Ollama model.");
  }
  if (!draft.voiceId.trim()) {
    errors.push("Choose an installed local voice.");
  }
  if (!draft.modelPath.trim()) {
    errors.push("Choose a Whisper GGML model file.");
  }
  if (draft.sampleRateHz !== 16000) {
    errors.push("Whisper input must remain fixed at 16000 Hz.");
  }
  if (draft.ttsOutputSampleRateHz !== 24000) {
    errors.push("Local voice output must remain fixed at 24000 Hz.");
  }
  if (draft.ttsRate < 0.5 || draft.ttsRate > 2) {
    errors.push("Speaking rate must be between 0.5 and 2.0.");
  }
  if (draft.ttsVolume < 0 || draft.ttsVolume > 1) {
    errors.push("Voice volume must be between 0 and 1.");
  }
  if (draft.minimumSpeechMs < 100 || draft.minimumSpeechMs > 3000) {
    errors.push("Minimum speech must be between 100 and 3000 ms.");
  }
  if (draft.silenceToCommitMs < 200 || draft.silenceToCommitMs > 5000) {
    errors.push("Trailing silence must be between 200 and 5000 ms.");
  }
  if (draft.maximumUtteranceMs < 1000 || draft.maximumUtteranceMs > 60000) {
    errors.push("Maximum utterance must be between 1000 and 60000 ms.");
  }
  if (draft.minimumSpeechMs > draft.maximumUtteranceMs) {
    errors.push("Minimum speech cannot exceed maximum utterance.");
  }
  return errors;
}

export function LocalLlmSettings({
  draft,
  dirty,
  saving,
  testing,
  testResult,
  voices,
  previewing,
  previewDisabled = false,
  onChange,
  onSave,
  onTest,
  onPreview,
}: LocalLlmSettingsProps) {
  const errors = validateLocalTranslationDraft(draft);
  const update = (patch: Partial<LocalTranslationConfigDraft>) =>
    onChange({ ...draft, ...patch });
  const number = (value: string, fallback: number) => {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  };

  return (
    <div className="panel local-llm-panel">
      <div className="panel-header">
        <div>
          <h2>Local LLM translation</h2>
          <p>Japanese speech → Whisper → Gemma → Vietnamese voice</p>
        </div>
        <span className={`panel-state ${testResult?.ok && !dirty ? "ok" : ""}`}>
          {testResult?.ok && !dirty ? "Ready" : dirty ? "Unsaved" : "Test required"}
        </span>
      </div>

      <fieldset className="local-config-group">
        <legend>Ollama</legend>
        <label className="field">
          <span>Server URL</span>
          <input
            value={draft.baseUrl}
            placeholder="http://localhost:11434"
            onChange={(event) => update({ baseUrl: event.currentTarget.value })}
          />
          <small>The native client always sends POST /api/chat, never /v1/chat/completions.</small>
        </label>
        <label className="field">
          <span>Installed Gemma model</span>
          <input
            value={draft.model}
            placeholder="gemma3:4b"
            onChange={(event) => update({ model: event.currentTarget.value })}
          />
        </label>
        <div className="field-grid two">
          <NumberField
            label="Timeout (seconds)"
            min={5}
            max={300}
            step={1}
            value={draft.timeoutSeconds}
            onChange={(value) => update({ timeoutSeconds: number(value, draft.timeoutSeconds) })}
          />
          <NumberField
            label="Maximum output tokens"
            min={32}
            max={2048}
            step={1}
            value={draft.maxOutputTokens}
            onChange={(value) => update({ maxOutputTokens: number(value, draft.maxOutputTokens) })}
          />
          <NumberField
            label="Temperature"
            min={0}
            max={1}
            step={0.1}
            value={draft.temperature}
            onChange={(value) => update({ temperature: number(value, draft.temperature) })}
          />
          <label className="field">
            <span>Keep alive</span>
            <input
              value={draft.keepAlive ?? ""}
              placeholder="10m"
              onChange={(event) => update({ keepAlive: event.currentTarget.value || undefined })}
            />
          </label>
        </div>
      </fieldset>

      <fieldset className="local-config-group">
        <legend>Vietnamese voice</legend>
        <label className="field">
          <span>Installed system voice</span>
          <select
            value={draft.voiceId}
            onChange={(event) => update({ voiceId: event.currentTarget.value })}
          >
            <option value="">Choose a voice</option>
            {voices.map((voice) => (
              <option value={voice.id} key={voice.id}>
                {voice.name} · {voice.language}
              </option>
            ))}
          </select>
          <small>Vietnamese voices are shown first. Install one in system speech settings if empty.</small>
        </label>
        <div className="field-grid two">
          <NumberField
            label="Speaking rate"
            min={0.5}
            max={2}
            step={0.1}
            value={draft.ttsRate}
            onChange={(value) => update({ ttsRate: number(value, draft.ttsRate) })}
          />
          <NumberField
            label="Volume"
            min={0}
            max={1}
            step={0.05}
            value={draft.ttsVolume}
            onChange={(value) => update({ ttsVolume: number(value, draft.ttsVolume) })}
          />
          <label className="field">
            <span>Playback format</span>
            <input value="PCM16 mono · 24000 Hz" readOnly aria-readonly="true" />
          </label>
        </div>
      </fieldset>

      <fieldset className="local-config-group">
        <legend>Whisper</legend>
        <label className="field">
          <span>GGML model path</span>
          <input
            value={draft.modelPath}
            placeholder="C:\\models\\ggml-small.bin"
            onChange={(event) => update({ modelPath: event.currentTarget.value })}
          />
          <small>Use an absolute path to a model file. Model binaries are not bundled.</small>
        </label>
        <div className="field-grid two">
          <label className="field">
            <span>Language</span>
            <input value="Japanese (ja)" readOnly aria-readonly="true" />
          </label>
          <NumberField
            label="CPU threads"
            min={1}
            max={128}
            step={1}
            value={draft.threads}
            onChange={(value) => update({ threads: number(value, draft.threads) })}
          />
        </div>
        <label className="toggle-row no-margin">
          <input
            type="checkbox"
            checked={draft.useGpu}
            onChange={(event) => update({ useGpu: event.currentTarget.checked })}
          />
          <span>Request GPU acceleration (falls back safely when unavailable)</span>
        </label>
      </fieldset>

      <fieldset className="local-config-group">
        <legend>Audio to text</legend>
        <div className="field-grid two">
          <label className="field">
            <span>Input format</span>
            <input value="PCM16 mono · 16000 Hz" readOnly aria-readonly="true" />
          </label>
          <NumberField
            label="Speech threshold"
            min={0.001}
            max={0.25}
            step={0.001}
            value={draft.speechThreshold}
            onChange={(value) => update({ speechThreshold: number(value, draft.speechThreshold) })}
          />
          <NumberField
            label="Minimum speech (ms)"
            min={100}
            max={3000}
            step={50}
            value={draft.minimumSpeechMs}
            onChange={(value) => update({ minimumSpeechMs: number(value, draft.minimumSpeechMs) })}
          />
          <NumberField
            label="Trailing silence (ms)"
            min={200}
            max={5000}
            step={50}
            value={draft.silenceToCommitMs}
            onChange={(value) => update({ silenceToCommitMs: number(value, draft.silenceToCommitMs) })}
          />
          <NumberField
            label="Maximum utterance (ms)"
            min={1000}
            max={60000}
            step={500}
            value={draft.maximumUtteranceMs}
            onChange={(value) => update({ maximumUtteranceMs: number(value, draft.maximumUtteranceMs) })}
          />
          <NumberField
            label="Pre-roll (ms)"
            min={0}
            max={2000}
            step={50}
            value={draft.preRollMs}
            onChange={(value) => update({ preRollMs: number(value, draft.preRollMs) })}
          />
        </div>
      </fieldset>

      <div className="local-health-grid" aria-live="polite">
        <Health label="Configuration saved" ok={!dirty} />
        <Health
          label="Whisper model readable and loaded"
          ok={Boolean(testResult?.whisperModelReadable && testResult.whisperModelLoaded && !dirty)}
        />
        <Health
          label="Ollama reachable and model accepted"
          ok={Boolean(testResult?.ollamaReachable && testResult.ollamaModelAccepted && !dirty)}
        />
        <Health
          label="Selected system voice is available"
          ok={Boolean(testResult?.ttsVoiceAvailable && !dirty)}
        />
      </div>

      {errors[0] ? <p className="local-config-error" role="alert">{errors[0]}</p> : null}
      {testResult?.message && !dirty ? <p className="key-test-row">{testResult.message}</p> : null}
      <div className="button-row">
        <button onClick={onSave} disabled={saving || testing || errors.length > 0}>
          {saving ? "Saving" : "Save local settings"}
        </button>
        <button
          className="primary"
          onClick={onTest}
          disabled={saving || testing || dirty || errors.length > 0}
        >
          {testing ? "Testing local pipeline" : "Test local pipeline"}
        </button>
        <button
          onClick={onPreview}
          disabled={
            saving || testing || previewing || previewDisabled || dirty || errors.length > 0
          }
        >
          {previewing ? "Playing voice" : "Test selected voice"}
        </button>
      </div>
      <p className="local-text-only-note">
        Voice audio uses the translated output device and left/right channel selected in Audio.
      </p>
    </div>
  );
}

function NumberField({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: string) => void;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <input
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(event) => onChange(event.currentTarget.value)}
      />
    </label>
  );
}

function Health({ label, ok }: { label: string; ok: boolean }) {
  return (
    <div className={`local-health ${ok ? "ok" : "pending"}`}>
      <span aria-hidden="true">{ok ? "✓" : "○"}</span>
      <span>{label}</span>
    </div>
  );
}
