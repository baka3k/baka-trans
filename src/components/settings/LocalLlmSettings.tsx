import type {
  HyMtModelProgress,
  HyMtModelStatus,
  LocalTranslationConfigDraft,
  LocalTranslationTestResult,
  LocalVoice,
  TranslationEngineTestResult,
  VieNeuRuntimeProgress,
  VieNeuRuntimeStatus,
  WhisperModelDownloadProgress,
  WhisperModelOption,
} from "../../types";

export const defaultLocalTranslationConfig: LocalTranslationConfigDraft = {
  translationEngine: "huggingface_offline",
  openaiBaseUrl: "",
  openaiModel: "",
  openaiTimeoutSeconds: 30,
  openaiTemperature: 0,
  openaiMaxOutputTokens: 256,
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
  ttsProvider: "system",
  vieneuBaseUrl: "http://127.0.0.1:23334",
  vieneuStyle: "tu_nhien",
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
  engineTesting?: boolean;
  engineTest?: TranslationEngineTestResult | null;
  voices: LocalVoice[];
  voicesLoading?: boolean;
  previewing: boolean;
  previewDisabled?: boolean;
  whisperModels: WhisperModelOption[];
  selectedWhisperModelId: string;
  whisperDownload: WhisperModelDownloadProgress | null;
  whisperDownloading: boolean;
  vieneuRuntime?: VieNeuRuntimeStatus | null;
  vieneuProgress?: VieNeuRuntimeProgress | null;
  vieneuBusy?: boolean;
  hyMtModel?: HyMtModelStatus | null;
  hyMtProgress?: HyMtModelProgress | null;
  hyMtBusy?: boolean;
  onChange: (draft: LocalTranslationConfigDraft) => void;
  onSave: () => void;
  onTest: () => void;
  onTestEngine?: () => void;
  onPreview: () => void;
  onRefreshVoices?: () => void;
  onWhisperModelSelect: (modelId: string) => void;
  onWhisperDownload: () => void;
  onVieNeuInstall?: () => void;
  onVieNeuCancel?: () => void;
  onVieNeuRestart?: () => void;
  onHyMtInstall?: () => void;
  onHyMtCancel?: () => void;
}

export function validateLocalTranslationDraft(
  draft: LocalTranslationConfigDraft,
  vieneuRuntime?: VieNeuRuntimeStatus | null,
): string[] {
  const errors: string[] = [];
  if (draft.translationEngine === "openai_compatible" && !/^https?:\/\//i.test((draft.openaiBaseUrl ?? "").trim())) {
    errors.push("Enter an http or https OpenAI-compatible server URL.");
  }
  if (draft.translationEngine === "openai_compatible" && !(draft.openaiModel ?? "").trim()) {
    errors.push("Enter an OpenAI-compatible model name.");
  }
  if (draft.ttsProvider === "vieneu" && vieneuRuntime && !vieneuRuntime.modelInstalled) {
    errors.push("Install VieNeu-TTS before choosing a neural voice.");
  } else if (!draft.voiceId.trim()) {
    errors.push("Choose an available local voice.");
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
  engineTesting = false,
  engineTest = null,
  voices,
  voicesLoading = false,
  previewing,
  previewDisabled = false,
  whisperModels,
  selectedWhisperModelId,
  whisperDownload,
  whisperDownloading,
  vieneuRuntime = null,
  vieneuProgress = null,
  vieneuBusy = false,
  hyMtModel = null,
  hyMtProgress = null,
  hyMtBusy = false,
  onChange,
  onSave,
  onTest,
  onTestEngine = () => undefined,
  onPreview,
  onRefreshVoices = () => undefined,
  onWhisperModelSelect,
  onWhisperDownload,
  onVieNeuInstall = () => undefined,
  onVieNeuCancel = () => undefined,
  onVieNeuRestart = () => undefined,
  onHyMtInstall = () => undefined,
  onHyMtCancel = () => undefined,
}: LocalLlmSettingsProps) {
  const errors = validateLocalTranslationDraft(draft, vieneuRuntime);
  const engineFieldsInvalid =
    draft.translationEngine === "openai_compatible" &&
    (!/^https?:\/\//i.test((draft.openaiBaseUrl ?? "").trim()) || !draft.openaiModel.trim());
  const engineHealthy = Boolean(
    !dirty &&
      ((testResult?.engineReachable && testResult.engineAccepted) ||
        (engineTest?.reachable && engineTest.accepted)),
  );
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
          <h2>Local translation</h2>
          <p>Selected source → Whisper → selected translator → selected local voice</p>
        </div>
        <span className={`panel-state ${testResult?.ok && !dirty ? "ok" : ""}`}>
          {testResult?.ok && !dirty ? "Ready" : dirty ? "Unsaved" : "Test required"}
        </span>
      </div>

      <fieldset className="local-config-group">
        <legend>Translation engine</legend>
        <label className="field">
          <span>Engine</span>
          <select
            value={draft.translationEngine}
            onChange={(event) => update({ translationEngine: event.currentTarget.value as LocalTranslationConfigDraft["translationEngine"] })}
          >
            <option value="huggingface_offline">Offline Hy-MT2 1.8B</option>
            <option value="openai_compatible">OpenAI-compatible API</option>
          </select>
        </label>
        {draft.translationEngine === "huggingface_offline" ? (
          <>
            <HyMtModelCard
              status={hyMtModel}
              progress={hyMtProgress}
              busy={hyMtBusy}
              onInstall={onHyMtInstall}
              onCancel={onHyMtCancel}
            />
            <p className="local-text-only-note">Managed Hy-MT2 runs from verified local files. The quality gate remains under CAUTION, so the offline engine is not selectable for live translation yet.</p>
          </>
        ) : <>
        <label className="field">
          <span>Server URL</span>
          <input
            value={draft.openaiBaseUrl}
            placeholder="https://api.example.com/v1"
            onChange={(event) => update({ openaiBaseUrl: event.currentTarget.value })}
          />
          <small>Meeting text is sent to this endpoint. Non-local endpoints should use HTTPS.</small>
        </label>
        <label className="field">
          <span>Model</span>
          <input
            value={draft.openaiModel}
            placeholder="gpt-4o-mini"
            onChange={(event) => update({ openaiModel: event.currentTarget.value })}
          />
        </label>
        <div className="field-grid two">
          <NumberField
            label="Timeout (seconds)"
            min={5}
            max={300}
            step={1}
            value={draft.openaiTimeoutSeconds}
            onChange={(value) => update({ openaiTimeoutSeconds: number(value, draft.openaiTimeoutSeconds) })}
          />
          <NumberField
            label="Maximum output tokens"
            min={32}
            max={2048}
            step={1}
            value={draft.openaiMaxOutputTokens}
            onChange={(value) => update({ openaiMaxOutputTokens: number(value, draft.openaiMaxOutputTokens) })}
          />
          <NumberField
            label="Temperature"
            min={0}
            max={1}
            step={0.1}
            value={draft.openaiTemperature}
            onChange={(value) => update({ openaiTemperature: number(value, draft.openaiTemperature) })}
          />
        </div>
        </>}
        <button
          type="button"
          onClick={onTestEngine}
          disabled={saving || testing || engineTesting || dirty || engineFieldsInvalid}
        >
          {engineTesting ? "Testing engine" : "Test translation engine"}
        </button>
        {engineTest && !dirty ? (
          <p className={`key-test-row ${engineTest.accepted ? "" : "engine-test-failed"}`}>
            {engineTest.message}
          </p>
        ) : null}
      </fieldset>

      <fieldset className="local-config-group">
        <legend>Translated voice</legend>
        <label className="field">
          <span>Speech engine</span>
          <select
            value={draft.ttsProvider}
            onChange={(event) =>
              update({
                ttsProvider: event.currentTarget.value as LocalTranslationConfigDraft["ttsProvider"],
                voiceId: "",
              })
            }
          >
            <option value="system">System TTS</option>
            <option value="vieneu">VieNeu-TTS v3 Turbo</option>
          </select>
        </label>
        {draft.ttsProvider === "vieneu" ? (
          <>
            <VieNeuRuntimeCard
              status={vieneuRuntime}
              progress={vieneuProgress}
              busy={vieneuBusy}
              onInstall={onVieNeuInstall}
              onCancel={onVieNeuCancel}
              onRestart={onVieNeuRestart}
            />
            <label className="field">
              <span>Reading style</span>
              <select
                value={draft.vieneuStyle}
                onChange={(event) =>
                  update({
                    vieneuStyle: event.currentTarget.value as LocalTranslationConfigDraft["vieneuStyle"],
                  })
                }
              >
                <option value="tu_nhien">Natural</option>
                <option value="tin_tuc">News</option>
                <option value="doc_truyen">Storytelling</option>
              </select>
            </label>
          </>
        ) : null}
        {draft.ttsProvider === "system" || vieneuRuntime?.modelInstalled ? (
          <>
            <label className="field">
              <span>{draft.ttsProvider === "vieneu" ? "VieNeu preset voice" : "Installed system voice"}</span>
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
              <small>
                {draft.ttsProvider === "vieneu"
                  ? "Preset voices are loaded from the private app-managed runtime."
                  : "Choose a system voice that matches the target language selected in the live controls."}
              </small>
            </label>
            <button type="button" onClick={onRefreshVoices} disabled={voicesLoading || saving || testing || vieneuBusy}>
              {voicesLoading ? "Refreshing voices" : "Refresh voice list"}
            </button>
          </>
        ) : null}
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
        <div className="whisper-download-card">
          <div className="whisper-download-copy">
            <strong>Download a speech model</strong>
            <span>Stored privately in ~/.bakatrans/whisper and selected automatically.</span>
          </div>
          <div className="whisper-download-controls">
            <label className="field">
              <span>Model</span>
              <select
                aria-label="Whisper model to download"
                value={selectedWhisperModelId}
                disabled={whisperDownloading || whisperModels.length === 0}
                onChange={(event) => onWhisperModelSelect(event.currentTarget.value)}
              >
                {whisperModels.length === 0 ? <option value="">Loading models…</option> : null}
                {whisperModels.map((model) => (
                  <option value={model.id} key={model.id}>
                    {model.label} · {model.sizeMib} MiB{model.recommended ? " · Recommended" : ""}
                  </option>
                ))}
              </select>
              <small>
                {whisperModels.find((model) => model.id === selectedWhisperModelId)?.description ??
                  "Choose a multilingual Whisper model."}
              </small>
            </label>
            <button
              type="button"
              className="primary whisper-download-button"
              onClick={onWhisperDownload}
              disabled={whisperDownloading || saving || testing || !selectedWhisperModelId}
            >
              {whisperDownloading
                ? whisperDownload?.percent !== undefined
                  ? `Downloading ${whisperDownload.percent}%`
                  : "Downloading…"
                : "Download model"}
            </button>
          </div>
          {whisperDownload ? (
            <div className={`whisper-download-status ${whisperDownload.status}`} aria-live="polite">
              {whisperDownload.status === "downloading" ? (
                <progress
                  max={100}
                  value={whisperDownload.percent}
                  aria-label={`Downloading ${whisperDownload.fileName}`}
                />
              ) : null}
              <span>{whisperDownload.message}</span>
            </div>
          ) : null}
        </div>
        <label className="field">
          <span>GGML model path</span>
          <input
            value={draft.modelPath}
            placeholder="~/.bakatrans/whisper/ggml-small-q5_1.bin"
            onChange={(event) => update({ modelPath: event.currentTarget.value })}
          />
          <small>Defaults to the per-OS Whisper cache folder. The download above fills it; you can still enter another absolute GGML path.</small>
        </label>
        <div className="field-grid two">
          <label className="field">
            <span>Language</span>
            <input value="Selected in live controls" readOnly aria-readonly="true" />
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
          label="Translation engine reachable and accepted"
          ok={engineHealthy}
        />
        <Health
          label="Selected local voice is available"
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

function VieNeuRuntimeCard({
  status,
  progress,
  busy,
  onInstall,
  onCancel,
  onRestart,
}: {
  status: VieNeuRuntimeStatus | null;
  progress: VieNeuRuntimeProgress | null;
  busy: boolean;
  onInstall: () => void;
  onCancel: () => void;
  onRestart: () => void;
}) {
  const phase = progress?.phase ?? status?.phase ?? "not_installed";
  const active = phase === "downloading" || phase === "verifying";
  const installed = Boolean(status?.modelInstalled);
  const completedBytes = progress?.downloadedBytes ?? status?.installedBytes ?? 0;
  const totalBytes = progress?.totalBytes ?? status?.totalBytes ?? 0;
  const percent = progress?.percent ??
    (status?.totalBytes ? Math.round((status.installedBytes / status.totalBytes) * 100) : 0);
  const message = progress?.message ?? status?.message ?? "Checking the managed VieNeu runtime…";
  const actionLabel = phase === "paused" ? "Resume setup" : phase === "repair_needed" || phase === "error" ? "Repair" : "Install VieNeu-TTS";

  return (
    <div className={`vieneu-runtime-card ${installed ? "installed" : ""}`} aria-live="polite">
      <div className="vieneu-runtime-heading">
        <div>
          <strong>Managed VieNeu-TTS</strong>
          <span>Runs privately on this computer; no Python or terminal setup required.</span>
        </div>
        <span className="vieneu-runtime-badge">{phase.replace(/_/g, " ")}</span>
      </div>
      <p>{message}</p>
      {totalBytes ? (
        <p>Model download: {formatMib(totalBytes)} · pinned ONNX/int8 CPU runtime</p>
      ) : null}
      {(active || phase === "paused") && totalBytes ? (
        <div className="vieneu-runtime-progress">
          <progress max={100} value={percent} aria-label="VieNeu-TTS setup progress" />
          <span>{percent}% · {formatMib(completedBytes)} / {formatMib(totalBytes)}</span>
        </div>
      ) : null}
      <div className="button-row vieneu-runtime-actions">
        {active ? (
          <button type="button" onClick={onCancel}>Pause download</button>
        ) : installed && phase !== "repair_needed" && phase !== "error" ? (
          <button type="button" onClick={onRestart} disabled={busy || phase === "starting"}>
            {phase === "starting" ? "Starting VieNeu-TTS" : status?.running ? "Restart VieNeu-TTS" : "Start VieNeu-TTS"}
          </button>
        ) : (
          <button type="button" className="primary" onClick={onInstall} disabled={busy || phase === "unsupported"}>
            {busy ? "Preparing setup" : actionLabel}
          </button>
        )}
      </div>
    </div>
  );
}

function HyMtModelCard({
  status,
  progress,
  busy,
  onInstall,
  onCancel,
}: {
  status: HyMtModelStatus | null;
  progress: HyMtModelProgress | null;
  busy: boolean;
  onInstall: () => void;
  onCancel: () => void;
}) {
  const phase = progress?.phase ?? status?.phase ?? "not_installed";
  const active = phase === "downloading" || phase === "verifying";
  const installed = Boolean(status?.modelInstalled);
  const downloadedBytes = progress?.downloadedBytes ?? (installed ? status?.totalBytes ?? 0 : 0);
  const totalBytes = progress?.totalBytes ?? status?.totalBytes ?? 0;
  const percent = progress?.percent ??
    (totalBytes > 0 ? Math.round((downloadedBytes / totalBytes) * 100) : 0);
  const message = progress?.message ?? status?.message ?? "Checking the managed Hy-MT2 model…";
  const actionLabel = phase === "paused"
    ? "Resume download"
    : phase === "error" ? "Retry install" : "Install Hy-MT2 model";

  return (
    <div className={`vieneu-runtime-card ${installed ? "installed" : ""}`} aria-live="polite">
      <div className="vieneu-runtime-heading">
        <div>
          <strong>Managed Hy-MT2 1.8B</strong>
          <span>Download the pinned, SHA-256 verified offline translation model (~3.8 GiB).</span>
        </div>
        <span className="vieneu-runtime-badge">{phase.replace(/_/g, " ")}</span>
      </div>
      <p>{message}</p>
      {(active || phase === "paused" || installed) && totalBytes ? (
        <div className="vieneu-runtime-progress">
          <progress max={100} value={percent} aria-label="Hy-MT2 model setup progress" />
          <span>{percent}% · {formatMib(downloadedBytes)} / {formatMib(totalBytes)}</span>
        </div>
      ) : null}
      <div className="button-row vieneu-runtime-actions">
        {active ? (
          <button type="button" onClick={onCancel}>Pause download</button>
        ) : installed ? (
          <span className="vieneu-runtime-badge">verified · {status?.modelRevision.slice(0, 7) ?? ""}</span>
        ) : (
          <button type="button" className="primary" onClick={onInstall} disabled={busy || phase === "unsupported"}>
            {busy ? "Preparing setup" : actionLabel}
          </button>
        )}
      </div>
    </div>
  );
}

function formatMib(bytes: number) {
  return `${(bytes / 1024 / 1024).toFixed(bytes > 0 ? 1 : 0)} MiB`;
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
