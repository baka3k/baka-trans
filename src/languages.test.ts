import { describe, expect, it } from "vitest";
import {
  sourceLanguageOptions,
  sourceLanguageOptionsForProvider,
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

  it("offers Whisper-supported local source languages and auto detection", () => {
    const localSources = sourceLanguageOptionsForProvider("local_whisper_ollama").map(
      (language) => language.value,
    );

    expect(localSources).toEqual(expect.arrayContaining(["auto", "en", "ja", "vi", "th"]));
    expect(localSources).not.toContain("dz");
  });

  it("offers all non-auto languages as local Ollama targets", () => {
    const localTargets = targetLanguageOptionsForProvider("local_whisper_ollama").map(
      (language) => language.value,
    );

    expect(localTargets).toEqual(expect.arrayContaining(["en", "ja", "vi", "th", "pt-BR"]));
    expect(localTargets).not.toContain("auto");
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
