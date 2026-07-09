import type {
  LlmProviderProfileDraft,
  MeetingSummaryConfig,
  MeetingSummaryResult,
  TranscriptItem,
} from "./types";

export function mergeTranscriptDelta(
  items: TranscriptItem[],
  update: TranscriptItem,
): TranscriptItem[] {
  const last = items[items.length - 1];
  if (!last || last.status !== "partial" || update.status !== "partial") {
    return [...items, update];
  }

  const canMergeSource =
    update.sourceText.length > 0 && update.translatedText.length === 0;
  const canMergeTranslation =
    update.translatedText.length > 0 && update.sourceText.length === 0;

  if (!canMergeSource && !canMergeTranslation) {
    return [...items, update];
  }

  return [
    ...items.slice(0, -1),
    {
      ...last,
      timestampMs: Math.min(last.timestampMs, update.timestampMs),
      sourceText: `${last.sourceText}${update.sourceText}`,
      translatedText: `${last.translatedText}${update.translatedText}`,
      status: update.status,
      latencyMs: update.latencyMs ?? last.latencyMs,
    },
  ];
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
      ...items.map(
        (item) =>
          `## ${new Date(item.timestampMs).toLocaleTimeString()} (${item.status})\n\n**Source:** ${item.sourceText}\n\n**Translation:** ${item.translatedText}\n`,
      ),
    ].join("\n");
    return notes ? `${transcript}\n\n${notes}` : transcript;
  }

  const transcript = items
    .map(
      (item) =>
        `[${new Date(item.timestampMs).toLocaleTimeString()}]\nSource: ${item.sourceText}\nTranslation: ${item.translatedText}\n`,
    )
    .join("\n");
  return notes ? `${transcript}\n\n${notes}` : transcript;
}

export function buildMeetingSummaryConfig(
  providerProfileId: string,
): MeetingSummaryConfig {
  return {
    providerProfileId,
    trigger: "manual",
    transcriptScope: "both",
    outputLanguage: "English",
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
