import type {
  ConversationDisplayItem,
  ConversationSentencePair,
  LlmProviderProfileDraft,
  MeetingSummaryConfig,
  MeetingSummaryResult,
  SessionStatus,
  SourceSignalSnapshot,
  SourceSignalState,
  TranscriptItem,
  TranslationActivityState,
} from "./types";

export function mergeTranscriptDelta(
  items: TranscriptItem[],
  update: TranscriptItem,
): TranscriptItem[] {
  const last = items[items.length - 1];
  if (!last || last.status === "final" || !isSingleSidedDelta(update)) {
    return [...items, update];
  }

  return [
    ...items.slice(0, -1),
    {
      ...last,
      timestampMs: Math.min(last.timestampMs, update.timestampMs),
      sourceText: appendTranscriptText(last.sourceText, update.sourceText, false),
      translatedText: appendTranscriptText(last.translatedText, update.translatedText, true),
      status: update.status,
      latencyMs: update.latencyMs ?? last.latencyMs,
    },
  ];
}

function isSingleSidedDelta(item: TranscriptItem) {
  return (
    (item.sourceText.length > 0 && item.translatedText.length === 0) ||
    (item.translatedText.length > 0 && item.sourceText.length === 0)
  );
}

function appendTranscriptText(current: string, delta: string, breakAfterSentence: boolean) {
  if (!delta) {
    return current;
  }
  if (!current) {
    return delta;
  }
  if (breakAfterSentence && shouldStartNewTranscriptLine(current, delta)) {
    return `${current.trimEnd()}\n${delta.trimStart()}`;
  }
  if (shouldInsertSpaceBetweenChunks(current, delta)) {
    return `${current.trimEnd()} ${delta.trimStart()}`;
  }
  return `${current}${delta}`;
}

function shouldStartNewTranscriptLine(current: string, delta: string) {
  const next = delta.trimStart();
  return /[.!?。！？]\s*$/.test(current) && next.length > 0 && !/^[,.;:!?)]/.test(next);
}

function shouldInsertSpaceBetweenChunks(current: string, delta: string) {
  const trimmedCurrent = current.trimEnd();
  const trimmedDelta = delta.trimStart();
  const previous = trimmedCurrent[trimmedCurrent.length - 1];
  const next = trimmedDelta[0];
  if (!previous || !next) {
    return false;
  }
  if (/\s/.test(current[current.length - 1] ?? "") || /\s/.test(delta[0] ?? "")) {
    return false;
  }
  if (isClosingPunctuation(next) || isOpeningPunctuation(previous)) {
    return false;
  }
  return isWordCharacter(previous) && isWordCharacter(next);
}

function isWordCharacter(value: string) {
  return /[\p{L}\p{N}]/u.test(value);
}

function isClosingPunctuation(value: string) {
  return /[,.;:!?%)]/.test(value);
}

function isOpeningPunctuation(value: string) {
  return /[(\[]/.test(value);
}

export function renderTranscript(
  items: TranscriptItem[],
  format: "text" | "markdown",
  meetingNotes?: MeetingSummaryResult | null,
) {
  const notes = meetingNotes ? renderMeetingNotes(meetingNotes, format) : "";
  if (format === "markdown") {
    const transcript = [
      "# Baka Trans Transcript",
      "",
      ...items.map((item) => renderTranscriptItem(item, "markdown")),
    ].join("\n");
    return notes ? `${transcript}\n\n${notes}` : transcript;
  }

  const transcript = items
    .map((item) => renderTranscriptItem(item, "text"))
    .join("\n");
  return notes ? `${transcript}\n\n${notes}` : transcript;
}

export function deriveConversationItems(items: TranscriptItem[]): ConversationDisplayItem[] {
  return items.map((item) => {
    const sourceText = normalizeTranscriptWhitespace(item.sourceText);
    const translatedText = normalizeTranscriptWhitespace(item.translatedText);
    return {
      id: item.id,
      timestampMs: item.timestampMs,
      sourceText,
      translatedText,
      sentencePairs: pairTranscriptSentences(sourceText, translatedText),
      status: item.status,
      latencyMs: item.latencyMs,
      speakerLabel: item.speakerLabel,
      speakerSegmentId: item.speakerSegmentId,
      speakerConfidence: item.speakerConfidence,
      speakerDisplayLabel: item.speakerLabel?.trim() || "Source",
      hasPendingTranslation:
        item.status !== "error" && sourceText.length > 0 && translatedText.length === 0,
    };
  });
}

function renderTranscriptItem(item: TranscriptItem, format: "text" | "markdown") {
  const timestamp = new Date(item.timestampMs).toLocaleTimeString();
  const sourceText = normalizeTranscriptWhitespace(item.sourceText);
  const translatedText = normalizeTranscriptWhitespace(item.translatedText);
  const pairs = pairTranscriptSentences(sourceText, translatedText);

  if (format === "markdown") {
    const lines = pairs.flatMap((pair) => [
      `**Original:** ${pair.sourceText || "_No source text_"}`,
      `**Translation:** ${pair.translatedText || "_No translation yet_"}`,
      "",
    ]);
    return [`## ${timestamp} (${item.status})`, "", ...lines].join("\n");
  }

  const lines = pairs.flatMap((pair) => [
    `Original: ${pair.sourceText || "No source text"}`,
    `Translation: ${pair.translatedText || "No translation yet"}`,
    "",
  ]);
  return [`[${timestamp}]`, ...lines].join("\n");
}

export function pairTranscriptSentences(
  sourceText: string,
  translatedText: string,
): ConversationSentencePair[] {
  const sourceSentences = splitTranscriptSentences(sourceText);
  const translatedSentences = splitTranscriptSentences(translatedText);
  const pairCount = Math.max(sourceSentences.length, translatedSentences.length);

  return Array.from({ length: pairCount }, (_, index) => ({
    sourceText: sourceSentences[index] ?? "",
    translatedText: translatedSentences[index] ?? "",
  }));
}

export function splitTranscriptSentences(text: string) {
  const normalized = normalizeTranscriptWhitespace(text).replace(/\n+/g, " ");
  if (!normalized) {
    return [];
  }

  const matches = normalized.match(/[^.!?。！？]+[.!?。！？]+["')\]]*|[^.!?。！？]+$/g);
  return (matches ?? [normalized])
    .map((sentence) => sentence.trim())
    .filter(Boolean);
}

function normalizeTranscriptWhitespace(text: string) {
  return text
    .replace(/[ \t]*\n[ \t]*/g, "\n")
    .replace(/[ \t]+/g, " ")
    .replace(/\s+([,.;:!?%)\]])/g, "$1")
    .replace(/([([])\s+/g, "$1")
    .trim();
}

export function deriveSourceSignalState(
  snapshot: SourceSignalSnapshot | null,
  selectedInputDeviceId: string,
  sessionStatus: SessionStatus,
  nowMs: number,
  options: { silenceThreshold?: number; staleAfterMs?: number } = {},
): SourceSignalState {
  if (sessionStatus === "error") {
    return "error";
  }

  if (!isSourceSignalSessionActive(sessionStatus)) {
    return "waiting";
  }

  if (!selectedInputDeviceId || !snapshot) {
    return "waiting";
  }

  if (snapshot.inputDeviceId !== selectedInputDeviceId) {
    return "waiting";
  }

  const staleAfterMs = options.staleAfterMs ?? 2000;
  if (nowMs - snapshot.receivedAtMs > staleAfterMs) {
    return "stale";
  }

  const silenceThreshold = options.silenceThreshold ?? 0.03;
  return Math.max(snapshot.peak, snapshot.rms) > silenceThreshold ? "receiving" : "silent";
}

export function deriveTranslationActivity(
  sessionStatus: SessionStatus,
  latestItem: ConversationDisplayItem | undefined,
  sourceSignalState: SourceSignalState,
  translatedPeak: number,
): TranslationActivityState {
  if (
    sessionStatus === "error" ||
    sourceSignalState === "error" ||
    (isSourceSignalSessionActive(sessionStatus) && sourceSignalState === "stale")
  ) {
    return "needs_attention";
  }

  if (sessionStatus === "translating" || latestItem?.hasPendingTranslation) {
    return "translating";
  }

  if (
    sessionStatus === "starting" ||
    sessionStatus === "listening"
  ) {
    return "listening";
  }

  if (sessionStatus === "speaking" || translatedPeak > 0.03) {
    return "ready";
  }

  return "ready";
}

function isSourceSignalSessionActive(status: SessionStatus) {
  return (
    status === "starting" ||
    status === "listening" ||
    status === "translating" ||
    status === "speaking"
  );
}

export function buildMeetingSummaryConfig(
  providerProfileId: string,
): MeetingSummaryConfig {
  return {
    providerProfileId,
    trigger: "manual",
    transcriptScope: "both",
    outputLanguage: "Vietnamese",
    sections: {
      summary: true,
      decisions: true,
      actionItems: true,
      blockers: true,
      importantPoints: true,
    },
    maxTranscriptChars: 24000,
    rollingMemoryEnabled: true,
  };
}

export function validateLlmProfileDraft(draft: LlmProviderProfileDraft) {
  const errors: string[] = [];
  if (!draft.name.trim()) {
    errors.push("Profile name is required.");
  }
  if (!draft.model.trim()) {
    errors.push("Model is required.");
  }
  if (
    (draft.kind === "openai_compatible" || draft.kind === "adk_litellm") &&
    !draft.baseUrl?.trim()
  ) {
    errors.push("Base URL is required for this provider.");
  }
  return errors;
}

function renderMeetingNotes(notes: MeetingSummaryResult, format: "text" | "markdown") {
  if (format === "markdown") {
    return [
      "# Meeting Notes",
      "",
      notes.summary ? `## Summary\n\n${notes.summary}` : "",
      renderMarkdownList("Decisions", notes.decisions),
      renderActionItems(notes),
      renderMarkdownList("Blockers", notes.blockers),
      renderMarkdownList("Important Points", notes.importantPoints),
      `\n_Model: ${notes.model}_`,
    ]
      .filter(Boolean)
      .join("\n\n");
  }

  return [
    "Meeting Notes",
    notes.summary ? `Summary:\n${notes.summary}` : "",
    renderTextList("Decisions", notes.decisions),
    renderTextActionItems(notes),
    renderTextList("Blockers", notes.blockers),
    renderTextList("Important Points", notes.importantPoints),
    `Model: ${notes.model}`,
  ]
    .filter(Boolean)
    .join("\n\n");
}

function renderMarkdownList(title: string, values: string[]) {
  if (values.length === 0) {
    return "";
  }
  return `## ${title}\n\n${values.map((value) => `- ${value}`).join("\n")}`;
}

function renderTextList(title: string, values: string[]) {
  if (values.length === 0) {
    return "";
  }
  return `${title}:\n${values.map((value) => `- ${value}`).join("\n")}`;
}

function renderActionItems(notes: MeetingSummaryResult) {
  if (notes.actionItems.length === 0) {
    return "";
  }
  return `## Action Items\n\n${notes.actionItems
    .map((item) => {
      const owner = item.owner ? ` Owner: ${item.owner}.` : "";
      const due = item.dueDate ? ` Due: ${item.dueDate}.` : "";
      return `- ${item.text}${owner}${due}`;
    })
    .join("\n")}`;
}

function renderTextActionItems(notes: MeetingSummaryResult) {
  if (notes.actionItems.length === 0) {
    return "";
  }
  return `Action Items:\n${notes.actionItems
    .map((item) => {
      const owner = item.owner ? ` Owner: ${item.owner}.` : "";
      const due = item.dueDate ? ` Due: ${item.dueDate}.` : "";
      return `- ${item.text}${owner}${due}`;
    })
    .join("\n")}`;
}
