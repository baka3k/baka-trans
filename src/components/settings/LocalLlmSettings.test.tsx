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

describe("LocalLlmSettings", () => {
  it("requires Ollama and Whisper models before save or test", () => {
    expect(validateLocalTranslationDraft(defaultLocalTranslationConfig)).toEqual([
      "Choose an installed local voice.",
      "Choose a Whisper GGML model file.",
    ]);
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
        />
      );
    }

    const { container } = render(<Harness />);
    expect(screen.getByText("Ready", { selector: ".panel-state" })).toBeInTheDocument();
    await user.type(screen.getByLabelText("Installed Gemma model"), "-fast");
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
      />,
    );

    expect(screen.getByText("Whisper model readable and loaded").closest("div")).toHaveClass("ok");
    expect(screen.getByText("Ollama reachable and model accepted").closest("div")).not.toHaveClass("ok");
    expect(screen.getByText("Could not connect to Ollama.")).toBeInTheDocument();
  });
});
