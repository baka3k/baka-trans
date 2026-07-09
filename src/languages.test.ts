import { describe, expect, it } from "vitest";
import { sourceLanguageOptions, targetLanguageOptions } from "./languages";

describe("language options", () => {
  it("includes exactly the OpenAI Realtime Translation output languages", () => {
    expect(targetLanguageOptions.map((language) => language.value)).toEqual([
      "es",
      "pt",
      "fr",
      "ja",
      "ru",
      "zh",
      "de",
      "ko",
      "hi",
      "id",
      "vi",
      "it",
      "en",
    ]);
  });

  it("excludes auto from target languages", () => {
    expect(targetLanguageOptions.some((language) => language.value === "auto")).toBe(false);
  });

  it("includes auto in source languages", () => {
    expect(sourceLanguageOptions[0]).toEqual({ value: "auto", label: "Auto" });
  });
});
