use super::ruler::MusicalInfo;

/// For retrieving information about the playhead.
pub trait Info: MusicalInfo {
    /// The location of the playhead in ticks relative to the start of the timeline.
    fn playhead_ticks(&self) -> f32;
}

/// For handling interaction with the playhead.
pub trait Interaction {
    /// Set the location of the playhead in ticks.
    fn set_playhead_ticks(&mut self, ticks: f32);
}

/// For both providing info and handling interaction.
pub trait PlayheadApi: Info + Interaction {}

/// Playhead configuration for a timeline widget.
pub struct Playhead {
    extend_beyond_last_track: f32,
    extend_to_available_height: bool,
    width: f32,
}

impl Playhead {
    pub const DEFAULT_EXTEND_BEYOND_LAST_TRACK: f32 = 0.0;
    pub const DEFAULT_EXTEND_TO_AVAILABLE_HEIGHT: bool = false;
    pub const DEFAULT_WIDTH: f32 = 1.0;

    /// Create a new default playhead.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether or not to extend the playhead to the total available height.
    ///
    /// This is useful if the timeline occupies a main `CentralPanel` and you
    /// want the playhead to extend across the entire available track space,
    /// rather than just the occupied track space.
    ///
    /// Default: `false`
    pub fn extend_to_available_height(mut self, b: bool) -> Self {
        self.extend_to_available_height = b;
        self
    }

    /// Extend the playhead beyond the last track by the given amount.
    ///
    /// Only applies if `extend_to_available_height` is `false`.
    ///
    /// Default: `0.0`
    pub fn extend_beyond_last_track(mut self, f: f32) -> Self {
        self.extend_beyond_last_track = f;
        self
    }

    /// Specify the width of the playhead rect.
    ///
    /// Default: `1.0`
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
}

impl Default for Playhead {
    fn default() -> Self {
        Self {
            extend_beyond_last_track: Self::DEFAULT_EXTEND_BEYOND_LAST_TRACK,
            extend_to_available_height: Self::DEFAULT_EXTEND_TO_AVAILABLE_HEIGHT,
            width: Self::DEFAULT_WIDTH,
        }
    }
}

impl<T> PlayheadApi for T where T: Info + Interaction {}

/// Set the playhead widget - a thin line for indicating progress through the timeline.
pub fn set(
    ui: &mut egui::Ui,
    api: &mut dyn PlayheadApi,
    timeline_rect: egui::Rect,
    tracks_bottom: f32,
    playhead: Playhead,
) -> egui::Response {
    // Allocate a thin `Rect` over the timeline at the playhead.
    let playhead_ticks = api.playhead_ticks();
    let playhead_x = timeline_rect.left() + playhead_ticks / api.ticks_per_point();
    let half_w = playhead.width * 0.5;
    let top = timeline_rect.top();
    let bottom = if playhead.extend_to_available_height {
        timeline_rect.bottom()
    } else {
        tracks_bottom + playhead.extend_beyond_last_track
    };
    let min = egui::Pos2::new(playhead_x - half_w, top);
    let max = egui::Pos2::new(playhead_x + half_w, bottom);
    let rect = egui::Rect::from_min_max(min, max);
    let mut response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

    let timeline_w = timeline_rect.width();
    let ticks_per_point = api.ticks_per_point();
    let visible_ticks = ticks_per_point * timeline_w;

    // Handle interactions.
    if response.clicked() || response.dragged() {
        if let Some(pt) = response.interact_pointer_pos() {
            let tick = (((pt.x - timeline_rect.min.x) / timeline_w) * visible_ticks).max(0.0);
            api.set_playhead_ticks(tick);
            response.mark_changed();
        }
    }

    // Draw a thin rect.
    if timeline_rect.x_range().contains(playhead_x) {
        let visuals = ui.style().interact(&response);
        let radius = 0.0;
        let stroke = egui::Stroke {
            width: 0.5,
            ..visuals.fg_stroke
        };
        ui.painter().rect(rect, radius, visuals.bg_fill, stroke);
    }

    response
}
