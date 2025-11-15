// Custom horizontal/vertical splitter for egui
// Based on https://gist.github.com/mkalte666/f9a982be0ac0276080d3434ab9ea4655
// Needed because egui's built-in TopBottomPanel::resizable() doesn't work reliably

use std::hash::Hash;
use egui::{CursorIcon, Id, Layout, Pos2, Rect, Rounding, Sense, Ui, Vec2};
use serde::{Deserialize, Serialize};

/// An axis that a Splitter can use
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum SplitterAxis {
    Horizontal,
    Vertical,
}

/// The internal data used by a splitter. Stored into memory
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SplitterData {
    axis: SplitterAxis,
    pos: f32,
    min_size: f32,
}

/// Splits a ui in half, using a draggable separator in the middle.
pub struct Splitter {
    id: Id,
    data: SplitterData,
}

impl Splitter {
    /// Create a new Splitter
    pub fn new(id_source: impl Hash, axis: SplitterAxis) -> Self {
        Self {
            id: Id::new(id_source),
            data: SplitterData {
                axis,
                pos: 0.5,
                min_size: 0.0,
            },
        }
    }

    /// Sets the minimum allowed size for each area
    pub fn min_size(mut self, points: f32) -> Self {
        self.data.min_size = points;
        self
    }

    /// Sets the default position of the splitter separator (0.0 to 1.0)
    /// 0.5 = center, 0.3 = 30% first panel / 70% second panel
    pub fn default_pos(mut self, pos: f32) -> Self {
        self.data.pos = pos.clamp(0.0, 1.0);
        self
    }

    /// Show the splitter and fill it with content.
    /// The callback receives two UIs - one for each side of the split
    pub fn show(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui, &mut Ui)) {
        // Load persisted data (falls back to default if not found)
        let mut data: SplitterData = ui.data_mut(|d| {
            d.get_persisted(self.id)
                .unwrap_or_else(|| self.data.clone())
        });

        let sep_size = 10.0;
        let sep_stroke = 2.0;
        let whole_area = ui.available_size();

        let split_axis_size = match data.axis {
            SplitterAxis::Horizontal => whole_area.x,
            SplitterAxis::Vertical => whole_area.y,
        };

        // Ensure we have enough space for the separator
        // If not, just give all space to first panel and skip separator
        let available_for_split = (split_axis_size - sep_size).max(0.0);

        let split_a_size = (available_for_split * data.pos).max(0.0);
        let split_b_size = (available_for_split - split_a_size).max(0.0);

        let child_size_a = match data.axis {
            SplitterAxis::Horizontal => Vec2::new(split_a_size, whole_area.y.max(0.0)),
            SplitterAxis::Vertical => Vec2::new(whole_area.x.max(0.0), split_a_size),
        };

        let child_size_b = match data.axis {
            SplitterAxis::Horizontal => Vec2::new(split_b_size, whole_area.y.max(0.0)),
            SplitterAxis::Vertical => Vec2::new(whole_area.x.max(0.0), split_b_size),
        };

        let child_rect_a = Rect::from_min_size(ui.next_widget_position(), child_size_a);
        let mut ui_a = ui.child_ui(child_rect_a, Layout::default(), None);

        let sep_rect = match data.axis {
            SplitterAxis::Horizontal => Rect::from_min_size(
                Pos2::new(child_rect_a.max.x, child_rect_a.min.y),
                Vec2::new(sep_size, whole_area.y.max(0.0)),
            ),
            SplitterAxis::Vertical => Rect::from_min_size(
                Pos2::new(child_rect_a.min.x, child_rect_a.max.y),
                Vec2::new(whole_area.x.max(0.0), sep_size),
            ),
        };

        let resp = ui.allocate_rect(sep_rect, Sense::hover().union(Sense::click_and_drag()));

        let sep_draw_rect = match data.axis {
            SplitterAxis::Horizontal => Rect::from_min_size(
                Pos2::new(
                    sep_rect.min.x + sep_size / 2.0 - sep_stroke / 2.0,
                    sep_rect.min.y,
                ),
                Vec2::new(sep_stroke, sep_rect.height()),
            ),
            SplitterAxis::Vertical => Rect::from_min_size(
                Pos2::new(
                    sep_rect.min.x,
                    sep_rect.min.y + sep_size / 2.0 - sep_stroke / 2.0,
                ),
                Vec2::new(sep_rect.width(), sep_stroke),
            ),
        };
        
        ui.painter().rect_filled(
            sep_draw_rect,
            Rounding::ZERO,
            ui.style().visuals.noninteractive().bg_stroke.color,
        );

        let child_rect_b = match data.axis {
            SplitterAxis::Horizontal => {
                Rect::from_min_size(Pos2::new(sep_rect.max.x, sep_rect.min.y), child_size_b)
            }
            SplitterAxis::Vertical => {
                Rect::from_min_size(Pos2::new(sep_rect.min.x, sep_rect.max.y), child_size_b)
            }
        };
        let mut ui_b = ui.child_ui(child_rect_b, Layout::default(), None);

        add_contents(&mut ui_a, &mut ui_b);

        if resp.hovered() {
            match data.axis {
                SplitterAxis::Horizontal => ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn),
                SplitterAxis::Vertical => ui.ctx().set_cursor_icon(CursorIcon::ResizeRow),
            }
        }

        if resp.dragged() {
            let delta_pos = match data.axis {
                SplitterAxis::Horizontal => resp.drag_delta().x / whole_area.x,
                SplitterAxis::Vertical => resp.drag_delta().y / whole_area.y,
            };

            data.pos += delta_pos;
        }

        // Clip pos to respect min_size
        let min_pos = (data.min_size / split_axis_size).min(1.0);
        let max_pos = (1.0 - min_pos).max(0.0);
        data.pos = data.pos.clamp(min_pos, max_pos);

        ui.data_mut(|d| {
            d.insert_persisted(self.id, data);
        });
    }
}

