import { describe, expect, it } from "vitest";
import { mergeTranscriptDelta, renderTranscript } from "./transcript";
import type { TranscriptItem } from "./types";

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

  it("renders markdown exports", () => {
    const output = renderTranscript(
      [{ ...base, translatedText: "Xin chao", status: "final" }],
      "markdown",
    );

    expect(output).toContain("# Baka Trans Transcript");
    expect(output).toContain("Xin chao");
  });
});

