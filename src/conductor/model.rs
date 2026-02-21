// Conductor dashboard data model — track, phase, task types.
//
// Adapted from conductor-dashboard. UI-independent pure Rust types.

use std::fmt;

use chrono::{DateTime, Utc};

// ---------------------------------------------------------------------------
// TrackId — newtype for type safety
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TrackId(pub String);

impl TrackId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TrackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for TrackId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TrackId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Track status (from tracks.md checkbox + metadata)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Status {
    #[default]
    New,
    InProgress,
    Blocked,
    Complete,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::New => "New",
            Self::InProgress => "Active",
            Self::Blocked => "Blocked",
            Self::Complete => "Complete",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        let lower = s.to_ascii_lowercase();
        let lower = lower.trim();
        match lower {
            "complete" | "completed" | "done" => Self::Complete,
            "in_progress" | "in-progress" | "active" | "implementation" => Self::InProgress,
            "blocked" | "on_hold" => Self::Blocked,
            _ => Self::New,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ---------------------------------------------------------------------------
// Priority
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Priority {
    Critical = 0,
    High = 1,
    #[default]
    Medium = 2,
    Low = 3,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        let lower = s.to_ascii_lowercase();
        let lower = lower.trim();
        match lower {
            "critical" => Self::Critical,
            "high" => Self::High,
            "medium" | "med" => Self::Medium,
            "low" => Self::Low,
            _ => Self::Medium,
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ---------------------------------------------------------------------------
// Checkbox status (from tracks.md headings)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CheckboxStatus {
    #[default]
    Unchecked,
    InProgress,
    Checked,
}

impl CheckboxStatus {
    pub fn to_status(self) -> Status {
        match self {
            Self::Unchecked => Status::New,
            Self::InProgress => Status::InProgress,
            Self::Checked => Status::Complete,
        }
    }
}

// ---------------------------------------------------------------------------
// Phase status (derived from task completion within a phase)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PhaseStatus {
    #[default]
    Pending,
    Active,
    Complete,
    Blocked,
}

impl PhaseStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Active => "Active",
            Self::Complete => "Complete",
            Self::Blocked => "Blocked",
        }
    }
}

impl fmt::Display for PhaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ---------------------------------------------------------------------------
// Track type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum TrackType {
    Feature,
    Bug,
    Migration,
    Refactor,
    #[default]
    Other,
}

impl TrackType {
    pub fn label(&self) -> &str {
        match self {
            Self::Feature => "FEATURE",
            Self::Bug => "BUG",
            Self::Migration => "MIGRATION",
            Self::Refactor => "REFACTOR",
            Self::Other => "TRACK",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        let lower = s.to_ascii_lowercase();
        match lower.trim() {
            "feature" | "feat" => Self::Feature,
            "bug" | "bugfix" | "fix" => Self::Bug,
            "migration" | "migrate" => Self::Migration,
            "refactor" | "refactoring" => Self::Refactor,
            _ => Self::Other,
        }
    }
}

impl fmt::Display for TrackType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ---------------------------------------------------------------------------
// Filter / Sort modes (UI state)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    #[default]
    All,
    Active,
    Blocked,
    Complete,
    New,
}

impl FilterMode {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Active,
            Self::Active => Self::Blocked,
            Self::Blocked => Self::Complete,
            Self::Complete => Self::New,
            Self::New => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Active => "Active",
            Self::Blocked => "Blocked",
            Self::Complete => "Done",
            Self::New => "New",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Id,
    Progress,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            Self::Id => Self::Progress,
            Self::Progress => Self::Id,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Progress => "Progress",
        }
    }
}

// ---------------------------------------------------------------------------
// Track — the core data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Track {
    pub id: TrackId,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    pub track_type: TrackType,
    pub phase: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub dependencies: Vec<TrackId>,
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub checkbox_status: CheckboxStatus,
    pub plan_phases: Vec<PlanPhase>,
    pub tags: Vec<String>,
    pub branch: Option<String>,
    pub description: Option<String>,
}

impl Track {
    pub fn progress_percent(&self) -> f32 {
        if self.tasks_total == 0 {
            return 0.0;
        }
        (self.tasks_completed as f32 / self.tasks_total as f32) * 100.0
    }

    pub fn is_complete(&self) -> bool {
        self.status == Status::Complete
            || (self.tasks_total > 0 && self.tasks_completed == self.tasks_total)
    }

    pub fn merge_metadata(&mut self, meta: TrackMetadata) {
        if meta.status != Status::New {
            self.status = meta.status;
        }
        if meta.priority != Priority::Medium {
            self.priority = meta.priority;
        }
        if meta.track_type != TrackType::Other {
            self.track_type = meta.track_type;
        }
        if let Some(dt) = meta.created_at {
            self.created_at = Some(dt);
        }
        if let Some(dt) = meta.updated_at {
            self.updated_at = Some(dt);
        }
        if !meta.dependencies.is_empty() {
            self.dependencies = meta.dependencies.into_iter().map(TrackId::new).collect();
        }
        if !meta.tags.is_empty() {
            self.tags = meta.tags;
        }
        if meta.branch.is_some() {
            self.branch = meta.branch;
        }
        if meta.description.is_some() {
            self.description = meta.description;
        }
    }

    pub fn mark_all_tasks_complete(&mut self) {
        for phase in &mut self.plan_phases {
            for task in &mut phase.tasks {
                task.done = true;
            }
            phase.status = PhaseStatus::Complete;
        }
        self.tasks_completed = self.tasks_total;
    }

    pub fn merge_plan(&mut self, phases: Vec<PlanPhase>) {
        let (total, completed) = phases.iter().fold((0usize, 0usize), |(t, c), phase| {
            let phase_total = phase.tasks.len();
            let phase_done = phase.tasks.iter().filter(|t| t.done).count();
            (t + phase_total, c + phase_done)
        });
        self.tasks_total = total;
        self.tasks_completed = completed;
        self.plan_phases = phases;

        if let Some(active) = self
            .plan_phases
            .iter()
            .find(|p| p.status == PhaseStatus::Active || p.status == PhaseStatus::Pending)
        {
            self.phase = active.name.clone();
        } else if let Some(last) = self.plan_phases.last() {
            self.phase = last.name.clone();
        }
    }
}

impl Default for Track {
    fn default() -> Self {
        Self {
            id: TrackId::new(""),
            title: String::new(),
            status: Status::New,
            priority: Priority::Medium,
            track_type: TrackType::Other,
            phase: String::new(),
            created_at: None,
            updated_at: None,
            dependencies: Vec::new(),
            tasks_total: 0,
            tasks_completed: 0,
            checkbox_status: CheckboxStatus::Unchecked,
            plan_phases: Vec::new(),
            tags: Vec::new(),
            branch: None,
            description: None,
        }
    }
}

// ---------------------------------------------------------------------------
// PlanPhase / PlanTask — parsed from plan.md
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PlanPhase {
    pub name: String,
    pub status: PhaseStatus,
    pub tasks: Vec<PlanTask>,
}

impl PlanPhase {
    pub fn tasks_completed(&self) -> usize {
        self.tasks.iter().filter(|t| t.done).count()
    }

    pub fn progress_percent(&self) -> f32 {
        if self.tasks.is_empty() {
            return 0.0;
        }
        (self.tasks_completed() as f32 / self.tasks.len() as f32) * 100.0
    }
}

#[derive(Debug, Clone)]
pub struct PlanTask {
    pub text: String,
    pub done: bool,
}

// ---------------------------------------------------------------------------
// TrackMetadata — intermediate struct from metadata.json
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct TrackMetadata {
    pub status: Status,
    pub priority: Priority,
    pub track_type: TrackType,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub branch: Option<String>,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_progress_zero_when_no_tasks() {
        let track = Track::default();
        assert_eq!(track.progress_percent(), 0.0);
    }

    #[test]
    fn track_progress_50_percent() {
        let track = Track {
            tasks_total: 10,
            tasks_completed: 5,
            ..Track::default()
        };
        assert_eq!(track.progress_percent(), 50.0);
    }

    #[test]
    fn track_progress_100_percent() {
        let track = Track {
            tasks_total: 3,
            tasks_completed: 3,
            ..Track::default()
        };
        assert_eq!(track.progress_percent(), 100.0);
    }

    #[test]
    fn track_is_complete_by_status() {
        let track = Track {
            status: Status::Complete,
            ..Track::default()
        };
        assert!(track.is_complete());
    }

    #[test]
    fn track_is_complete_by_task_count() {
        let track = Track {
            tasks_total: 5,
            tasks_completed: 5,
            ..Track::default()
        };
        assert!(track.is_complete());
    }

    #[test]
    fn track_not_complete() {
        let track = Track {
            tasks_total: 5,
            tasks_completed: 3,
            status: Status::InProgress,
            ..Track::default()
        };
        assert!(!track.is_complete());
    }

    #[test]
    fn status_from_str_loose() {
        assert_eq!(Status::from_str_loose("complete"), Status::Complete);
        assert_eq!(Status::from_str_loose("Completed"), Status::Complete);
        assert_eq!(Status::from_str_loose("in_progress"), Status::InProgress);
        assert_eq!(Status::from_str_loose("active"), Status::InProgress);
        assert_eq!(Status::from_str_loose("blocked"), Status::Blocked);
        assert_eq!(Status::from_str_loose("new"), Status::New);
        assert_eq!(Status::from_str_loose("not_started"), Status::New);
    }

    #[test]
    fn priority_from_str_loose() {
        assert_eq!(Priority::from_str_loose("high"), Priority::High);
        assert_eq!(Priority::from_str_loose("HIGH"), Priority::High);
        assert_eq!(Priority::from_str_loose("low"), Priority::Low);
        assert_eq!(Priority::from_str_loose("unknown"), Priority::Medium);
    }

    #[test]
    fn checkbox_to_status() {
        assert_eq!(CheckboxStatus::Checked.to_status(), Status::Complete);
        assert_eq!(CheckboxStatus::InProgress.to_status(), Status::InProgress);
        assert_eq!(CheckboxStatus::Unchecked.to_status(), Status::New);
    }

    #[test]
    fn filter_mode_cycles() {
        let mode = FilterMode::All;
        assert_eq!(mode.next(), FilterMode::Active);
        assert_eq!(mode.next().next(), FilterMode::Blocked);
        assert_eq!(mode.next().next().next(), FilterMode::Complete);
        assert_eq!(mode.next().next().next().next(), FilterMode::New);
        assert_eq!(mode.next().next().next().next().next(), FilterMode::All);
    }

    #[test]
    fn sort_mode_cycles() {
        let mode = SortMode::Id;
        assert_eq!(mode.next(), SortMode::Progress);
        assert_eq!(mode.next().next(), SortMode::Id);
    }

    #[test]
    fn merge_plan_updates_task_counts() {
        let mut track = Track::default();
        let phases = vec![
            PlanPhase {
                name: "Phase 1".to_string(),
                status: PhaseStatus::Complete,
                tasks: vec![
                    PlanTask { text: "A".to_string(), done: true },
                    PlanTask { text: "B".to_string(), done: true },
                ],
            },
            PlanPhase {
                name: "Phase 2".to_string(),
                status: PhaseStatus::Active,
                tasks: vec![
                    PlanTask { text: "C".to_string(), done: true },
                    PlanTask { text: "D".to_string(), done: false },
                ],
            },
        ];
        track.merge_plan(phases);
        assert_eq!(track.tasks_total, 4);
        assert_eq!(track.tasks_completed, 3);
        assert_eq!(track.phase, "Phase 2");
    }

    #[test]
    fn mark_all_tasks_complete() {
        let mut track = Track {
            tasks_total: 3,
            tasks_completed: 1,
            plan_phases: vec![PlanPhase {
                name: "Phase 1".to_string(),
                status: PhaseStatus::Active,
                tasks: vec![
                    PlanTask { text: "A".to_string(), done: true },
                    PlanTask { text: "B".to_string(), done: false },
                    PlanTask { text: "C".to_string(), done: false },
                ],
            }],
            ..Track::default()
        };
        track.mark_all_tasks_complete();
        assert_eq!(track.tasks_completed, 3);
        assert!(track.plan_phases[0].tasks.iter().all(|t| t.done));
        assert_eq!(track.plan_phases[0].status, PhaseStatus::Complete);
    }
}
