import type { TranscriptItem } from "./types";

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
) {
  if (format === "markdown") {
    return [
      "# Baka Trans Transcript",
      "",
      ...items.map(
        (item) =>
          `## ${new Date(item.timestampMs).toLocaleTimeString()} (${item.status})\n\n**Source:** ${item.sourceText}\n\n**Translation:** ${item.translatedText}\n`,
      ),
    ].join("\n");
  }

  return items
    .map(
      (item) =>
        `[${new Date(item.timestampMs).toLocaleTimeString()}]\nSource: ${item.sourceText}\nTranslation: ${item.translatedText}\n`,
    )
    .join("\n");
}

