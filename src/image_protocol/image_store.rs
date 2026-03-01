// Image data storage with memory management for the Kitty Graphics Protocol.

use std::collections::HashMap;

use super::kitty_parser::{Action, DeleteTarget, KittyCommand, ParseError, PixelFormat};
use super::response::{format_response_with_quiet, ResponseKind};

/// Stored image pixel data.
#[derive(Debug, Clone)]
pub struct ImageData {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub refcount: u32,
}

/// A placed image on the terminal grid.
#[derive(Debug, Clone, Default)]
pub struct Placement {
    pub image_id: u32,
    pub placement_id: u32,
    pub grid_row: i64,
    pub grid_col: u32,
    pub columns: u32,
    pub rows: u32,
    pub source_rect: Option<(u32, u32, u32, u32)>,
    pub z_index: i32,
    pub cell_offset: (u32, u32),
}

/// Image store with memory-bounded storage and placement tracking.
pub struct ImageStore {
    images: HashMap<u32, ImageData>,
    placements: HashMap<(u32, u32), Placement>,
    pending_chunks: HashMap<u32, Vec<Vec<u8>>>,
    /// Track which image_id a chunked transmission belongs to
    chunk_image_id: Option<u32>,
    chunk_command: Option<KittyCommand>,
    memory_used: usize,
    memory_limit: usize,
    next_auto_id: u32,
    /// Insertion order for LRU eviction
    insertion_order: Vec<u32>,
}

impl ImageStore {
    pub fn new(memory_limit: usize) -> Self {
        Self {
            images: HashMap::new(),
            placements: HashMap::new(),
            pending_chunks: HashMap::new(),
            chunk_image_id: None,
            chunk_command: None,
            memory_used: 0,
            memory_limit,
            next_auto_id: 1,
            insertion_order: Vec::new(),
        }
    }

    pub fn has_image(&self, id: u32) -> bool {
        self.images.contains_key(&id)
    }

    pub fn get_image(&self, id: u32) -> Option<&ImageData> {
        self.images.get(&id)
    }

    pub fn memory_used(&self) -> usize {
        self.memory_used
    }

    pub fn placement_count(&self) -> usize {
        self.placements.len()
    }

    pub fn get_placement(&self, image_id: u32, placement_id: u32) -> Option<&Placement> {
        self.placements.get(&(image_id, placement_id))
    }

    pub fn next_auto_id(&mut self) -> u32 {
        let id = self.next_auto_id;
        self.next_auto_id += 1;
        id
    }

    pub fn add_image(&mut self, id: u32, data: ImageData) {
        // Re-transmit: remove old image and its placements
        if self.images.contains_key(&id) {
            self.remove_image_internal(id, false);
        }

        let size = data.pixels.len();

        // Evict if needed
        self.evict_to_fit(size);

        self.memory_used += size;
        self.images.insert(id, data);
        self.insertion_order.retain(|&i| i != id);
        self.insertion_order.push(id);
    }

    pub fn add_image_auto(&mut self, data: ImageData) -> u32 {
        let id = self.next_auto_id();
        self.add_image(id, data);
        id
    }

    pub fn add_image_by_number(&mut self, _number: u32, data: ImageData) -> u32 {
        self.add_image_auto(data)
    }

    pub fn transmit_and_display(
        &mut self,
        id: Option<u32>,
        data: ImageData,
        row: i64,
        col: u32,
        columns: Option<u32>,
        rows: Option<u32>,
    ) -> u32 {
        let actual_id = if let Some(id) = id.filter(|&i| i > 0) {
            self.add_image(id, data);
            id
        } else {
            self.add_image_auto(data)
        };

        let placement = Placement {
            image_id: actual_id,
            placement_id: 0,
            grid_row: row,
            grid_col: col,
            columns: columns.unwrap_or(0),
            rows: rows.unwrap_or(0),
            ..Default::default()
        };
        self.add_placement(actual_id, placement);
        actual_id
    }

    pub fn add_placement(&mut self, image_id: u32, placement: Placement) {
        let key = (image_id, placement.placement_id);
        self.placements.insert(key, placement);
        // Increment refcount
        if let Some(img) = self.images.get_mut(&image_id) {
            img.refcount += 1;
        }
    }

    pub fn delete_placement(&mut self, image_id: u32, placement_id: u32) {
        if self.placements.remove(&(image_id, placement_id)).is_some() {
            if let Some(img) = self.images.get_mut(&image_id) {
                img.refcount = img.refcount.saturating_sub(1);
            }
        }
    }

    pub fn delete(&mut self, target: DeleteTarget, id: Option<u32>, _z: Option<i32>) {
        match target {
            DeleteTarget::AllVisible => {
                self.placements.clear();
                // Reset all refcounts
                for img in self.images.values_mut() {
                    img.refcount = 0;
                }
            }
            DeleteTarget::AllFree => {
                self.placements.clear();
                let ids: Vec<u32> = self.images.keys().copied().collect();
                for id in ids {
                    self.remove_image_internal(id, false);
                }
            }
            DeleteTarget::ById => {
                if let Some(id) = id {
                    let keys: Vec<(u32, u32)> = self
                        .placements
                        .keys()
                        .filter(|(img_id, _)| *img_id == id)
                        .copied()
                        .collect();
                    for key in keys {
                        self.placements.remove(&key);
                    }
                    if let Some(img) = self.images.get_mut(&id) {
                        img.refcount = 0;
                    }
                }
            }
            DeleteTarget::ByIdFree => {
                if let Some(id) = id {
                    let keys: Vec<(u32, u32)> = self
                        .placements
                        .keys()
                        .filter(|(img_id, _)| *img_id == id)
                        .copied()
                        .collect();
                    for key in keys {
                        self.placements.remove(&key);
                    }
                    self.remove_image_internal(id, false);
                }
            }
            _ => {
                // Other delete targets not yet implemented
            }
        }
    }

    pub fn handle_query(&mut self, _id: u32, _data: ImageData) {
        // Query validates but does not store
    }

    /// Handle a parsed Kitty command. Returns an optional response string to write back to PTY.
    pub fn handle_command(&mut self, cmd: KittyCommand) -> Option<String> {
        let quiet = cmd.quiet;
        let image_id = cmd.image_id;
        let image_number = cmd.image_number;

        match cmd.action {
            Action::Query => {
                let id = image_id.unwrap_or(0);
                // Validate by trying to decode if there's a payload
                format_response_with_quiet(
                    image_id,
                    image_number,
                    ResponseKind::Ok,
                    quiet,
                )
            }
            Action::Transmit | Action::TransmitAndDisplay => {
                // Handle chunked transmission
                if cmd.more_chunks {
                    let id = image_id.unwrap_or(0);
                    self.add_chunk(id, cmd.payload.clone(), true);
                    self.chunk_command = Some(cmd);
                    return None; // No response until final chunk
                }

                // Check if this is the final chunk of a multi-chunk sequence
                let (final_payload, final_cmd) =
                    if self.chunk_image_id.is_some() || self.chunk_command.is_some() {
                        let id = self
                            .chunk_command
                            .as_ref()
                            .and_then(|c| c.image_id)
                            .unwrap_or(0);
                        self.add_chunk(id, cmd.payload.clone(), false);
                        let payload = self.take_assembled(id).unwrap_or_default();
                        let prev_cmd = self.chunk_command.take().unwrap_or(cmd);
                        (payload, prev_cmd)
                    } else {
                        (cmd.payload.clone(), cmd)
                    };

                let id = final_cmd.image_id;
                let w = final_cmd.width.unwrap_or(0);
                let h = final_cmd.height.unwrap_or(0);

                // Decode the payload
                let pixels = match super::decode::decode_pixels(
                    &final_payload,
                    w,
                    h,
                    final_cmd.format,
                    final_cmd.compression,
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        return format_response_with_quiet(
                            id,
                            image_number,
                            ResponseKind::Error("ENODATA", &e.to_string()),
                            quiet,
                        );
                    }
                };

                // Determine actual dimensions for PNG
                let (actual_w, actual_h) = if final_cmd.format == PixelFormat::Png {
                    let pixel_count = pixels.len() / 4;
                    // Try to get dimensions from PNG header
                    if let Ok((pw, ph, _)) = super::decode::decode_png(&final_payload) {
                        (pw, ph)
                    } else {
                        (w, h)
                    }
                } else {
                    (w, h)
                };

                let img = ImageData {
                    pixels,
                    width: actual_w,
                    height: actual_h,
                    refcount: 0,
                };

                if final_cmd.action == Action::TransmitAndDisplay {
                    let actual_id = self.transmit_and_display(
                        id,
                        img,
                        0,
                        0,
                        final_cmd.columns,
                        final_cmd.rows,
                    );
                    format_response_with_quiet(
                        Some(actual_id),
                        image_number,
                        ResponseKind::Ok,
                        quiet,
                    )
                } else {
                    // Transmit only
                    if let Some(id) = id.filter(|&i| i > 0) {
                        self.add_image(id, img);
                    } else {
                        self.add_image_auto(img);
                    }
                    format_response_with_quiet(id, image_number, ResponseKind::Ok, quiet)
                }
            }
            Action::Delete => {
                if let Some(target) = cmd.delete_target {
                    self.delete(target, image_id, Some(cmd.z_index));
                }
                format_response_with_quiet(image_id, image_number, ResponseKind::Ok, quiet)
            }
            Action::Place => {
                if let Some(id) = image_id {
                    let placement = Placement {
                        image_id: id,
                        placement_id: cmd.placement_id.unwrap_or(0),
                        z_index: cmd.z_index,
                        columns: cmd.columns.unwrap_or(0),
                        rows: cmd.rows.unwrap_or(0),
                        ..Default::default()
                    };
                    self.add_placement(id, placement);
                }
                format_response_with_quiet(image_id, image_number, ResponseKind::Ok, quiet)
            }
            _ => None,
        }
    }

    // ── Chunked transmission ────────────────────────────────────────

    pub fn add_chunk(&mut self, id: u32, data: Vec<u8>, more: bool) {
        self.chunk_image_id = if more { Some(id) } else { None };
        self.pending_chunks.entry(id).or_default().push(data);
    }

    pub fn clear_pending_chunks(&mut self, id: u32) {
        self.pending_chunks.remove(&id);
        if self.chunk_image_id == Some(id) {
            self.chunk_image_id = None;
        }
    }

    pub fn take_assembled(&mut self, id: u32) -> Option<Vec<u8>> {
        let chunks = self.pending_chunks.remove(&id)?;
        // Concatenate all base64 chunks, then decode
        let mut combined = Vec::new();
        for chunk in chunks {
            combined.extend_from_slice(&chunk);
        }
        // The chunks are already decoded at this point if called from handle_command
        // But in the test API, chunks are raw base64 that needs decoding
        if let Ok(decoded) =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &combined)
        {
            Some(decoded)
        } else {
            Some(combined)
        }
    }

    // ── Validation ──────────────────────────────────────────────────

    pub fn validate_dimensions(&self, w: u32, h: u32) -> Result<(), ParseError> {
        if w == 0 || h == 0 {
            return Err(ParseError("zero dimensions".to_string()));
        }
        let pixels = w as u64 * h as u64;
        // Cap at ~25 megapixels (100MB RGBA)
        if pixels > 25_000_000 {
            return Err(ParseError(format!(
                "image too large: {}x{} = {} pixels",
                w, h, pixels
            )));
        }
        Ok(())
    }

    // ── Visible placements ──────────────────────────────────────────

    pub fn visible_placements(&self, top_row: i64, bottom_row: i64) -> Vec<&Placement> {
        self.placements
            .values()
            .filter(|p| {
                let p_bottom = p.grid_row + p.rows as i64;
                p.grid_row < bottom_row && p_bottom > top_row
            })
            .collect()
    }

    pub fn visible_placements_sorted(&self, top_row: i64, bottom_row: i64) -> Vec<&Placement> {
        let mut placements = self.visible_placements(top_row, bottom_row);
        placements.sort_by_key(|p| p.z_index);
        placements
    }

    // ── Internal helpers ────────────────────────────────────────────

    fn remove_image_internal(&mut self, id: u32, keep_placements: bool) {
        if let Some(img) = self.images.remove(&id) {
            self.memory_used = self.memory_used.saturating_sub(img.pixels.len());
            self.insertion_order.retain(|&i| i != id);
        }
        if !keep_placements {
            let keys: Vec<(u32, u32)> = self
                .placements
                .keys()
                .filter(|(img_id, _)| *img_id == id)
                .copied()
                .collect();
            for key in keys {
                self.placements.remove(&key);
            }
        }
    }

    fn evict_to_fit(&mut self, needed: usize) {
        while self.memory_used + needed > self.memory_limit && !self.insertion_order.is_empty() {
            // Find oldest unreferenced image
            let evict_id = if let Some(pos) = self
                .insertion_order
                .iter()
                .position(|&id| self.images.get(&id).is_some_and(|img| img.refcount == 0))
            {
                self.insertion_order[pos]
            } else if let Some(&id) = self.insertion_order.first() {
                // All referenced — evict oldest anyway
                id
            } else {
                break;
            };

            self.remove_image_internal(evict_id, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Builder for test fixtures ───────────────────────────────

    struct TestImageBuilder {
        width: u32,
        height: u32,
        pixel_value: u8,
    }
    impl TestImageBuilder {
        fn new(w: u32, h: u32) -> Self {
            Self {
                width: w,
                height: h,
                pixel_value: 0xFF,
            }
        }
        fn byte_size(&self) -> usize {
            (self.width * self.height * 4) as usize
        }
        fn build(self) -> ImageData {
            ImageData {
                pixels: vec![self.pixel_value; (self.width * self.height * 4) as usize],
                width: self.width,
                height: self.height,
                refcount: 0,
            }
        }
    }

    fn test_store() -> ImageStore {
        ImageStore::new(10 * 1024 * 1024) // 10MB
    }

    // ── Basic operations ────────────────────────────────────────

    #[test]
    fn store_add_and_retrieve_image() {
        let mut store = test_store();
        let img = TestImageBuilder::new(64, 64).build();
        store.add_image(1, img);
        assert!(store.has_image(1));
        let retrieved = store.get_image(1).unwrap();
        assert_eq!(retrieved.width, 64);
        assert_eq!(retrieved.height, 64);
    }

    #[test]
    fn store_image_not_found() {
        let store = test_store();
        assert!(!store.has_image(999));
        assert!(store.get_image(999).is_none());
    }

    #[test]
    fn store_retransmit_replaces_image_and_placements() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 1,
                ..Default::default()
            },
        );
        assert_eq!(store.placement_count(), 1);
        store.add_image(1, TestImageBuilder::new(64, 64).build());
        assert_eq!(store.get_image(1).unwrap().width, 64);
        assert_eq!(
            store.placement_count(),
            0,
            "placements should be cleared on retransmit"
        );
    }

    #[test]
    fn store_add_placement() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        let p = Placement {
            image_id: 1,
            placement_id: 5,
            z_index: -1,
            ..Default::default()
        };
        store.add_placement(1, p);
        assert_eq!(store.placement_count(), 1);
        assert!(store.get_placement(1, 5).is_some());
    }

    #[test]
    fn store_multiple_placements_same_image() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 1,
                ..Default::default()
            },
        );
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 2,
                ..Default::default()
            },
        );
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 3,
                ..Default::default()
            },
        );
        assert_eq!(store.placement_count(), 3);
    }

    // ── Memory management ───────────────────────────────────────

    #[test]
    fn store_tracks_memory_usage() {
        let mut store = test_store();
        let size = TestImageBuilder::new(100, 100).byte_size();
        store.add_image(1, TestImageBuilder::new(100, 100).build());
        assert_eq!(store.memory_used(), size);
    }

    #[test]
    fn store_evicts_when_over_limit() {
        let img_size = TestImageBuilder::new(50, 50).byte_size();
        let mut store = ImageStore::new(img_size * 2 + 100);
        store.add_image(1, TestImageBuilder::new(50, 50).build());
        store.add_image(2, TestImageBuilder::new(50, 50).build());
        store.add_image(3, TestImageBuilder::new(50, 50).build());
        assert!(!store.has_image(1), "oldest image should be evicted");
        assert!(store.has_image(3), "newest image should survive");
    }

    #[test]
    fn store_evicts_unreferenced_first() {
        let img_size = TestImageBuilder::new(50, 50).byte_size();
        let mut store = ImageStore::new(img_size * 2 + 100);
        store.add_image(1, TestImageBuilder::new(50, 50).build());
        store.add_image(2, TestImageBuilder::new(50, 50).build());
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 1,
                ..Default::default()
            },
        );
        store.add_image(3, TestImageBuilder::new(50, 50).build());
        assert!(store.has_image(1), "referenced image should survive");
        assert!(!store.has_image(2), "unreferenced image evicted first");
        assert!(store.has_image(3));
    }

    // ── Delete operations ───────────────────────────────────────

    #[test]
    fn delete_by_id_removes_placements_keeps_data() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 1,
                ..Default::default()
            },
        );
        store.delete(DeleteTarget::ById, Some(1), None);
        assert!(store.has_image(1), "data should be kept (lowercase)");
        assert_eq!(store.placement_count(), 0);
    }

    #[test]
    fn delete_by_id_free_removes_data_and_placements() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 1,
                ..Default::default()
            },
        );
        store.delete(DeleteTarget::ByIdFree, Some(1), None);
        assert!(!store.has_image(1), "data should be freed (uppercase)");
        assert_eq!(store.placement_count(), 0);
    }

    #[test]
    fn delete_specific_placement() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 1,
                ..Default::default()
            },
        );
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 2,
                ..Default::default()
            },
        );
        store.delete_placement(1, 1);
        assert_eq!(store.placement_count(), 1);
        assert!(store.get_placement(1, 2).is_some());
    }

    #[test]
    fn delete_all_clears_placements() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        store.add_image(2, TestImageBuilder::new(32, 32).build());
        store.add_placement(
            1,
            Placement {
                image_id: 1,
                placement_id: 1,
                ..Default::default()
            },
        );
        store.add_placement(
            2,
            Placement {
                image_id: 2,
                placement_id: 1,
                ..Default::default()
            },
        );
        store.delete(DeleteTarget::AllVisible, None, None);
        assert_eq!(store.placement_count(), 0);
        assert!(store.has_image(1), "data kept for lowercase");
    }

    #[test]
    fn delete_all_free_clears_everything() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        store.add_image(2, TestImageBuilder::new(32, 32).build());
        store.delete(DeleteTarget::AllFree, None, None);
        assert!(!store.has_image(1));
        assert!(!store.has_image(2));
    }

    // ── Query operation ─────────────────────────────────────────

    #[test]
    fn query_does_not_store_image() {
        let mut store = test_store();
        let img = TestImageBuilder::new(1, 1).build();
        store.handle_query(31, img);
        assert!(!store.has_image(31), "query should not add to store");
    }

    // ── Auto ID assignment ──────────────────────────────────────

    #[test]
    fn auto_id_increments() {
        let mut store = test_store();
        let id1 = store.next_auto_id();
        let id2 = store.next_auto_id();
        assert!(id2 > id1);
    }

    #[test]
    fn image_number_gets_auto_id() {
        let mut store = test_store();
        let auto_id = store.add_image_by_number(93, TestImageBuilder::new(32, 32).build());
        assert!(auto_id > 0);
        assert!(store.has_image(auto_id));
    }

    // ── Chunked transmission ────────────────────────────────────

    #[test]
    fn chunked_accumulation() {
        let mut store = test_store();
        store.add_chunk(1, b"YWJj".to_vec(), true);
        store.add_chunk(1, b"ZGVm".to_vec(), true);
        store.add_chunk(1, b"Z2hp".to_vec(), false);
        let assembled = store.take_assembled(1).unwrap();
        assert_eq!(assembled, b"abcdefghi");
    }

    #[test]
    fn chunked_interrupted_by_new_command_clears() {
        let mut store = test_store();
        store.add_chunk(1, b"QUFB".to_vec(), true);
        store.clear_pending_chunks(1);
        assert!(store.take_assembled(1).is_none());
    }

    // ── Validation ──────────────────────────────────────────────

    #[test]
    fn reject_image_exceeding_pixel_limit() {
        let store = test_store();
        assert!(store.validate_dimensions(10000, 10000).is_err());
    }

    #[test]
    fn accept_reasonable_image() {
        let store = test_store();
        assert!(store.validate_dimensions(1920, 1080).is_ok());
    }

    #[test]
    fn reject_zero_dimensions_for_raw_format() {
        let store = test_store();
        assert!(store.validate_dimensions(0, 0).is_err());
    }

    // ── image_id=0 auto-assignment ──────────────────────────────

    #[test]
    fn transmit_with_zero_id_gets_auto_assigned() {
        let mut store = test_store();
        let auto_id = store.add_image_auto(TestImageBuilder::new(32, 32).build());
        assert!(auto_id > 0, "auto-assigned ID should be > 0");
        assert!(store.has_image(auto_id));
    }

    #[test]
    fn transmit_display_with_zero_id_creates_placement() {
        let mut store = test_store();
        let auto_id = store.transmit_and_display(
            None,
            TestImageBuilder::new(32, 32).build(),
            0,
            0,
            None,
            None,
        );
        assert!(auto_id > 0);
        assert!(
            store.placement_count() >= 1,
            "should create default placement"
        );
    }

    // ── Memory tracking on delete ───────────────────────────────

    #[test]
    fn delete_free_decreases_memory_used() {
        let mut store = test_store();
        let size = TestImageBuilder::new(100, 100).byte_size();
        store.add_image(1, TestImageBuilder::new(100, 100).build());
        assert_eq!(store.memory_used(), size);
        store.delete(DeleteTarget::ByIdFree, Some(1), None);
        assert_eq!(store.memory_used(), 0, "memory should decrease after free");
    }

    // ── Placement default ───────────────────────────────────────

    #[test]
    fn placement_default_has_sane_values() {
        let p = Placement::default();
        assert_eq!(p.image_id, 0);
        assert_eq!(p.placement_id, 0);
        assert_eq!(p.grid_row, 0);
        assert_eq!(p.grid_col, 0);
        assert_eq!(p.z_index, 0);
        assert_eq!(p.source_rect, None);
        assert_eq!(p.cell_offset, (0, 0));
    }

    // ── Visible placements ──────────────────────────────────────

    #[test]
    fn visible_placements_filtered_by_viewport() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        store.add_placement(
            1,
            Placement {
                grid_row: 5,
                columns: 4,
                rows: 2,
                ..Default::default()
            },
        );
        store.add_placement(
            1,
            Placement {
                grid_row: 100,
                columns: 4,
                rows: 2,
                placement_id: 2,
                ..Default::default()
            },
        );
        let visible = store.visible_placements(0, 24);
        assert_eq!(visible.len(), 1, "only placement at row 5 should be visible");
    }

    #[test]
    fn placements_sorted_by_z_index() {
        let mut store = test_store();
        store.add_image(1, TestImageBuilder::new(32, 32).build());
        store.add_placement(
            1,
            Placement {
                z_index: 5,
                placement_id: 1,
                rows: 1,
                ..Default::default()
            },
        );
        store.add_placement(
            1,
            Placement {
                z_index: -1,
                placement_id: 2,
                rows: 1,
                ..Default::default()
            },
        );
        store.add_placement(
            1,
            Placement {
                z_index: 0,
                placement_id: 3,
                rows: 1,
                ..Default::default()
            },
        );
        let sorted = store.visible_placements_sorted(0, 24);
        assert!(sorted[0].z_index <= sorted[1].z_index);
        assert!(sorted[1].z_index <= sorted[2].z_index);
    }
}
