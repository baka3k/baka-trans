import { describe, expect, it } from "vitest";
import {
  sourceLanguageOptions,
  targetLanguageOptions,
  targetLanguageOptionsForProvider,
} from "./languages";

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

  it("includes regional Google Live Translation target codes", () => {
    const googleTargets = targetLanguageOptionsForProvider("google_live_translate").map(
      (language) => language.value,
    );

    expect(googleTargets).toContain("pt-BR");
    expect(googleTargets).toContain("pt-PT");
    expect(googleTargets).toContain("zh-Hans");
    expect(googleTargets).toContain("zh-Hant");
  });

  it("keeps auto out of Google target languages", () => {
    expect(
      targetLanguageOptionsForProvider("google_live_translate").some(
        (language) => language.value === "auto",
      ),
    ).toBe(false);
  });
});
