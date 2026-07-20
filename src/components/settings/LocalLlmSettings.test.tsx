import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { axe } from "vitest-axe";
import {
  LocalLlmSettings,
  defaultLocalTranslationConfig,
  validateLocalTranslationDraft,
} from "./LocalLlmSettings";
import type { LocalTranslationConfigDraft } from "../../types";

const whisperModels = [
  {
    id: "small-q5_1",
    label: "Small Q5",
    description: "Good Japanese accuracy without the full model size.",
    fileName: "ggml-small-q5_1.bin",
    sizeMib: 181,
    recommended: true,
  },
];

describe("LocalLlmSettings", () => {
  it("requires Ollama and Whisper models before save or test", () => {
    expect(validateLocalTranslationDraft(defaultLocalTranslationConfig)).toEqual([
      "Choose an available local voice.",
      "Choose a Whisper GGML model file.",
    ]);
  });

  it("requires managed VieNeu setup instead of a bridge URL", () => {
    expect(
      validateLocalTranslationDraft({
        ...defaultLocalTranslationConfig,
        ttsProvider: "vieneu",
        vieneuBaseUrl: "https://example.com",
        modelPath: "C:\\models\\ggml-small.bin",
      }, {
        phase: "not_installed",
        runtimeAvailable: true,
        modelInstalled: false,
        running: false,
        modelVersion: "v3-turbo-int8-2026-07",
        installedBytes: 0,
        totalBytes: 256068309,
        message: "Install VieNeu-TTS.",
      }),
    ).toContain("Install VieNeu-TTS before choosing a neural voice.");
  });

  it("offers one-click managed VieNeu installation", async () => {
    const user = userEvent.setup();
    const onInstall = vi.fn();
    render(
      <LocalLlmSettings
        draft={{ ...defaultLocalTranslationConfig, ttsProvider: "vieneu" }}
        dirty={false}
        saving={false}
        testing={false}
        testResult={null}
        voices={[]}
        previewing={false}
        whisperModels={whisperModels}
        selectedWhisperModelId="small-q5_1"
        whisperDownload={null}
        whisperDownloading={false}
        vieneuRuntime={{
          phase: "not_installed",
          runtimeAvailable: true,
          modelInstalled: false,
          running: false,
          modelVersion: "v3-turbo-int8-2026-07",
          installedBytes: 0,
          totalBytes: 256068309,
          message: "Install VieNeu-TTS to continue.",
        }}
        onChange={() => undefined}
        onSave={() => undefined}
        onTest={() => undefined}
        onPreview={() => undefined}
        onWhisperModelSelect={() => undefined}
        onWhisperDownload={() => undefined}
        onVieNeuInstall={onInstall}
      />,
    );

    expect(screen.queryByText("VieNeu bridge URL")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Install VieNeu-TTS" }));
    expect(onInstall).toHaveBeenCalledTimes(1);
  });

  it("keeps edits local, clears readiness, and remains accessible", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    const onTest = vi.fn();

    function Harness() {
      const [draft, setDraft] = useState<LocalTranslationConfigDraft>({
        ...defaultLocalTranslationConfig,
        model: "qwen2.5:7b",
        modelPath: "C:\\models\\ggml-small.bin",
        voiceId: "vi-voice",
      });
      const [dirty, setDirty] = useState(false);
      return (
        <LocalLlmSettings
          draft={draft}
          dirty={dirty}
          saving={false}
          testing={false}
          testResult={
            dirty
              ? null
              : {
                  ok: true,
                  message: "Ready",
                  model: draft.model,
                  endpoint: "http://localhost:11434/api/chat",
                  whisperModelReadable: true,
                  whisperModelLoaded: true,
                  ollamaReachable: true,
                  ollamaModelAccepted: true,
                  ttsVoiceAvailable: true,
                }
          }
          onChange={(next) => {
            setDraft(next);
            setDirty(true);
          }}
          onSave={onSave}
          onTest={onTest}
          voices={[{ id: "vi-voice", name: "Vietnamese", language: "vi-VN" }]}
          previewing={false}
          onPreview={() => undefined}
          whisperModels={whisperModels}
          selectedWhisperModelId="small-q5_1"
          whisperDownload={null}
          whisperDownloading={false}
          onWhisperModelSelect={() => undefined}
          onWhisperDownload={() => undefined}
        />
      );
    }

    const { container } = render(<Harness />);
    expect(screen.getByText("Ready", { selector: ".panel-state" })).toBeInTheDocument();
    await user.type(screen.getByLabelText("Installed TranslateGemma model"), "-fast");
    expect(screen.getByText("Unsaved")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Test local pipeline" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Save local settings" }));
    expect(onSave).toHaveBeenCalledTimes(1);
    expect((await axe(container)).violations).toEqual([]);
  });

  it("shows partial health when Whisper passes but Ollama is offline", () => {
    render(
      <LocalLlmSettings
        draft={{
          ...defaultLocalTranslationConfig,
          model: "qwen2.5:7b",
          modelPath: "C:\\models\\ggml-small.bin",
          voiceId: "vi-voice",
        }}
        dirty={false}
        saving={false}
        testing={false}
        testResult={{
          ok: false,
          message: "Could not connect to Ollama.",
          model: "qwen2.5:7b",
          endpoint: "http://localhost:11434/api/chat",
          whisperModelReadable: true,
          whisperModelLoaded: true,
          ollamaReachable: false,
          ollamaModelAccepted: false,
          ttsVoiceAvailable: true,
        }}
        onChange={() => undefined}
        onSave={() => undefined}
        onTest={() => undefined}
        voices={[{ id: "vi-voice", name: "Vietnamese", language: "vi-VN" }]}
        previewing={false}
        onPreview={() => undefined}
        whisperModels={whisperModels}
        selectedWhisperModelId="small-q5_1"
        whisperDownload={null}
        whisperDownloading={false}
        onWhisperModelSelect={() => undefined}
        onWhisperDownload={() => undefined}
      />,
    );

    expect(screen.getByText("Whisper model readable and loaded").closest("div")).toHaveClass("ok");
    expect(screen.getByText("Ollama reachable and model accepted").closest("div")).not.toHaveClass("ok");
    expect(screen.getByText("Could not connect to Ollama.")).toBeInTheDocument();
  });

  it("offers a managed Whisper download and reports progress", async () => {
    const user = userEvent.setup();
    const onDownload = vi.fn();
    const baseProps = {
      draft: { ...defaultLocalTranslationConfig, voiceId: "vi-voice" },
      dirty: false,
      saving: false,
      testing: false,
      testResult: null,
      onChange: () => undefined,
      onSave: () => undefined,
      onTest: () => undefined,
      voices: [],
      previewing: false,
      onPreview: () => undefined,
      whisperModels,
      selectedWhisperModelId: "small-q5_1",
      onWhisperModelSelect: () => undefined,
      onWhisperDownload: onDownload,
    };
    const { rerender } = render(
      <LocalLlmSettings
        {...baseProps}
        whisperDownload={null}
        whisperDownloading={false}
      />,
    );

    expect(screen.getByLabelText("Whisper model to download")).toHaveValue("small-q5_1");
    expect(screen.getByText(/181 MiB/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Download model" }));
    expect(onDownload).toHaveBeenCalledTimes(1);

    rerender(
      <LocalLlmSettings
        {...baseProps}
        whisperDownload={{
          modelId: "small-q5_1",
          fileName: "ggml-small-q5_1.bin",
          downloadedBytes: 95,
          totalBytes: 190,
          percent: 50,
          status: "downloading",
          message: "Downloading model…",
        }}
        whisperDownloading
      />,
    );
    expect(screen.getByRole("button", { name: "Downloading 50%" })).toBeDisabled();
    expect(screen.getByRole("progressbar", { name: "Downloading ggml-small-q5_1.bin" })).toHaveValue(50);
  });
});
