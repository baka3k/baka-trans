use crate::error::{AppError, AppResult};
use crate::llm::{self, ChatMessage};
use crate::models::{
    ActionItem, MeetingSummaryConfig, MeetingSummaryResult, MeetingSummaryStatus,
    MeetingSummaryStatusEvent, TranscriptItem, TranscriptScope, TranscriptStatus,
};
use serde::Deserialize;
use std::collections::HashSet;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

const DEFAULT_MAX_TRANSCRIPT_CHARS: usize = 24_000;
const CHUNK_TARGET_CHARS: usize = 10_000;

#[derive(Debug, Clone)]
struct TranscriptChunk {
    text: String,
    source_item_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SummaryDraft {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    decisions: Vec<String>,
    #[serde(default)]
    action_items: Vec<ActionItem>,
    #[serde(default)]
    blockers: Vec<String>,
    #[serde(default)]
    important_points: Vec<String>,
}

pub async fn run_meeting_summary_agent(
    app: AppHandle,
    transcript: Vec<TranscriptItem>,
    config: MeetingSummaryConfig,
) -> AppResult<MeetingSummaryResult> {
    emit_status(&app, MeetingSummaryStatus::Running, "Preparing transcript")?;
    let profile = llm::get_profile(&config.provider_profile_id)?;
    let chunks = transcript_chunks(&transcript, &config)?;
    let source_item_ids = chunks
        .iter()
        .flat_map(|chunk| chunk.source_item_ids.clone())
        .collect::<Vec<_>>();

    emit_status(
        &app,
        MeetingSummaryStatus::Running,
        "Calling summary provider",
    )?;
    let mut drafts = Vec::new();
    let mut rolling_memory = String::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let prompt = build_chunk_prompt(index + 1, chunks.len(), chunk, &rolling_memory, &config);
        let completion = llm::chat_completion(
            &profile,
            vec![
                ChatMessage {
                    role: "system",
                    content: system_prompt(),
                },
                ChatMessage {
                    role: "user",
                    content: prompt,
                },
            ],
            true,
        )
        .await?;
        let draft = parse_summary_draft(&completion.content)?;
        if config.rolling_memory_enabled {
            rolling_memory = compact_memory(&rolling_memory, &draft);
        }
        drafts.push(draft);
    }

    emit_status(&app, MeetingSummaryStatus::Running, "Validating notes")?;
    let draft = merge_drafts(drafts);
    let result = MeetingSummaryResult {
        id: Uuid::new_v4().to_string(),
        created_at_ms: now_ms(),
        source_item_ids,
        summary: if config.sections.summary {
            draft.summary
        } else {
            String::new()
        },
        decisions: if config.sections.decisions {
            draft.decisions
        } else {
            Vec::new()
        },
        action_items: if config.sections.action_items {
            draft.action_items
        } else {
            Vec::new()
        },
        blockers: if config.sections.blockers {
            draft.blockers
        } else {
            Vec::new()
        },
        important_points: if config.sections.important_points {
            draft.important_points
        } else {
            Vec::new()
        },
        model: profile.model,
        provider_profile_id: config.provider_profile_id,
        status: MeetingSummaryStatus::Complete,
        error_message: None,
    };

    app.emit("meeting-summary-update", &result)
        .map_err(|err| AppError::new("event_emit_error", err.to_string()))?;
    emit_status(&app, MeetingSummaryStatus::Complete, "Meeting notes ready")?;
    Ok(result)
}

fn system_prompt() -> String {
    [
        "You are MeetingSummaryAgent for a realtime meeting translation app.",
        "Return only a JSON object with keys summary, decisions, actionItems, blockers, importantPoints.",
        "actionItems must contain objects with text, owner, dueDate, sourceItemIds.",
        "Use only information present in the transcript. Do not invent owners, dates, or decisions.",
    ]
    .join(" ")
}

fn build_chunk_prompt(
    chunk_index: usize,
    chunk_count: usize,
    chunk: &TranscriptChunk,
    rolling_memory: &str,
    config: &MeetingSummaryConfig,
) -> String {
    format!(
        "Output language: {language}\nChunk: {chunk_index}/{chunk_count}\nEnabled sections: summary={summary}, decisions={decisions}, actionItems={action_items}, blockers={blockers}, importantPoints={important_points}\nRolling memory:\n{memory}\n\nTranscript chunk with source item IDs:\n{transcript}\n\nReturn compact valid JSON.",
        language = config.output_language.trim(),
        summary = config.sections.summary,
        decisions = config.sections.decisions,
        action_items = config.sections.action_items,
        blockers = config.sections.blockers,
        important_points = config.sections.important_points,
        memory = if rolling_memory.trim().is_empty() {
            "(none)"
        } else {
            rolling_memory
        },
        transcript = chunk.text,
    )
}

fn transcript_chunks(
    transcript: &[TranscriptItem],
    config: &MeetingSummaryConfig,
) -> AppResult<Vec<TranscriptChunk>> {
    let selected = select_transcript_items(transcript);
    if selected.is_empty() {
        return Err(AppError::new(
            "summary_agent_empty_transcript",
            "No transcript text is available for meeting notes.",
        ));
    }

    let max_chars = if config.max_transcript_chars == 0 {
        DEFAULT_MAX_TRANSCRIPT_CHARS
    } else {
        config.max_transcript_chars
    };
    let chunk_target = CHUNK_TARGET_CHARS.min(max_chars.max(1));
    let mut consumed = 0usize;
    let mut chunks = Vec::<TranscriptChunk>::new();
    let mut current = TranscriptChunk {
        text: String::new(),
        source_item_ids: Vec::new(),
    };

    for item in selected {
        if consumed >= max_chars {
            break;
        }
        let mut line = format_transcript_item(item, config.transcript_scope);
        if line.trim().is_empty() {
            continue;
        }
        let remaining = max_chars.saturating_sub(consumed);
        if line.len() > remaining {
            line.truncate(remaining);
        }
        if !current.text.is_empty() && current.text.len() + line.len() > chunk_target {
            chunks.push(current);
            current = TranscriptChunk {
                text: String::new(),
                source_item_ids: Vec::new(),
            };
        }
        current.text.push_str(&line);
        current.text.push('\n');
        current.source_item_ids.push(item.id.clone());
        consumed += line.len();
    }

    if !current.text.trim().is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() {
        return Err(AppError::new(
            "summary_agent_empty_transcript",
            "No transcript text matches the selected notes scope.",
        ));
    }
    Ok(chunks)
}

fn select_transcript_items(transcript: &[TranscriptItem]) -> Vec<&TranscriptItem> {
    let finals = transcript
        .iter()
        .filter(|item| item.status == TranscriptStatus::Final && has_text(item))
        .collect::<Vec<_>>();
    if finals.is_empty() {
        transcript
            .iter()
            .filter(|item| item.status != TranscriptStatus::Error && has_text(item))
            .collect()
    } else {
        finals
    }
}

fn format_transcript_item(item: &TranscriptItem, scope: TranscriptScope) -> String {
    match scope {
        TranscriptScope::Source => {
            if item.source_text.trim().is_empty() {
                String::new()
            } else {
                format!("[{}] Source: {}", item.id, item.source_text.trim())
            }
        }
        TranscriptScope::Translated => {
            if item.translated_text.trim().is_empty() {
                String::new()
            } else {
                format!("[{}] Translation: {}", item.id, item.translated_text.trim())
            }
        }
        TranscriptScope::Both => {
            let mut lines = Vec::new();
            if !item.source_text.trim().is_empty() {
                lines.push(format!("Source: {}", item.source_text.trim()));
            }
            if !item.translated_text.trim().is_empty() {
                lines.push(format!("Translation: {}", item.translated_text.trim()));
            }
            if lines.is_empty() {
                String::new()
            } else {
                format!("[{}] {}", item.id, lines.join(" | "))
            }
        }
    }
}

fn parse_summary_draft(content: &str) -> AppResult<SummaryDraft> {
    let value = llm::parse_json_object(content)?;
    serde_json::from_value(value).map_err(|err| {
        AppError::new(
            "summary_agent_parse_error",
            format!("The summary model response did not match the notes schema: {err}"),
        )
    })
}

fn merge_drafts(drafts: Vec<SummaryDraft>) -> SummaryDraft {
    let mut merged = SummaryDraft::default();
    let mut seen = HashSet::new();
    for draft in drafts {
        push_unique_block(&mut merged.summary, draft.summary);
        push_unique_strings(&mut merged.decisions, draft.decisions, &mut seen);
        push_unique_strings(&mut merged.blockers, draft.blockers, &mut seen);
        push_unique_strings(
            &mut merged.important_points,
            draft.important_points,
            &mut seen,
        );
        for item in draft.action_items {
            if seen.insert(format!("action:{}", item.text.trim().to_lowercase())) {
                merged.action_items.push(item);
            }
        }
    }
    merged
}

fn push_unique_block(target: &mut String, value: String) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push_str("\n\n");
    }
    target.push_str(trimmed);
}

fn push_unique_strings(target: &mut Vec<String>, values: Vec<String>, seen: &mut HashSet<String>) {
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_lowercase()) {
            target.push(trimmed.to_string());
        }
    }
}

fn compact_memory(previous: &str, draft: &SummaryDraft) -> String {
    let mut memory = String::new();
    if !previous.trim().is_empty() {
        memory.push_str(previous.trim());
        memory.push('\n');
    }
    if !draft.summary.trim().is_empty() {
        memory.push_str("Summary so far: ");
        memory.push_str(draft.summary.trim());
        memory.push('\n');
    }
    for decision in draft.decisions.iter().take(5) {
        memory.push_str("Decision: ");
        memory.push_str(decision.trim());
        memory.push('\n');
    }
    memory
        .chars()
        .rev()
        .take(3_000)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn has_text(item: &TranscriptItem) -> bool {
    !item.source_text.trim().is_empty() || !item.translated_text.trim().is_empty()
}

fn emit_status(
    app: &AppHandle,
    status: MeetingSummaryStatus,
    message: impl Into<String>,
) -> AppResult<()> {
    app.emit(
        "summary-agent-status",
        MeetingSummaryStatusEvent {
            status,
            message: message.into(),
        },
    )
    .map_err(|err| AppError::new("event_emit_error", err.to_string()))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{parse_summary_draft, transcript_chunks};
    use crate::models::{
        MeetingSummaryConfig, MeetingSummarySections, MeetingSummaryTrigger, TranscriptItem,
        TranscriptScope, TranscriptStatus,
    };

    fn config() -> MeetingSummaryConfig {
        MeetingSummaryConfig {
            provider_profile_id: "profile".to_string(),
            trigger: MeetingSummaryTrigger::Manual,
            transcript_scope: TranscriptScope::Both,
            output_language: "English".to_string(),
            sections: MeetingSummarySections {
                summary: true,
                decisions: true,
                action_items: true,
                blockers: true,
                important_points: true,
            },
            max_transcript_chars: 2_000,
            rolling_memory_enabled: true,
        }
    }

    #[test]
    fn transcript_chunks_preserve_source_ids() {
        let transcript = vec![TranscriptItem {
            id: "item-1".to_string(),
            timestamp_ms: 1,
            source_text: "We decided to ship Friday.".to_string(),
            translated_text: "Chung ta quyet dinh giao vao thu Sau.".to_string(),
            status: TranscriptStatus::Partial,
            latency_ms: None,
        }];
        let chunks = transcript_chunks(&transcript, &config()).unwrap();
        assert_eq!(chunks[0].source_item_ids, vec!["item-1"]);
        assert!(chunks[0].text.contains("We decided"));
    }

    #[test]
    fn malformed_summary_output_is_rejected() {
        let err = parse_summary_draft("not json").unwrap_err();
        assert_eq!(err.code, "summary_agent_parse_error");
    }
}
