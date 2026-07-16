import type { TranslationProvider } from "./types";

export interface LanguageMetadata {
  code: string;
  label: string;
  supportsSource: boolean;
  supportsTarget: boolean;
  isAuto: boolean;
}

const openaiTargetLanguageCodes = [
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
] as const;
const openaiTargetLanguageCodeSet = new Set<string>(openaiTargetLanguageCodes);

export const languageMetadata = [
  { code: "auto", label: "Auto", supportsSource: true, supportsTarget: false, isAuto: true },
  { code: "ar", label: "Arabic", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "af", label: "Afrikaans", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "az", label: "Azerbaijani", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "be", label: "Belarusian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "bn", label: "Bengali", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "bs", label: "Bosnian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "bg", label: "Bulgarian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "ca", label: "Catalan", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "zh", label: "Chinese", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "zh-Hans", label: "Chinese (Simplified)", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "zh-Hant", label: "Chinese (Traditional)", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "hr", label: "Croatian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "cs", label: "Czech", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "da", label: "Danish", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "nl", label: "Dutch", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "dz", label: "Dzongkha", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "en", label: "English", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "eo", label: "Esperanto", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "et", label: "Estonian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "eu", label: "Basque", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "fa", label: "Persian / Farsi", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "fi", label: "Finnish", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "fil", label: "Filipino", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "fr", label: "French", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "gl", label: "Galician", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "de", label: "German", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "el", label: "Greek", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "gu", label: "Gujarati", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "ht", label: "Haitian Creole", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "haw", label: "Hawaiian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "he", label: "Hebrew", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "hi", label: "Hindi", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "hu", label: "Hungarian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "hy", label: "Armenian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "id", label: "Indonesian", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "it", label: "Italian", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "ja", label: "Japanese", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "jv", label: "Javanese", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "ka", label: "Georgian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "kk", label: "Kazakh", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "ko", label: "Korean", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "ku", label: "Kurdish", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "la", label: "Latin", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "lv", label: "Latvian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "lt", label: "Lithuanian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "mk", label: "Macedonian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "ms", label: "Malay", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "ml", label: "Malayalam", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "mi", label: "Maori", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "mn", label: "Mongolian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "my", label: "Burmese / Myanmar", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "ne", label: "Nepali", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "no", label: "Norwegian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "nn", label: "Nynorsk", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "pl", label: "Polish", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "pt", label: "Portuguese", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "pt-BR", label: "Portuguese (Brazil)", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "pt-PT", label: "Portuguese (Portugal)", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "pa", label: "Punjabi", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "ro", label: "Romanian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "ru", label: "Russian", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "sr", label: "Serbian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "sn", label: "Shona", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "sk", label: "Slovak", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "sl", label: "Slovenian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "sq", label: "Albanian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "es", label: "Spanish", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "sw", label: "Swahili", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "sv", label: "Swedish", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "tl", label: "Tagalog", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "te", label: "Telugu", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "th", label: "Thai", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "tr", label: "Turkish", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "uk", label: "Ukrainian", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "uz", label: "Uzbek", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "vi", label: "Vietnamese", supportsSource: true, supportsTarget: true, isAuto: false },
  { code: "cy", label: "Welsh", supportsSource: true, supportsTarget: false, isAuto: false },
  { code: "yo", label: "Yoruba", supportsSource: true, supportsTarget: false, isAuto: false },
] as const satisfies readonly LanguageMetadata[];

export type LanguageCode = (typeof languageMetadata)[number]["code"];

export interface LanguageOption {
  value: LanguageCode;
  label: string;
}

function toLanguageOption(language: (typeof languageMetadata)[number]): LanguageOption {
  return {
    value: language.code,
    label: language.label,
  };
}

function targetLanguageOrder(code: LanguageCode) {
  return openaiTargetLanguageCodes.indexOf(code as (typeof openaiTargetLanguageCodes)[number]);
}

export const sourceLanguageOptions = languageMetadata
  .filter((language) => language.supportsSource)
  .map(toLanguageOption);

function supportsTargetByProvider(
  language: (typeof languageMetadata)[number],
  provider: TranslationProvider,
) {
  if (provider === "openai_realtime") {
    return openaiTargetLanguageCodeSet.has(language.code) && language.supportsTarget;
  }
  if (provider === "local_whisper_ollama") {
    return language.code === "vi";
  }
  return !language.isAuto;
}

export function targetLanguageOptionsForProvider(provider: TranslationProvider) {
  const options = languageMetadata.filter((language) => supportsTargetByProvider(language, provider));
  if (provider === "openai_realtime") {
    options.sort((first, second) => targetLanguageOrder(first.code) - targetLanguageOrder(second.code));
  }
  return options.map(toLanguageOption);
}

export const targetLanguageOptions = targetLanguageOptionsForProvider("openai_realtime");
