// Conductor dashboard integration — embedded track progress viewer.
//
// Loads conductor track data from disk and provides a snapshot
// for the iced overlay to render.

pub mod model;
pub mod parser;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use model::*;

/// Snapshot of conductor state for the iced UI layer.
/// All data is owned so UiState can borrow it without lifetime issues.
#[derive(Debug, Clone)]
pub struct ConductorSnapshot {
    pub tracks: Vec<Track>,
    pub selected: usize,
    pub filter: FilterMode,
    pub sort: SortMode,
    pub split_percent: u16,
    pub detail_scroll: usize,
    pub error: Option<String>,
    pub conductor_dir: PathBuf,
    pub search_query: String,
    pub searching: bool,
}

impl ConductorSnapshot {
    pub fn stats(&self) -> (usize, usize, usize, usize, usize) {
        let total = self.tracks.len();
        let active = self.tracks.iter().filter(|t| t.status == Status::InProgress).count();
        let blocked = self.tracks.iter().filter(|t| t.status == Status::Blocked).count();
        let complete = self.tracks.iter().filter(|t| t.status == Status::Complete).count();
        let new = self.tracks.iter().filter(|t| t.status == Status::New).count();
        (total, active, blocked, complete, new)
    }

    pub fn selected_track(&self) -> Option<&Track> {
        self.tracks.get(self.selected)
    }
}

/// Long-lived state for the conductor dashboard overlay.
pub struct ConductorState {
    all_tracks: BTreeMap<TrackId, Track>,
    filtered_ids: Vec<TrackId>,
    selected: usize,
    filter: FilterMode,
    sort: SortMode,
    split_percent: u16,
    detail_scroll: usize,
    conductor_dir: PathBuf,
    search_query: String,
    searching: bool,
    error: Option<String>,
}

impl ConductorState {
    /// Create by loading tracks from the given conductor directory.
    pub fn load(conductor_dir: &Path) -> Self {
        let (all_tracks, error) = match parser::load_all_tracks(conductor_dir) {
            Ok(tracks) => (tracks, None),
            Err(e) => (BTreeMap::new(), Some(e.to_string())),
        };

        let mut state = Self {
            all_tracks,
            filtered_ids: Vec::new(),
            selected: 0,
            filter: FilterMode::default(),
            sort: SortMode::default(),
            split_percent: 40,
            detail_scroll: 0,
            conductor_dir: conductor_dir.to_path_buf(),
            search_query: String::new(),
            searching: false,
            error,
        };
        state.apply_filter_sort();
        state
    }

    /// Path to the conductor directory this state was loaded from.
    pub fn conductor_dir(&self) -> &Path {
        &self.conductor_dir
    }

    /// Reload tracks from disk.
    pub fn reload(&mut self) {
        match parser::load_all_tracks(&self.conductor_dir) {
            Ok(tracks) => {
                self.all_tracks = tracks;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
        self.apply_filter_sort();
    }

    /// Build a snapshot for the UI layer.
    pub fn snapshot(&self) -> ConductorSnapshot {
        let tracks: Vec<Track> = self
            .filtered_ids
            .iter()
            .filter_map(|id| self.all_tracks.get(id).cloned())
            .collect();

        ConductorSnapshot {
            tracks,
            selected: self.selected,
            filter: self.filter,
            sort: self.sort,
            split_percent: self.split_percent,
            detail_scroll: self.detail_scroll,
            error: self.error.clone(),
            conductor_dir: self.conductor_dir.clone(),
            search_query: self.search_query.clone(),
            searching: self.searching,
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered_ids.is_empty() && self.selected < self.filtered_ids.len() - 1 {
            self.selected += 1;
            self.detail_scroll = 0;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    pub fn select_index(&mut self, idx: usize) {
        if idx < self.filtered_ids.len() {
            self.selected = idx;
            self.detail_scroll = 0;
        }
    }

    pub fn cycle_filter(&mut self) {
        self.filter = self.filter.next();
        self.apply_filter_sort();
    }

    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.apply_filter_sort();
    }

    pub fn scroll_detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(3);
    }

    pub fn scroll_detail_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(3);
    }

    pub fn widen_left(&mut self) {
        if self.split_percent < 70 {
            self.split_percent += 5;
        }
    }

    pub fn widen_right(&mut self) {
        if self.split_percent > 20 {
            self.split_percent -= 5;
        }
    }

    pub fn start_search(&mut self) {
        self.searching = true;
    }

    pub fn stop_search(&mut self) {
        self.searching = false;
    }

    pub fn is_searching(&self) -> bool {
        self.searching
    }

    pub fn search_char(&mut self, ch: char) {
        self.search_query.push(ch);
        self.apply_filter_sort();
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        self.apply_filter_sort();
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.apply_filter_sort();
    }

    fn apply_filter_sort(&mut self) {
        let mut ids: Vec<TrackId> = self
            .all_tracks
            .iter()
            .filter(|(_, track)| match self.filter {
                FilterMode::All => true,
                FilterMode::Active => track.status == Status::InProgress,
                FilterMode::Blocked => track.status == Status::Blocked,
                FilterMode::Complete => track.status == Status::Complete,
                FilterMode::New => track.status == Status::New,
            })
            .filter(|(_, track)| {
                if self.search_query.is_empty() {
                    return true;
                }
                let q = self.search_query.to_lowercase();
                track.title.to_lowercase().contains(&q)
                    || track.id.as_str().to_lowercase().contains(&q)
            })
            .map(|(id, _)| id.clone())
            .collect();

        match self.sort {
            SortMode::Id => {
                // BTreeMap already sorted by key
            }
            SortMode::Progress => {
                ids.sort_by(|a, b| {
                    let ta = self.all_tracks.get(a).unwrap();
                    let tb = self.all_tracks.get(b).unwrap();
                    tb.progress_percent()
                        .partial_cmp(&ta.progress_percent())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        self.filtered_ids = ids;
        if self.selected >= self.filtered_ids.len() {
            self.selected = self.filtered_ids.len().saturating_sub(1);
        }
    }
}

/// Discover a conductor directory by walking up from `start_dir`.
/// Looks for a directory containing `tracks.md` (either `conductor/tracks.md`
/// or the start dir itself if it contains `tracks.md`).
pub fn discover_conductor_dir(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = start_dir.to_path_buf();
    loop {
        let candidate = dir.join("conductor").join("tracks.md");
        if candidate.exists() {
            return Some(dir.join("conductor"));
        }
        // Also check if `dir` itself is a conductor dir
        let direct = dir.join("tracks.md");
        if direct.exists() && dir.file_name().map_or(false, |n| n == "conductor") {
            return Some(dir.clone());
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_veloterm_conductor() {
        let project_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let conductor = discover_conductor_dir(project_dir);
        assert!(conductor.is_some(), "Should find conductor dir in project root");
        assert!(conductor.unwrap().join("tracks.md").exists());
    }

    #[test]
    fn discover_from_subdirectory() {
        let project_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let sub = project_dir.join("src").join("conductor");
        let conductor = discover_conductor_dir(&sub);
        assert!(conductor.is_some(), "Should find conductor dir from subdirectory");
    }

    #[test]
    fn conductor_state_load_and_snapshot() {
        let conductor_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("conductor");
        if !conductor_dir.exists() {
            return;
        }
        let state = ConductorState::load(&conductor_dir);
        assert!(state.error.is_none());

        let snapshot = state.snapshot();
        assert!(snapshot.tracks.len() >= 25);
        assert_eq!(snapshot.selected, 0);
        assert_eq!(snapshot.filter, FilterMode::All);
    }

    #[test]
    fn conductor_state_filter() {
        let conductor_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("conductor");
        if !conductor_dir.exists() {
            return;
        }
        let mut state = ConductorState::load(&conductor_dir);

        // All VeloTerm tracks are complete, so filtering for Active should give 0
        state.filter = FilterMode::Active;
        state.apply_filter_sort();
        let snapshot = state.snapshot();
        assert_eq!(snapshot.tracks.len(), 0);

        // Filtering for Complete should give all
        state.filter = FilterMode::Complete;
        state.apply_filter_sort();
        let snapshot = state.snapshot();
        assert!(snapshot.tracks.len() >= 25);
    }

    #[test]
    fn conductor_state_navigation() {
        let conductor_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("conductor");
        if !conductor_dir.exists() {
            return;
        }
        let mut state = ConductorState::load(&conductor_dir);
        assert_eq!(state.selected, 0);

        state.select_next();
        assert_eq!(state.selected, 1);

        state.select_prev();
        assert_eq!(state.selected, 0);

        // Can't go below 0
        state.select_prev();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn conductor_state_cycle_filter() {
        let conductor_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("conductor");
        if !conductor_dir.exists() {
            return;
        }
        let mut state = ConductorState::load(&conductor_dir);
        assert_eq!(state.filter, FilterMode::All);
        state.cycle_filter();
        assert_eq!(state.filter, FilterMode::Active);
        state.cycle_filter();
        assert_eq!(state.filter, FilterMode::Blocked);
    }

    #[test]
    fn conductor_state_split_resize() {
        let conductor_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("conductor");
        if !conductor_dir.exists() {
            return;
        }
        let mut state = ConductorState::load(&conductor_dir);
        assert_eq!(state.split_percent, 40);
        state.widen_left();
        assert_eq!(state.split_percent, 45);
        state.widen_right();
        assert_eq!(state.split_percent, 40);
    }

    #[test]
    fn conductor_snapshot_stats() {
        let conductor_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("conductor");
        if !conductor_dir.exists() {
            return;
        }
        let state = ConductorState::load(&conductor_dir);
        let snapshot = state.snapshot();
        let (total, active, blocked, complete, new) = snapshot.stats();
        assert!(total >= 25);
        assert_eq!(complete, total); // All tracks complete
        assert_eq!(active, 0);
        assert_eq!(blocked, 0);
        assert_eq!(new, 0);
    }
}
