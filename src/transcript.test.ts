import { describe, expect, it } from "vitest";
import {
  buildMeetingSummaryConfig,
  mergeTranscriptDelta,
  renderTranscript,
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
    expect(config.sections.actionItems).toBe(true);
  });
});
