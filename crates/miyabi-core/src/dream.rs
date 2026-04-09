//! Dreaming over deterministic task events to extract patterns and learnings.

use crate::error::Result;
use crate::store::{EventStore, TaskEvent, TaskEventType};
use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DREAM_TASK_ID: &str = "__dream__";
const LONG_COMPLETION_SECS: u64 = 60 * 60;
const VERY_LONG_COMPLETION_SECS: u64 = 4 * 60 * 60;
const FREQUENT_PATTERN_THRESHOLD: usize = 2;
const HIGH_PATTERN_THRESHOLD: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamReport {
    pub patterns: DreamPatterns,
    pub learnings: Vec<Learning>,
    pub events_processed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DreamPatterns {
    pub gate_rejections: HashMap<String, usize>,
    pub lock_conflicts: HashMap<String, usize>,
    pub completion_times: Vec<(String, Duration)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Learning {
    pub title: String,
    pub importance: Importance,
    pub content: String,
    pub related_task: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Importance {
    High,
    Medium,
    Low,
}

pub fn dream(event_store: &EventStore, since: Option<ChronoDuration>) -> Result<DreamReport> {
    let events = collect_events(event_store, since)?;
    Ok(analyze_events(&events))
}

pub fn write_high_learnings(report: &DreamReport, directory: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(directory)?;

    let mut written = Vec::new();
    for (index, learning) in report
        .learnings
        .iter()
        .filter(|learning| learning.importance == Importance::High)
        .enumerate()
    {
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let filename = format!("{timestamp}-{index:02}-{}.md", slugify(&learning.title));
        let path = directory.join(filename);
        let tmp_path = path.with_extension("md.tmp");
        let content = format!(
            "# {}\n\n- importance: {:?}\n- related_task: {}\n\n{}\n",
            learning.title,
            learning.importance,
            learning.related_task.as_deref().unwrap_or("none"),
            learning.content
        );
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, &path)?;
        written.push(path);
    }

    Ok(written)
}

pub fn analyze_events(events: &[TaskEvent]) -> DreamReport {
    let mut patterns = DreamPatterns::default();
    let mut task_started_at: HashMap<&str, chrono::DateTime<Utc>> = HashMap::new();

    for event in events {
        match event.event_type {
            TaskEventType::DagChanged => {
                if event.task_id != DREAM_TASK_ID {
                    task_started_at
                        .entry(event.task_id.as_str())
                        .or_insert(event.ts);
                }
            }
            TaskEventType::GateRejected => {
                if let Some(gate_name) = event.payload.get("gate").and_then(|value| value.as_str())
                {
                    *patterns
                        .gate_rejections
                        .entry(gate_name.to_string())
                        .or_default() += 1;
                }
                if let Some(files) = event
                    .payload
                    .get("files")
                    .and_then(|value| value.as_array())
                {
                    for file in files.iter().filter_map(|value| value.as_str()) {
                        *patterns.lock_conflicts.entry(file.to_string()).or_default() += 1;
                    }
                }
            }
            TaskEventType::MergeVerified => {
                if let Some(started_at) = task_started_at.get(event.task_id.as_str()) {
                    let elapsed = event.ts.signed_duration_since(*started_at);
                    if let Ok(duration) = elapsed.to_std() {
                        patterns
                            .completion_times
                            .push((event.task_id.clone(), duration));
                    }
                }
            }
            _ => {}
        }
    }

    patterns
        .completion_times
        .sort_by(|left, right| left.0.cmp(&right.0));
    let learnings = extract_learnings(&patterns);

    DreamReport {
        patterns,
        learnings,
        events_processed: events.len(),
    }
}

fn collect_events(
    event_store: &EventStore,
    since: Option<ChronoDuration>,
) -> Result<Vec<TaskEvent>> {
    let events = event_store.replay(None)?;
    if let Some(since) = since {
        let cutoff = Utc::now() - since;
        return Ok(events
            .into_iter()
            .filter(|event| event.ts >= cutoff)
            .collect());
    }

    let start_index = events
        .iter()
        .rposition(|event| event.event_type == TaskEventType::DreamRecorded)
        .map_or(0, |index| index + 1);
    Ok(events.into_iter().skip(start_index).collect())
}

fn extract_learnings(patterns: &DreamPatterns) -> Vec<Learning> {
    let mut learnings = Vec::new();

    let mut gate_entries: Vec<_> = patterns.gate_rejections.iter().collect();
    gate_entries.sort_by(|left, right| left.0.cmp(right.0));
    for (gate_name, count) in gate_entries {
        if *count >= FREQUENT_PATTERN_THRESHOLD {
            learnings.push(Learning {
                title: format!("{gate_name} の拒否が多発"),
                importance: if *count >= HIGH_PATTERN_THRESHOLD {
                    Importance::High
                } else {
                    Importance::Medium
                },
                content: format!(
                    "{gate_name} の拒否が {count} 回発生しています。手順書の改善が必要です。"
                ),
                related_task: None,
            });
        }
    }

    let mut lock_entries: Vec<_> = patterns.lock_conflicts.iter().collect();
    lock_entries.sort_by(|left, right| left.0.cmp(right.0));
    for (file, count) in lock_entries {
        learnings.push(Learning {
            title: format!("ロック競合が発生: {file}"),
            importance: if *count >= HIGH_PATTERN_THRESHOLD {
                Importance::High
            } else {
                Importance::Medium
            },
            content: format!(
                "{file} でロック競合が {count} 回発生しました。ファイル分割を検討してください。"
            ),
            related_task: None,
        });
    }

    for (task_id, duration) in &patterns.completion_times {
        if duration.as_secs() >= LONG_COMPLETION_SECS {
            learnings.push(Learning {
                title: format!("完了時間が長い: {task_id}"),
                importance: if duration.as_secs() >= VERY_LONG_COMPLETION_SECS {
                    Importance::High
                } else {
                    Importance::Medium
                },
                content: format!(
                    "{task_id} は完了まで {} 秒かかりました。見積もり精度の改善が必要です。",
                    duration.as_secs()
                ),
                related_task: Some(task_id.clone()),
            });
        }
    }

    learnings
}

fn slugify(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_ascii_lowercase();
    if slug.is_empty() {
        "learning".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::TaskEvent;
    use chrono::{Duration as ChronoDuration, TimeZone};
    use serde_json::json;

    fn event(
        event_type: TaskEventType,
        task_id: &str,
        ts_offset_sec: i64,
        payload: serde_json::Value,
    ) -> TaskEvent {
        let base = Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap();
        TaskEvent {
            id: format!("{task_id}-{ts_offset_sec}-{:?}", event_type),
            ts: base + ChronoDuration::seconds(ts_offset_sec),
            event_type,
            task_id: task_id.to_string(),
            agent: "test".to_string(),
            node: "test".to_string(),
            payload,
            version: 1,
        }
    }

    #[test]
    fn dream_with_mix_of_events_produces_correct_patterns() {
        let events = vec![
            event(TaskEventType::DagChanged, "task-a", 0, json!({})),
            event(
                TaskEventType::GateRejected,
                "task-a",
                60,
                json!({"gate": "GATE 3", "files": ["src/shared.rs"]}),
            ),
            event(
                TaskEventType::GateRejected,
                "task-b",
                120,
                json!({"gate": "GATE 4", "files": ["src/shared.rs", "src/other.rs"]}),
            ),
            event(TaskEventType::MergeVerified, "task-a", 7200, json!({})),
        ];

        let report = analyze_events(&events);

        assert_eq!(report.events_processed, 4);
        assert_eq!(report.patterns.gate_rejections.get("GATE 3"), Some(&1));
        assert_eq!(report.patterns.gate_rejections.get("GATE 4"), Some(&1));
        assert_eq!(
            report.patterns.lock_conflicts.get("src/shared.rs"),
            Some(&2)
        );
        assert_eq!(report.patterns.lock_conflicts.get("src/other.rs"), Some(&1));
        assert_eq!(report.patterns.completion_times.len(), 1);
        assert_eq!(report.patterns.completion_times[0].0, "task-a");
        assert_eq!(
            report.patterns.completion_times[0].1,
            Duration::from_secs(7200)
        );
    }

    #[test]
    fn learning_extraction_from_gate_rejected_events() {
        let events = vec![
            event(
                TaskEventType::GateRejected,
                "task-a",
                0,
                json!({"gate": "GATE 3"}),
            ),
            event(
                TaskEventType::GateRejected,
                "task-b",
                10,
                json!({"gate": "GATE 3"}),
            ),
            event(
                TaskEventType::GateRejected,
                "task-c",
                20,
                json!({"gate": "GATE 3"}),
            ),
        ];

        let report = analyze_events(&events);
        assert!(report.learnings.iter().any(|learning| {
            learning.title.contains("GATE 3")
                && learning.importance == Importance::High
                && learning.content.contains("手順書の改善が必要")
        }));
    }

    #[test]
    fn empty_events_produce_empty_report() {
        let report = analyze_events(&[]);
        assert_eq!(report.events_processed, 0);
        assert!(report.patterns.gate_rejections.is_empty());
        assert!(report.patterns.lock_conflicts.is_empty());
        assert!(report.patterns.completion_times.is_empty());
        assert!(report.learnings.is_empty());
    }
}
