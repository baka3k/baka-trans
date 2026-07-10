import { describe, expect, it } from "vitest";
import {
  MEETING_SUMMARY_CUSTOM_PROMPT_MAX_CHARS,
  buildMeetingSummaryConfig,
  deriveConversationItems,
  deriveSourceSignalState,
  deriveTranslationActivity,
  meetingSummaryPromptPresetDescription,
  meetingSummaryPromptPresets,
  mergeTranscriptDelta,
  renderTranscript,
  selectMeetingSummaryPromptPreset,
  validateMeetingSummaryCustomPrompt,
  validateLlmProfileDraft,
} from "./transcript";
import type { MeetingSummaryResult, TranscriptItem } from "./types";

const base: TranscriptItem = {
  id: "1",
  timestampMs: 1,
  sourceText: "Hello",
  translatedText: "",
  status: "partial",
};

describe("transcript helpers", () => {
  it("merges adjacent partial deltas", () => {
    const merged = mergeTranscriptDelta([base], {
      ...base,
      id: "2",
      sourceText: " world",
    });

    expect(merged).toHaveLength(1);
    expect(merged[0].sourceText).toBe("Hello world");
  });

  it("repairs missing spaces between streamed word chunks", () => {
    const merged = mergeTranscriptDelta(
      [
        {
          ...base,
          sourceText: "To test your",
          translatedText: "De kiem tra",
        },
      ],
      {
        ...base,
        id: "2",
        sourceText: "call quality,",
        translatedText: "",
      },
    );

    const translated = mergeTranscriptDelta(merged, {
      ...base,
      id: "3",
      sourceText: "",
      translatedText: "chat luong cuoc goi",
    });

    expect(translated).toHaveLength(1);
    expect(translated[0].sourceText).toBe("To test your call quality,");
    expect(translated[0].translatedText).toBe("De kiem tra chat luong cuoc goi");
  });

  it("merges final translation-only deltas into the current row", () => {
    const merged = mergeTranscriptDelta([base], {
      ...base,
      id: "2",
      sourceText: "",
      translatedText: "Xin chao",
      status: "final",
    });

    expect(merged).toHaveLength(1);
    expect(merged[0]).toMatchObject({
      sourceText: "Hello",
      translatedText: "Xin chao",
      status: "final",
    });
  });

  it("starts a new translated line after sentence boundaries", () => {
    const merged = mergeTranscriptDelta(
      [
        {
          ...base,
          translatedText: "Good morning.",
        },
      ],
      {
        ...base,
        id: "2",
        sourceText: "",
        translatedText: " We can start now.",
      },
    );

    expect(merged).toHaveLength(1);
    expect(merged[0].translatedText).toBe("Good morning.\nWe can start now.");
  });

  it("renders markdown exports", () => {
    const output = renderTranscript(
      [{ ...base, translatedText: "Xin chao", status: "final" }],
      "markdown",
    );

    expect(output).toContain("# Baka Trans Transcript");
    expect(output).toContain("Xin chao");
  });

  it("includes meeting notes in markdown exports", () => {
    const notes: MeetingSummaryResult = {
      id: "notes",
      createdAtMs: 2,
      sourceItemIds: ["1"],
      summary: "The team agreed to ship.",
      decisions: ["Ship on Friday"],
      actionItems: [
        {
          text: "Prepare release notes",
          owner: "Mai",
          dueDate: "Friday",
          sourceItemIds: ["1"],
        },
      ],
      blockers: [],
      importantPoints: ["Customer demo is Monday"],
      model: "gpt-test",
      providerProfileId: "profile",
      status: "complete",
    };

    const output = renderTranscript([{ ...base, status: "final" }], "markdown", notes);

    expect(output).toContain("# Meeting Notes");
    expect(output).toContain("Prepare release notes");
  });

  it("validates provider profiles for required model and base URL", () => {
    expect(
      validateLlmProfileDraft({
        name: "Local",
        kind: "openai_compatible",
        model: "",
        baseUrl: "",
      }),
    ).toEqual(["Model is required.", "Base URL is required for this provider."]);
  });

  it("builds default meeting summary config", () => {
    const config = buildMeetingSummaryConfig("profile");

    expect(config.providerProfileId).toBe("profile");
    expect(config.transcriptScope).toBe("both");
    expect(config.outputLanguage).toBe("Vietnamese");
    expect(config.promptPreset).toBe("balanced");
    expect(config.customSystemPrompt).toBe("");
    expect(config.sections.actionItems).toBe(true);
  });

  it("defines stable metadata for every meeting summary prompt preset", () => {
    expect(meetingSummaryPromptPresets.map((preset) => preset.id)).toEqual([
      "balanced",
      "professional",
      "gentle",
      "detailed",
      "timeline",
      "custom",
    ]);

    for (const preset of meetingSummaryPromptPresets) {
      expect(preset.label).not.toBe("");
      expect(preset.description).not.toBe("");
      expect(meetingSummaryPromptPresetDescription(preset.id)).toBe(preset.description);
    }
  });

  it("validates custom meeting summary instructions at the shared limit", () => {
    expect(validateMeetingSummaryCustomPrompt("custom", "   ")).toBe(
      "Enter custom summary instructions.",
    );
    expect(
      validateMeetingSummaryCustomPrompt(
        "custom",
        "x".repeat(MEETING_SUMMARY_CUSTOM_PROMPT_MAX_CHARS),
      ),
    ).toBeNull();
    expect(
      validateMeetingSummaryCustomPrompt(
        "custom",
        "x".repeat(MEETING_SUMMARY_CUSTOM_PROMPT_MAX_CHARS + 1),
      ),
    ).toContain("8,000 characters or fewer");
    expect(validateMeetingSummaryCustomPrompt("balanced", "")).toBeNull();
  });

  it("preserves custom instructions while switching summary prompt presets", () => {
    const customConfig = {
      ...buildMeetingSummaryConfig("profile"),
      promptPreset: "custom" as const,
      customSystemPrompt: "Focus on technical tradeoffs.",
    };

    const gentleConfig = selectMeetingSummaryPromptPreset(customConfig, "gentle");
    const restoredConfig = selectMeetingSummaryPromptPreset(gentleConfig, "custom");

    expect(gentleConfig.customSystemPrompt).toBe("Focus on technical tradeoffs.");
    expect(restoredConfig.customSystemPrompt).toBe("Focus on technical tradeoffs.");
  });

  it("classifies a fresh source signal above the threshold as receiving", () => {
    expect(
      deriveSourceSignalState(
        { inputDeviceId: "mic-1", peak: 0.12, rms: 0.06, receivedAtMs: 1000 },
        "mic-1",
        "listening",
        1400,
      ),
    ).toBe("receiving");
  });

  it("classifies a fresh low-level source signal as silent", () => {
    expect(
      deriveSourceSignalState(
        { inputDeviceId: "mic-1", peak: 0.01, rms: 0.004, receivedAtMs: 1000 },
        "mic-1",
        "listening",
        1200,
      ),
    ).toBe("silent");
  });

  it("marks active sessions stale when source events stop", () => {
    expect(
      deriveSourceSignalState(
        { inputDeviceId: "mic-1", peak: 0.2, rms: 0.1, receivedAtMs: 1000 },
        "mic-1",
        "translating",
        3501,
      ),
    ).toBe("stale");
  });

  it("ignores source events from a different input device", () => {
    expect(
      deriveSourceSignalState(
        { inputDeviceId: "mic-2", peak: 0.3, rms: 0.1, receivedAtMs: 1000 },
        "mic-1",
        "listening",
        1100,
      ),
    ).toBe("waiting");
  });

  it("derives conversation cards with source and translation together", () => {
    const items = deriveConversationItems([
      { ...base, translatedText: "Xin chao", status: "final", latencyMs: 740 },
    ]);

    expect(items[0]).toMatchObject({
      id: "1",
      sourceText: "Hello",
      translatedText: "Xin chao",
      status: "final",
      latencyMs: 740,
      speakerDisplayLabel: "Source",
      hasPendingTranslation: false,
    });
  });

  it("pairs source and translated sentences for readable display", () => {
    const [item] = deriveConversationItems([
      {
        ...base,
        sourceText:
          "To test your call quality, record a short message after the beep. Then your message will be played back to you.",
        translatedText:
          "De kiem tra chat luong cuoc goi, hay ghi am mot tin nhan ngan sau tieng bip. Sau do tin nhan cua ban se duoc phat lai cho ban.",
        status: "final",
      },
    ]);

    expect(item.sentencePairs).toEqual([
      {
        sourceText:
          "To test your call quality, record a short message after the beep.",
        translatedText:
          "De kiem tra chat luong cuoc goi, hay ghi am mot tin nhan ngan sau tieng bip.",
      },
      {
        sourceText: "Then your message will be played back to you.",
        translatedText: "Sau do tin nhan cua ban se duoc phat lai cho ban.",
      },
    ]);
  });

  it("marks source-only items as pending translation", () => {
    const items = deriveConversationItems([base]);

    expect(items[0].hasPendingTranslation).toBe(true);
  });

  it("uses optional speaker labels when available", () => {
    const items = deriveConversationItems([
      {
        ...base,
        speakerLabel: "Speaker 2",
        speakerSegmentId: "segment-2",
        speakerConfidence: 0.88,
      },
    ]);

    expect(items[0]).toMatchObject({
      speakerDisplayLabel: "Speaker 2",
      speakerSegmentId: "segment-2",
      speakerConfidence: 0.88,
    });
  });

  it("flags stale active translation as needing attention", () => {
    expect(deriveTranslationActivity("listening", undefined, "stale", 0)).toBe(
      "needs_attention",
    );
  });

  it("flags pending conversation items as translating", () => {
    const [item] = deriveConversationItems([base]);

    expect(deriveTranslationActivity("listening", item, "receiving", 0)).toBe("translating");
  });
});
