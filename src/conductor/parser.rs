// Conductor track parser — loads tracks from conductor directory.
//
// Adapted from conductor-dashboard with heading-level support for both
// H2 (`## [x] Track: Title`) and H3 (`### [x] Track 01: Title`) formats.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, NaiveDate, Utc};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::model::*;

// ---------------------------------------------------------------------------
// ParseError
// ---------------------------------------------------------------------------

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("tracks.md not found at {0}")]
    IndexNotFound(std::path::PathBuf),

    #[error("Invalid metadata for track {track_id}: {message}")]
    MetadataInvalid { track_id: String, message: String },

    #[error("Failed to read {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}

// ---------------------------------------------------------------------------
// Index entry (intermediate from tracks.md)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct IndexEntry {
    id: TrackId,
    title: String,
    checkbox: CheckboxStatus,
    status: Status,
    priority: Priority,
    tags: Vec<String>,
    branch: Option<String>,
    dependencies: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API: load all tracks
// ---------------------------------------------------------------------------

pub fn load_all_tracks(conductor_dir: &Path) -> Result<BTreeMap<TrackId, Track>, ParseError> {
    let mut tracks = parse_index(conductor_dir)?;

    let tracks_dir = conductor_dir.join("tracks");

    for (id, track) in tracks.iter_mut() {
        let track_dir = tracks_dir.join(id.as_str());

        // Load metadata.json
        match parse_metadata(&track_dir, id.as_str()) {
            Ok(Some(meta)) => track.merge_metadata(meta),
            Ok(None) => {}
            Err(e) => {
                log::warn!("conductor: failed to parse metadata for {}: {}", id, e);
            }
        }

        // Load plan.md
        let plan_path = track_dir.join("plan.md");
        if plan_path.exists() {
            match parse_plan(&plan_path) {
                Ok(phases) => track.merge_plan(phases),
                Err(e) => {
                    log::warn!("conductor: failed to parse plan for {}: {}", id, e);
                }
            }
        }
    }

    // Auto-complete tasks for tracks marked as done
    for track in tracks.values_mut() {
        if track.status == Status::Complete {
            track.mark_all_tasks_complete();
        }
    }

    Ok(tracks)
}

// ---------------------------------------------------------------------------
// Index parser (tracks.md)
// ---------------------------------------------------------------------------

fn parse_index(conductor_dir: &Path) -> Result<BTreeMap<TrackId, Track>, ParseError> {
    let index_path = conductor_dir.join("tracks.md");
    let content = std::fs::read_to_string(&index_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ParseError::IndexNotFound(index_path.clone())
        } else {
            ParseError::Io {
                path: index_path.clone(),
                source: e,
            }
        }
    })?;

    let entries = parse_index_content(&content);

    let mut tracks = BTreeMap::new();
    for entry in entries {
        let status = if entry.status != Status::New {
            entry.status
        } else {
            entry.checkbox.to_status()
        };

        let track = Track {
            id: entry.id.clone(),
            title: entry.title,
            status,
            priority: entry.priority,
            checkbox_status: entry.checkbox,
            tags: entry.tags,
            branch: entry.branch,
            dependencies: entry.dependencies.into_iter().map(TrackId::new).collect(),
            ..Track::default()
        };
        tracks.insert(entry.id, track);
    }

    Ok(tracks)
}

/// Parse the raw markdown content of tracks.md into index entries.
/// Supports both H2 and H3 headings (VeloTerm uses H3).
pub(crate) fn parse_index_content(content: &str) -> Vec<IndexEntry> {
    let opts = Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(content, opts);

    let mut entries = Vec::new();
    let mut in_track_heading = false;
    let mut heading_text = String::new();
    let mut _heading_level: Option<HeadingLevel> = None;
    let mut current_entry: Option<IndexEntry> = None;
    let mut in_paragraph = false;
    let mut in_item = false;
    let mut in_strong = false;
    let mut strong_text = String::new();
    let mut field_key: Option<String> = None;

    for event in parser {
        match event {
            // H2 or H3 headings can contain track entries
            Event::Start(Tag::Heading { level, .. })
                if level == HeadingLevel::H2 || level == HeadingLevel::H3 =>
            {
                // Flush previous entry
                if let Some(entry) = current_entry.take() {
                    entries.push(entry);
                }
                in_track_heading = true;
                _heading_level = Some(level);
                heading_text.clear();
            }

            Event::End(TagEnd::Heading(level))
                if level == HeadingLevel::H2 || level == HeadingLevel::H3 =>
            {
                in_track_heading = false;
                if let Some(entry) = parse_track_heading(&heading_text) {
                    current_entry = Some(entry);
                }
                _heading_level = None;
            }

            // Bold text (for field keys like **Priority**)
            Event::Start(Tag::Strong) => {
                in_strong = true;
                strong_text.clear();
            }
            Event::End(TagEnd::Strong) => {
                in_strong = false;
                if current_entry.is_some() {
                    let key = strong_text.trim_end_matches(':').trim().to_string();
                    field_key = Some(key);
                }
            }

            Event::Start(Tag::Paragraph) => {
                in_paragraph = true;
            }
            Event::End(TagEnd::Paragraph) => {
                in_paragraph = false;
                field_key = None;
            }

            Event::Start(Tag::Item) => {
                in_item = true;
            }
            Event::End(TagEnd::Item) => {
                in_item = false;
                field_key = None;
            }

            // Links — extract track ID from link target
            Event::Start(Tag::Link { dest_url, .. }) => {
                if let Some(ref mut entry) = current_entry {
                    if entry.id.as_str().is_empty() {
                        if let Some(track_id) = extract_track_id_from_link(&dest_url) {
                            entry.id = TrackId::new(track_id);
                        }
                    }
                }
            }

            Event::Text(text) => {
                if in_track_heading {
                    heading_text.push_str(&text);
                } else if in_strong {
                    strong_text.push_str(&text);
                } else if let Some(ref mut entry) = current_entry {
                    if in_paragraph || in_item {
                        if let Some(ref key) = field_key {
                            let value = text.trim();
                            if value.starts_with(':') {
                                let value = value.trim_start_matches(':').trim();
                                apply_field(entry, key, value);
                                field_key = None;
                            } else if !value.is_empty() {
                                apply_field(entry, key, value);
                                field_key = None;
                            }
                        }
                    }
                }
            }

            Event::Rule => {}
            _ => {}
        }
    }

    // Flush last entry
    if let Some(entry) = current_entry {
        entries.push(entry);
    }

    entries
}

/// Parse a track heading line. Supports both formats:
/// - `[x] Track: Dashboard UI Overhaul`  (conductor-dashboard original)
/// - `[x] Track 01: Cross-Platform Window...`  (VeloTerm format)
fn parse_track_heading(text: &str) -> Option<IndexEntry> {
    let text = text.trim();

    // Must contain "Track" to be a track entry
    if !text.contains("Track") {
        return None;
    }

    // Must have a colon after "Track" (possibly with a number between)
    let track_pos = text.find("Track")?;
    let after_track = &text[track_pos + "Track".len()..];

    // Find the colon — may be "Track:" or "Track 01:" or "Track 25:"
    let colon_pos = after_track.find(':')?;
    let title_start = &after_track[colon_pos + 1..];

    // Parse checkbox: [x], [ ], [~], [-]
    let checkbox = if text.starts_with("[x]") || text.starts_with("[X]") {
        CheckboxStatus::Checked
    } else if text.starts_with("[~]") || text.starts_with("[-]") {
        CheckboxStatus::InProgress
    } else if text.starts_with("[ ]") {
        CheckboxStatus::Unchecked
    } else {
        CheckboxStatus::Unchecked
    };

    // Extract title: everything after the colon, trimmed of size markers and status emojis
    let title = title_start
        .split('\u{2705}') // ✅
        .next()
        .unwrap_or(title_start)
        .trim()
        .trim_end_matches("(S)")
        .trim_end_matches("(M)")
        .trim_end_matches("(L)")
        .trim_end_matches("(XL)")
        .trim()
        .to_string();

    if title.is_empty() {
        return None;
    }

    Some(IndexEntry {
        id: TrackId::new(""),
        title,
        checkbox,
        status: Status::New,
        priority: Priority::Medium,
        tags: Vec::new(),
        branch: None,
        dependencies: Vec::new(),
    })
}

/// Extract track ID from a link like `./conductor/tracks/some_track_id/`
fn extract_track_id_from_link(url: &str) -> Option<String> {
    let url = url.trim_end_matches('/');
    if let Some(pos) = url.rfind("/tracks/") {
        let after = &url[pos + "/tracks/".len()..];
        let id = after.trim_end_matches('/');
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    url.rsplit('/').next().map(|s| s.to_string())
}

fn apply_field(entry: &mut IndexEntry, key: &str, value: &str) {
    let value = value.trim();
    match key {
        "Priority" => entry.priority = Priority::from_str_loose(value),
        "Status" => entry.status = Status::from_str_loose(value),
        "Tags" => {
            entry.tags = value
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
        }
        "Branch" => {
            let branch = value.trim_matches('`').to_string();
            if !branch.is_empty() {
                entry.branch = Some(branch);
            }
        }
        "ID" | "Id" => {
            let id = value.trim_matches('`').to_string();
            if !id.is_empty() && entry.id.as_str().is_empty() {
                entry.id = TrackId::new(id);
            }
        }
        "Dependencies" | "Depends on" => {
            entry.dependencies = value
                .split(',')
                .map(|d| d.trim().trim_matches('`').trim().to_string())
                .filter(|d| !d.is_empty())
                .collect();
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Plan parser (plan.md)
// ---------------------------------------------------------------------------

fn parse_plan(plan_path: &Path) -> Result<Vec<PlanPhase>, ParseError> {
    let content = std::fs::read_to_string(plan_path).map_err(|e| ParseError::Io {
        path: plan_path.to_path_buf(),
        source: e,
    })?;
    Ok(parse_plan_content(&content))
}

pub(crate) fn parse_plan_content(content: &str) -> Vec<PlanPhase> {
    let opts = Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(content, opts);

    let mut phases: Vec<PlanPhase> = Vec::new();
    let mut in_heading = false;
    let mut heading_text = String::new();
    let mut in_task_item = false;
    let mut task_text = String::new();
    let mut task_done = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                flush_task(&mut phases, &mut in_task_item, &mut task_text, &task_done);
                in_heading = true;
                heading_text.clear();
            }

            Event::End(TagEnd::Heading(level)) => {
                in_heading = false;
                let name = heading_text.trim().to_string();
                if (level == HeadingLevel::H2 || level == HeadingLevel::H3)
                    && is_phase_heading(&name)
                {
                    phases.push(PlanPhase {
                        name,
                        status: PhaseStatus::Pending,
                        tasks: Vec::new(),
                    });
                }
            }

            Event::TaskListMarker(checked) => {
                flush_task(&mut phases, &mut in_task_item, &mut task_text, &task_done);
                in_task_item = true;
                task_done = checked;
                task_text.clear();
            }

            Event::End(TagEnd::Item) => {
                flush_task(&mut phases, &mut in_task_item, &mut task_text, &task_done);
            }

            Event::Text(text) => {
                if in_heading {
                    heading_text.push_str(&text);
                } else if in_task_item {
                    task_text.push_str(&text);
                }
            }

            Event::Code(code) => {
                if in_heading {
                    heading_text.push_str(&code);
                } else if in_task_item {
                    task_text.push('`');
                    task_text.push_str(&code);
                    task_text.push('`');
                }
            }

            Event::SoftBreak | Event::HardBreak => {
                if in_heading {
                    heading_text.push(' ');
                } else if in_task_item {
                    task_text.push(' ');
                }
            }

            _ => {}
        }
    }

    flush_task(&mut phases, &mut in_task_item, &mut task_text, &task_done);
    compute_phase_statuses(&mut phases);
    phases
}

fn flush_task(
    phases: &mut Vec<PlanPhase>,
    in_task_item: &mut bool,
    task_text: &mut String,
    task_done: &bool,
) {
    if !*in_task_item {
        return;
    }
    let text = task_text.trim().strip_prefix("Task:").unwrap_or(task_text.trim()).trim().to_string();
    if !text.is_empty() {
        if phases.is_empty() {
            phases.push(PlanPhase {
                name: "Tasks".to_string(),
                status: PhaseStatus::Pending,
                tasks: Vec::new(),
            });
        }
        phases.last_mut().unwrap().tasks.push(PlanTask {
            text,
            done: *task_done,
        });
    }
    *in_task_item = false;
    task_text.clear();
}

fn is_phase_heading(name: &str) -> bool {
    name.to_ascii_lowercase().contains("phase")
}

fn compute_phase_statuses(phases: &mut [PlanPhase]) {
    let mut found_active = false;

    for phase in phases.iter_mut() {
        if phase.tasks.is_empty() {
            phase.status = PhaseStatus::Pending;
            continue;
        }

        let all_done = phase.tasks.iter().all(|t| t.done);
        let any_done = phase.tasks.iter().any(|t| t.done);

        if all_done {
            phase.status = PhaseStatus::Complete;
        } else if any_done || !found_active {
            phase.status = PhaseStatus::Active;
            found_active = true;
        } else {
            phase.status = PhaseStatus::Pending;
        }
    }

    if !found_active {
        if let Some(phase) = phases
            .iter_mut()
            .find(|p| p.status == PhaseStatus::Pending && !p.tasks.is_empty())
        {
            phase.status = PhaseStatus::Active;
        }
    }
}

// ---------------------------------------------------------------------------
// Metadata parser (metadata.json)
// ---------------------------------------------------------------------------

use serde::Deserialize;

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
struct RawJsonMetadata {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    track_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default, rename = "type")]
    track_type: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    end_date: Option<String>,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

fn parse_metadata(
    track_dir: &Path,
    track_id: &str,
) -> Result<Option<TrackMetadata>, ParseError> {
    let json_path = track_dir.join("metadata.json");
    if !json_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&json_path).map_err(|e| ParseError::Io {
        path: json_path.clone(),
        source: e,
    })?;

    let raw: RawJsonMetadata =
        serde_json::from_str(&content).map_err(|e| ParseError::MetadataInvalid {
            track_id: track_id.to_string(),
            message: e.to_string(),
        })?;

    let created_at = raw
        .created_at
        .as_deref()
        .or(raw.start_date.as_deref())
        .and_then(parse_datetime);

    let updated_at = raw
        .updated_at
        .as_deref()
        .or(raw.end_date.as_deref())
        .and_then(parse_datetime);

    Ok(Some(TrackMetadata {
        status: raw.status.as_deref().map(Status::from_str_loose).unwrap_or_default(),
        priority: raw.priority.as_deref().map(Priority::from_str_loose).unwrap_or_default(),
        track_type: raw.track_type.as_deref().map(TrackType::from_str_loose).unwrap_or_default(),
        created_at,
        updated_at,
        dependencies: raw.dependencies,
        tags: raw.tags,
        branch: raw.branch,
        description: raw.description,
    }))
}

fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim().trim_matches('(').trim_matches(')').trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Track heading parsing ──────────────────────────────────

    #[test]
    fn parse_h3_veloterm_format_checked() {
        let entry = parse_track_heading("[x] Track 01: Cross-Platform Window and GPU Rendering Pipeline").unwrap();
        assert_eq!(entry.checkbox, CheckboxStatus::Checked);
        assert_eq!(entry.title, "Cross-Platform Window and GPU Rendering Pipeline");
    }

    #[test]
    fn parse_h3_veloterm_format_with_size() {
        let entry = parse_track_heading("[x] Track 03: Configuration & Theming (M)").unwrap();
        assert_eq!(entry.checkbox, CheckboxStatus::Checked);
        assert_eq!(entry.title, "Configuration & Theming");
    }

    #[test]
    fn parse_h2_original_format_checked() {
        let entry = parse_track_heading("[x] Track: Dashboard UI Overhaul \u{2705} COMPLETE").unwrap();
        assert_eq!(entry.checkbox, CheckboxStatus::Checked);
        assert_eq!(entry.title, "Dashboard UI Overhaul");
    }

    #[test]
    fn parse_h2_original_format_unchecked() {
        let entry = parse_track_heading("[ ] Track: Compliance Workflow Enhancements").unwrap();
        assert_eq!(entry.checkbox, CheckboxStatus::Unchecked);
        assert_eq!(entry.title, "Compliance Workflow Enhancements");
    }

    #[test]
    fn parse_h2_in_progress() {
        let entry = parse_track_heading("[~] Track: Chatbot Robustness Hardening").unwrap();
        assert_eq!(entry.checkbox, CheckboxStatus::InProgress);
    }

    #[test]
    fn parse_heading_no_track_marker() {
        assert!(parse_track_heading("Wave 0 — Foundation (COMPLETE)").is_none());
    }

    #[test]
    fn parse_heading_no_colon() {
        assert!(parse_track_heading("[x] Track Progress Summary").is_none());
    }

    // ── Index content parsing (VeloTerm format) ────────────────

    #[test]
    fn parse_veloterm_index() {
        let md = r#"# Project Tracks

---

## Wave 0 — Foundation (COMPLETE)

### [x] Track 01: Cross-Platform Window and GPU Rendering Pipeline
_Link: [./conductor/tracks/window_gpu_pipeline_20260207/](./conductor/tracks/window_gpu_pipeline_20260207/)_

### [x] Track 02: Core Terminal Emulation
_Link: [./conductor/tracks/core_terminal_emulation_20260207/](./conductor/tracks/core_terminal_emulation_20260207/)_

---

## Wave 1 — Configuration & Optimization (COMPLETE)

### [x] Track 03: Configuration & Theming (M)
_Link: [./conductor/tracks/03_config_20260207/](./conductor/tracks/03_config_20260207/)_
"#;
        let entries = parse_index_content(md);
        assert_eq!(entries.len(), 3);

        assert_eq!(entries[0].title, "Cross-Platform Window and GPU Rendering Pipeline");
        assert_eq!(entries[0].id.as_str(), "window_gpu_pipeline_20260207");
        assert_eq!(entries[0].checkbox, CheckboxStatus::Checked);

        assert_eq!(entries[1].title, "Core Terminal Emulation");
        assert_eq!(entries[1].id.as_str(), "core_terminal_emulation_20260207");

        assert_eq!(entries[2].title, "Configuration & Theming");
        assert_eq!(entries[2].id.as_str(), "03_config_20260207");
    }

    // ── Index content parsing (original H2 format) ────────────

    #[test]
    fn parse_original_h2_index() {
        let md = r#"# Project Tracks

## [x] Track: Dashboard UI Overhaul ✅ COMPLETE
*Link: [./conductor/tracks/dashboard_overhaul_20260206/](./conductor/tracks/dashboard_overhaul_20260206/)*
**Priority**: High

---

## [ ] Track: Compliance Workflow
*Link: [./conductor/tracks/compliance_20260127/](./conductor/tracks/compliance_20260127/)*
"#;
        let entries = parse_index_content(md);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "Dashboard UI Overhaul");
        assert_eq!(entries[0].priority, Priority::High);
        assert_eq!(entries[1].title, "Compliance Workflow");
    }

    // ── Link ID extraction ─────────────────────────────────────

    #[test]
    fn extract_track_id_from_conductor_link() {
        assert_eq!(
            extract_track_id_from_link("./conductor/tracks/window_gpu_pipeline_20260207/"),
            Some("window_gpu_pipeline_20260207".to_string())
        );
    }

    #[test]
    fn extract_track_id_from_relative_link() {
        assert_eq!(
            extract_track_id_from_link("./tracks/03_config_20260207/"),
            Some("03_config_20260207".to_string())
        );
    }

    // ── Plan parsing ──────────────────────────────────────────

    #[test]
    fn plan_simple_phases() {
        let md = r#"# Implementation Plan

## Phase 1: Setup
- [x] Create project structure
- [x] Add dependencies
- [ ] Configure CI

## Phase 2: Implementation
- [ ] Build parser
- [ ] Add tests
"#;
        let phases = parse_plan_content(md);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].name, "Phase 1: Setup");
        assert_eq!(phases[0].tasks.len(), 3);
        assert!(phases[0].tasks[0].done);
        assert!(!phases[0].tasks[2].done);
        assert_eq!(phases[0].status, PhaseStatus::Active);

        assert_eq!(phases[1].name, "Phase 2: Implementation");
        assert_eq!(phases[1].tasks.len(), 2);
        assert_eq!(phases[1].status, PhaseStatus::Pending);
    }

    #[test]
    fn plan_all_complete() {
        let md = r#"## Phase 1: Done
- [x] A
- [x] B
"#;
        let phases = parse_plan_content(md);
        assert_eq!(phases[0].status, PhaseStatus::Complete);
    }

    #[test]
    fn plan_empty() {
        let phases = parse_plan_content("# Nothing\n\nJust a description.\n");
        assert!(phases.is_empty());
    }

    // ── Metadata parsing ──────────────────────────────────────

    #[test]
    fn parse_json_metadata_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("metadata.json"),
            r#"{"status": "complete", "priority": "high", "tags": ["ui"]}"#,
        ).unwrap();
        let meta = parse_metadata(dir.path(), "test").unwrap().unwrap();
        assert_eq!(meta.status, Status::Complete);
        assert_eq!(meta.priority, Priority::High);
        assert_eq!(meta.tags, vec!["ui"]);
    }

    #[test]
    fn parse_json_metadata_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let meta = parse_metadata(dir.path(), "test").unwrap();
        assert!(meta.is_none());
    }

    #[test]
    fn parse_json_metadata_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("metadata.json"), "{}").unwrap();
        let meta = parse_metadata(dir.path(), "test").unwrap().unwrap();
        assert_eq!(meta.status, Status::New);
        assert_eq!(meta.priority, Priority::Medium);
    }

    // ── Datetime parsing ──────────────────────────────────────

    #[test]
    fn parse_datetime_iso() {
        let dt = parse_datetime("2026-02-12T14:45:00Z").unwrap();
        assert_eq!(dt.year(), 2026);
    }

    #[test]
    fn parse_datetime_date_only() {
        let dt = parse_datetime("2026-02-04").unwrap();
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 4);
    }

    #[test]
    fn parse_datetime_invalid() {
        assert!(parse_datetime("not a date").is_none());
    }

    use chrono::Datelike;

    // ── H2 format with inline metadata ────────────────────────

    #[test]
    fn parse_h2_format_with_inline_metadata() {
        let content = r#"# Project Tracks

## [x] Track: Dashboard Build Remediation ✅ COMPLETE
*Link: [./conductor/tracks/dashboard_build_remediation_20260217/](./conductor/tracks/dashboard_build_remediation_20260217/)*
**Priority**: Critical
**Tags**: frontend, typescript, bugfix
**Status**: Completed (2026-02-17)
"#;
        let entries = parse_index_content(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Dashboard Build Remediation");
        assert_eq!(entries[0].id.as_str(), "dashboard_build_remediation_20260217");
        assert_eq!(entries[0].checkbox, CheckboxStatus::Checked);
    }

    // ── Integration: load VeloTerm's own conductor dir ────────

    #[test]
    fn load_veloterm_conductor_dir() {
        let conductor_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("conductor");
        if !conductor_dir.exists() {
            return; // Skip if no conductor dir
        }
        let tracks = load_all_tracks(&conductor_dir).unwrap();
        assert!(tracks.len() >= 25, "Expected at least 25 tracks, got {}", tracks.len());

        // All tracks should be complete (all 25 tracks are done)
        for (id, track) in &tracks {
            assert_eq!(
                track.status, Status::Complete,
                "Track {} ({}) should be complete",
                id, track.title
            );
        }
    }
}
