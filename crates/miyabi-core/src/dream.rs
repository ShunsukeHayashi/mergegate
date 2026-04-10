//! Dreaming over deterministic task events to extract patterns and learnings.

use crate::error::Result;
use crate::store::{EventStore, TaskEvent, TaskEventType};
use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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

pub fn dream_with_auto(
    event_store: &EventStore,
    since: Option<ChronoDuration>,
    auto: bool,
    repo_root: &Path,
) -> Result<DreamReport> {
    let events = collect_events(event_store, since)?;
    let report = analyze_events(&events);
    if auto {
        run_theta_cycle(&report, repo_root)?;
    }
    Ok(report)
}

pub fn write_high_learnings(report: &DreamReport, directory: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(directory)?;

    let mut written = Vec::new();
    let date_prefix = Utc::now().format("%Y-%m-%d").to_string();
    for learning in report
        .learnings
        .iter()
        .filter(|learning| learning.importance == Importance::High)
    {
        let path = next_learning_path(directory, &date_prefix, &slugify(&learning.title));
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

fn run_theta_cycle(report: &DreamReport, repo_root: &Path) -> Result<()> {
    run_theta_cycle_with_runner(report, repo_root, &mut run_command)
}

fn run_theta_cycle_with_runner<F>(report: &DreamReport, repo_root: &Path, run: &mut F) -> Result<()>
where
    F: FnMut(&str, &[String], &Path) -> Result<()>,
{
    let score = average_learning_score(report);
    run(
        "npx",
        &[
            "miyabi".to_string(),
            "bus".to_string(),
            "record-run".to_string(),
            "polaris".to_string(),
            "--result".to_string(),
            "success".to_string(),
            "--score".to_string(),
            format!("{score:.2}"),
            "--task".to_string(),
            "dream cycle".to_string(),
        ],
        repo_root,
    )?;

    let learning_dir = repo_root.join("docs").join("learnings");
    let high_learnings = report
        .learnings
        .iter()
        .filter(|learning| learning.importance == Importance::High)
        .collect::<Vec<_>>();
    let written = write_high_learnings(report, &learning_dir)?;

    for (path, learning) in written.iter().zip(high_learnings) {
        let relative_path = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .display()
            .to_string();
        run("git", &[String::from("add"), relative_path], repo_root)?;
        run(
            "git",
            &[
                String::from("commit"),
                String::from("-m"),
                format!("[学習] {}", learning.title),
            ],
            repo_root,
        )?;
    }

    // θ6: Auto-update SKILL.md with drift corrections from gate rejections
    if !report.patterns.gate_rejections.is_empty() {
        update_skill_md_from_patterns(report, repo_root)?;
    }

    Ok(())
}

/// Append a "Common Rejection Patterns" section to SKILL.md if gate rejections are detected.
fn update_skill_md_from_patterns(report: &DreamReport, repo_root: &Path) -> Result<()> {
    let skill_path = repo_root.join("skills/polaris-ops/SKILL.md");
    if !skill_path.exists() {
        return Ok(());
    }

    let existing = fs::read_to_string(&skill_path)?;
    let marker = "## よくある拒否パターン（自動生成）";
    let new_section = build_rejection_section(&report.patterns.gate_rejections);

    let content = if let Some(start_idx) = existing.find(marker) {
        // Replace only the auto-generated section, preserving content after it
        let after_marker = &existing[start_idx + marker.len()..];
        // Find the next ## heading (or EOF) to determine section boundary
        let section_end = after_marker
            .find("\n## ")
            .map(|i| start_idx + marker.len() + i + 1) // +1 to keep the newline before next heading
            .unwrap_or(existing.len());
        let mut content = existing[..start_idx].trim_end().to_string();
        content.push_str("\n\n");
        content.push_str(&new_section);
        content.push_str(&existing[section_end..]);
        content
    } else {
        let mut content = existing;
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push('\n');
        content.push_str(&new_section);
        content.push('\n');
        content
    };

    let tmp = skill_path.with_extension("md.tmp");
    fs::write(&tmp, &content)?;
    fs::rename(&tmp, &skill_path)?;
    Ok(())
}

fn build_rejection_section(gate_rejections: &HashMap<String, usize>) -> String {
    let mut section = String::from("## よくある拒否パターン（自動生成）\n\n");
    section.push_str("| GATE | 回数 | 対処法 |\n|------|------|--------|\n");
    let mut entries: Vec<_> = gate_rejections.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1));
    for (gate, count) in entries {
        let remedy = match gate.as_str() {
            "GATE 0" | "gate_0" => "Issue を先に作成する",
            "GATE 2" | "gate_2" => "依存タスクを完了してから実行",
            "GATE 3" | "gate_3" => "impact --approve で承認を付ける",
            "GATE 4" | "gate_4" => "assign でロック獲得してから編集",
            "GATE 5" | "gate_5" => "ブランチ名を feature/issue-N-slug 形式に",
            _ => "手順を確認して条件を満たす",
        };
        section.push_str(&format!("| {gate} | {count} | {remedy} |\n"));
    }
    section
}

pub fn obsidian_export(learning: &Learning, vault_path: Option<&Path>) -> Result<PathBuf> {
    let root = vault_path
        .map(Path::to_path_buf)
        .unwrap_or_else(default_obsidian_vault_path);
    let date = Utc::now().format("%Y-%m-%d").to_string();
    let note_dir = root.join("Docs-Operations").join("dtp-learnings");
    fs::create_dir_all(&note_dir)?;

    let filename = format!("{date}-{}.md", slugify(&learning.title));
    let path = note_dir.join(filename);
    let tmp_path = path.with_extension("md.tmp");
    let created = Utc::now().to_rfc3339();
    let tags = format!(
        "- dtp\n- learning\n- importance/{}",
        importance_slug(learning.importance)
    );
    let content = format!(
        "---\n\
type: dtp_learning\n\
domain: docs-operations\n\
status: captured\n\
tags:\n\
{tags}\n\
created: {created}\n\
---\n\n\
# {title}\n\n\
- importance: {:?}\n\
- related_task: {}\n\n\
{body}\n\n\
[[DTP Learnings MOC]]\n",
        learning.importance,
        learning.related_task.as_deref().unwrap_or("none"),
        title = learning.title,
        body = learning.content,
    );
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, &path)?;
    Ok(path)
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

fn average_learning_score(report: &DreamReport) -> f32 {
    if report.learnings.is_empty() {
        return 1.0;
    }

    let total: f32 = report
        .learnings
        .iter()
        .map(|learning| match learning.importance {
            Importance::High => 1.0,
            Importance::Medium => 0.7,
            Importance::Low => 0.4,
        })
        .sum();
    total / report.learnings.len() as f32
}

fn next_learning_path(directory: &Path, date_prefix: &str, slug: &str) -> PathBuf {
    let base_name = format!("{date_prefix}-{slug}");
    let initial = directory.join(format!("{base_name}.md"));
    if !initial.exists() {
        return initial;
    }

    let mut suffix = 2;
    loop {
        let candidate = directory.join(format!("{base_name}-{suffix}.md"));
        if !candidate.exists() {
            return candidate;
        }
        suffix += 1;
    }
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

fn default_obsidian_vault_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join("dev")
        .join("content")
        .join("obsidian")
}

fn importance_slug(importance: Importance) -> &'static str {
    match importance {
        Importance::High => "high",
        Importance::Medium => "medium",
        Importance::Low => "low",
    }
}

fn run_command(program: &str, args: &[String], cwd: &Path) -> Result<()> {
    let status = Command::new(program).args(args).current_dir(cwd).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(crate::error::Error::Other(format!(
            "command failed: {} {}",
            program,
            args.join(" ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::TaskEvent;
    use chrono::{Duration as ChronoDuration, TimeZone};
    use serde_json::json;
    use tempfile::TempDir;

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

    #[test]
    fn obsidian_export_creates_valid_markdown_with_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let learning = Learning {
            title: "GATE 3 の拒否が多発".to_string(),
            importance: Importance::High,
            content: "手順書の改善が必要です。".to_string(),
            related_task: Some("task-3".to_string()),
        };

        let path = obsidian_export(&learning, Some(tmp.path())).unwrap();
        let note = fs::read_to_string(&path).unwrap();

        assert!(path.starts_with(tmp.path().join("Docs-Operations").join("dtp-learnings")));
        assert!(note.starts_with("---\n"));
        assert!(note.contains("type: dtp_learning"));
        assert!(note.contains("domain: docs-operations"));
        assert!(note.contains("status: captured"));
        assert!(note.contains("- dtp"));
        assert!(note.contains("[[DTP Learnings MOC]]"));
    }

    #[test]
    fn dream_with_auto_produces_learning_files() {
        let tmp = TempDir::new().unwrap();
        let event_store = EventStore::new(tmp.path().join("events.jsonl"));
        let base = Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap();
        for index in 0..3 {
            event_store
                .append(&TaskEvent {
                    id: format!("gate-{index}"),
                    ts: base + ChronoDuration::seconds(index),
                    event_type: TaskEventType::GateRejected,
                    task_id: format!("task-{index}"),
                    agent: "test".into(),
                    node: "test".into(),
                    payload: json!({"gate": "GATE 3"}),
                    version: index as u64 + 1,
                })
                .unwrap();
        }

        let report = dream_with_auto(&event_store, None, false, tmp.path()).unwrap();
        let mut calls = Vec::new();
        run_theta_cycle_with_runner(&report, tmp.path(), &mut |program, args, _cwd| {
            calls.push((program.to_string(), args.to_vec()));
            Ok(())
        })
        .unwrap();

        let learning_dir = tmp.path().join("docs").join("learnings");
        let entries = fs::read_dir(&learning_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        let content = fs::read_to_string(&entries[0]).unwrap();
        assert!(content.contains("GATE 3"));
        assert!(
            calls
                .iter()
                .any(|(program, args)| program == "npx"
                    && args.first() == Some(&"miyabi".to_string()))
        );
        assert!(calls
            .iter()
            .any(|(program, args)| program == "git" && args.first() == Some(&"add".to_string())));
        assert!(
            calls
                .iter()
                .any(|(program, args)| program == "git"
                    && args.first() == Some(&"commit".to_string()))
        );
    }

    #[test]
    fn build_rejection_section_empty() {
        let section = build_rejection_section(&HashMap::new());
        assert!(section.contains("| GATE | 回数 | 対処法 |"));
        // heading + blank + table header + separator = 4 lines, no data rows
        assert_eq!(section.lines().count(), 4);
    }

    #[test]
    fn build_rejection_section_sorted_by_count_desc() {
        let mut rejections = HashMap::new();
        rejections.insert("GATE 0".to_string(), 1);
        rejections.insert("GATE 3".to_string(), 5);
        rejections.insert("GATE 4".to_string(), 3);

        let section = build_rejection_section(&rejections);
        let lines: Vec<&str> = section.lines().collect();
        // lines: [0]=heading, [1]=blank, [2]=table header, [3]=separator, [4..]=data
        assert!(lines[4].contains("GATE 3"));
        assert!(lines[4].contains("5"));
        assert!(lines[5].contains("GATE 4"));
        assert!(lines[6].contains("GATE 0"));
    }

    #[test]
    fn update_skill_md_appends_when_no_marker() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join("skills/polaris-ops");
        fs::create_dir_all(&skills_dir).unwrap();
        let skill_path = skills_dir.join("SKILL.md");
        fs::write(&skill_path, "# Polaris Ops\n\nExisting content.\n").unwrap();

        let mut rejections = HashMap::new();
        rejections.insert("GATE 3".to_string(), 2);
        let report = DreamReport {
            patterns: DreamPatterns {
                gate_rejections: rejections,
                ..Default::default()
            },
            learnings: Vec::new(),
            events_processed: 0,
        };

        update_skill_md_from_patterns(&report, tmp.path()).unwrap();
        let content = fs::read_to_string(&skill_path).unwrap();
        assert!(content.contains("# Polaris Ops"));
        assert!(content.contains("Existing content."));
        assert!(content.contains("## よくある拒否パターン（自動生成）"));
        assert!(content.contains("GATE 3"));
    }

    #[test]
    fn update_skill_md_replaces_marker_preserves_following_sections() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join("skills/polaris-ops");
        fs::create_dir_all(&skills_dir).unwrap();
        let skill_path = skills_dir.join("SKILL.md");
        fs::write(
            &skill_path,
            "# Polaris Ops\n\n\
             ## よくある拒否パターン（自動生成）\n\n\
             | GATE | 回数 | 対処法 |\n|------|------|--------|\n| old | 1 | old |\n\n\
             ## 関連スキル\n\n- important link\n",
        )
        .unwrap();

        let mut rejections = HashMap::new();
        rejections.insert("GATE 4".to_string(), 7);
        let report = DreamReport {
            patterns: DreamPatterns {
                gate_rejections: rejections,
                ..Default::default()
            },
            learnings: Vec::new(),
            events_processed: 0,
        };

        update_skill_md_from_patterns(&report, tmp.path()).unwrap();
        let content = fs::read_to_string(&skill_path).unwrap();
        // Old data should be gone
        assert!(!content.contains("| old |"));
        // New data should be present
        assert!(content.contains("GATE 4"));
        assert!(content.contains("7"));
        // Following section MUST be preserved
        assert!(content.contains("## 関連スキル"));
        assert!(content.contains("- important link"));
    }
}
