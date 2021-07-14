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
pub trait Playhead: Info + Interaction {}

impl<T> Playhead for T where T: Info + Interaction {}

/// Set the playhead widget - a thin line for indicating progress through the timeline.
pub fn set(ui: &mut egui::Ui, timeline_rect: egui::Rect, api: &mut dyn Playhead) -> egui::Response {
    // Allocate a thin `Rect` over the timeline at the playhead.
    let playhead_ticks = api.playhead_ticks();
    let playhead_x = timeline_rect.left() + playhead_ticks / api.ticks_per_point();
    let playhead_w = 1.0;
    let half_w = playhead_w * 0.5;
    let min = egui::Pos2::new(playhead_x - half_w, timeline_rect.top());
    let max = egui::Pos2::new(playhead_x + half_w, timeline_rect.bottom());
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
    if timeline_rect.x_range().contains(&playhead_x) {
        let visuals = ui.style().interact(&response);
        let radius = 0.0;
        let stroke = egui::Stroke { width: 0.5, ..visuals.fg_stroke };
        ui.painter().rect(rect, radius, visuals.bg_fill, stroke);
    }

    response
}
