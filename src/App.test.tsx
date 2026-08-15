import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { axe } from "vitest-axe";
import App, { resolveApplicationRoute } from "./App";
import { defaultLocalTranslationConfig } from "./components/settings/LocalLlmSettings";
import { ApplicationThemeProvider } from "./ui/ThemeProvider";

afterEach(() => {
  clearMocks();
  delete (globalThis as typeof globalThis & { isTauri?: boolean }).isTauri;
});

describe("application routes", () => {
  it.each([
    ["", "main"],
    ["?overlay=transparent", "transparent"],
    ["?overlay=look-help", "look-help"],
    ["?overlay=unknown", "main"],
  ] as const)("resolves %s to %s", (search, expected) => {
    expect(resolveApplicationRoute(search)).toBe(expected);
  });

  it.each([
    ["/", "heading", "Choose how to translate"],
    ["/?overlay=transparent", "main", "Look Through"],
    ["/?overlay=look-help", "main", "Look & Help"],
  ])("renders %s without the native runtime", async (path, role, name) => {
    window.history.replaceState({}, "", path);
    const { container } = render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );

    expect(screen.getByRole(role, { name })).toBeInTheDocument();
    expect((await axe(container)).violations).toEqual([]);
  });

  it("preserves unsaved translation settings while changing destinations", async () => {
    const user = userEvent.setup();
    const { unmount } = render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );

    await user.click(screen.getByRole("button", { name: /Cloud API/ }));
    await user.click(screen.getByRole("tab", { name: "Translation" }));
    const keyInput = screen.getByPlaceholderText("Google Live Translation API key");
    await user.type(keyInput, "unsaved-key");
    await user.click(screen.getByRole("tab", { name: "Summary" }));
    await user.click(screen.getByRole("tab", { name: "Translation" }));

    expect(screen.getByDisplayValue("unsaved-key")).toBeInTheDocument();
    unmount();
  });

  it("opens a focused local workspace and returns to the chooser", async () => {
    const user = userEvent.setup();
    window.history.replaceState({}, "", "/");
    render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );

    await user.click(screen.getByRole("button", { name: /Local Whisper/ }));

    expect(screen.getByText("Local setup needed")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Local LLM" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Translation" })).not.toBeInTheDocument();
    expect(screen.getByLabelText("Source language")).toHaveValue("ja");
    await user.selectOptions(screen.getByLabelText("Source language"), "en");
    await user.selectOptions(screen.getByLabelText("Target language"), "ja");
    expect(screen.getByLabelText("Source language")).toHaveValue("en");
    expect(screen.getByLabelText("Target language")).toHaveValue("ja");

    await user.click(screen.getByRole("button", { name: "Change translation mode" }));
    expect(screen.getByRole("heading", { name: "Choose how to translate" })).toBeInTheDocument();
  });

  it("opens audio settings from the live empty state", async () => {
    const user = userEvent.setup();
    render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );

    await user.click(screen.getByRole("button", { name: /Cloud API/ }));
    await user.click(screen.getByRole("button", { name: "Open audio settings" }));

    expect(screen.getByRole("dialog", { name: "Audio settings" })).toBeInTheDocument();
  });

  it("starts a local session from a valid saved config without a transient health test", async () => {
    const user = userEvent.setup();
    let startedConfig: Record<string, unknown> | null = null;
    Object.assign(globalThis, { isTauri: true });
    mockIPC(
      (command, args) => {
        switch (command) {
          case "list_audio_devices":
            return {
              inputs: [
                {
                  id: "input:0:Microphone",
                  name: "Microphone",
                  kind: "input",
                  isDefault: true,
                  maxChannels: 1,
                },
              ],
              outputs: [
                {
                  id: "output:0:Headphones",
                  name: "Headphones",
                  kind: "output",
                  isDefault: true,
                  maxChannels: 2,
                },
              ],
            };
          case "get_app_status":
            return { sessionStatus: "idle", hasApiKey: false, transcriptCount: 0 };
          case "get_transcript_snapshot":
          case "list_llm_profiles":
          case "list_whisper_models":
            return [];
          case "get_local_translation_config":
            return {
              schemaVersion: 1,
              ...defaultLocalTranslationConfig,
              modelPath: "C:\\models\\ggml-small.bin",
              voiceId: "vi-voice",
            };
          case "get_whisper_model_dir":
            return "/Users/test/.bakatrans/whisper";
          case "list_local_tts_voices":
            return [{ id: "vi-voice", name: "Vietnamese", language: "vi-VN" }];
          case "get_vieneu_runtime_status":
            return null;
          case "translation_credential_status":
            return { provider: "local_whisper", hasApiKey: false };
          case "start_session":
            startedConfig = (args as unknown as { config: Record<string, unknown> }).config;
            return undefined;
          default:
            return undefined;
        }
      },
      { shouldMockEvents: true },
    );

    const view = render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );
    await user.click(screen.getByRole("button", { name: /Local Whisper/ }));

    const start = await screen.findByRole("button", { name: "Start" });
    await waitFor(() => expect(start).toBeEnabled());
    await user.click(start);
    await waitFor(() => expect(startedConfig).not.toBeNull());
    expect(startedConfig).toMatchObject({
      translationProvider: "local_whisper",
      inputDeviceId: "input:0:Microphone",
    });
    view.unmount();
    await Promise.resolve();
  });

  it("rehydrates the backend transcript after changing workspaces", async () => {
    const user = userEvent.setup();
    Object.assign(globalThis, { isTauri: true });
    mockIPC(
      (command) => {
        switch (command) {
          case "list_audio_devices":
            return { inputs: [], outputs: [] };
          case "get_app_status":
            return { sessionStatus: "idle", hasApiKey: false, transcriptCount: 1 };
          case "get_transcript_snapshot":
            return [
              {
                id: "persisted-utterance",
                timestampMs: 1,
                sourceText: "継続する会話",
                translatedText: "Cuộc trò chuyện tiếp tục",
                status: "final",
                revision: 1,
                updateMode: "snapshot",
              },
            ];
          case "list_llm_profiles":
            return [];
          case "get_local_translation_config":
            return { schemaVersion: 2, ...defaultLocalTranslationConfig };
          case "list_local_tts_voices":
            return [];
          case "list_whisper_models":
            return [];
          case "get_whisper_model_dir":
            return "/Users/test/.bakatrans/whisper";
          case "translation_credential_status":
            return {
              provider: "google_live_translate",
              hasApiKey: false,
            };
          default:
            return undefined;
        }
      },
      { shouldMockEvents: true },
    );

    const view = render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );
    await user.click(screen.getByRole("button", { name: /Cloud API/ }));
    expect(await screen.findByText("Cuộc trò chuyện tiếp tục")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Change translation mode" }));
    await user.click(screen.getByRole("button", { name: /Local Whisper/ }));
    expect(await screen.findByText("Cuộc trò chuyện tiếp tục")).toBeInTheDocument();

    view.unmount();
  });

  it.each([
    ["/?overlay=transparent", "Look Through settings"],
    ["/?overlay=look-help", "Look & Help settings"],
  ])("exposes settings disclosure on %s", async (path, accessibleName) => {
    const user = userEvent.setup();
    window.history.replaceState({}, "", path);
    render(
      <ApplicationThemeProvider>
        <App />
      </ApplicationThemeProvider>,
    );

    await user.click(screen.getByRole("button", { name: "Show settings" }));

    expect(screen.getByRole("region", { name: accessibleName })).toBeInTheDocument();
  });
});
