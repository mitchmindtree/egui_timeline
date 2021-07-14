use egui::plot;
use std::{
    hash::Hash,
    ops::{Range, RangeInclusive},
};

pub use playhead::Playhead;
pub use ruler::MusicalRuler;

pub mod playhead;
pub mod ruler;

pub const MIN_STEP_GAP: f32 = 4.0;

/// The implementation required to instantiate a timeline widget.
pub trait Timeline {
    /// Access to the ruler info.
    fn musical_ruler_info(&self) -> &dyn ruler::MusicalInfo;
    /// Shift the timeline start by the given number of ticks due to a scroll event.
    fn shift_timeline_start(&mut self, ticks: f32);
    /// The timeline was scrolled with with `Ctrl` held down to zoom in/out.
    fn zoom(&mut self, y_delta: f32);
}

#[derive(Clone, Debug)]
pub struct Bar {
    /// The start and end offsets of the bar.
    pub tick_range: Range<f32>,
    /// The time signature of this bar.
    pub time_sig: TimeSig,
}

#[derive(Clone, Debug)]
pub struct TimeSig {
    pub top: u16,
    pub bottom: u16,
}

impl TimeSig {
    /// The number of beats per bar of this time signature.
    pub fn beats_per_bar(&self) -> f32 {
        4.0 * self.top as f32 / self.bottom as f32
    }
}

/// The result of setting the timeline, ready to start laying out tracks.
pub struct Set {
    tl: TimelineCtx,
    ui: egui::Ui,
}

/// Some context for the timeline, providing short-hand for setting some useful widgets.
pub struct TimelineCtx {
    /// The rect encompassing the total timeline.
    full_rect: egui::Rect,
    /// The rect of the current timeline area.
    rect: egui::Rect,
    /// The total number of ticks visible on the timeline.
    visible_ticks: f32,
}

/// Context for instantiating the playhead after all tracks have been set.
pub struct SetPlayhead {
    full_rect: egui::Rect,
}

impl Set {
    /// Allows for drawing some widgets in the background before showing the grid.
    ///
    /// Can be useful for subtly colouring different ranges, etc.
    pub fn background(mut self, background: impl FnOnce(&TimelineCtx, &mut egui::Ui)) -> Self {
        let Set { ref mut ui, ref tl } = self;
        ui.scope(|ui| background(tl, ui));
        self
    }

    /// Paints the grid over the timeline `Rect`.
    ///
    /// If using a custom `background`, you may wish to call this after.
    pub fn paint_grid(self, info: &dyn ruler::MusicalInfo) -> Self {
        let vis = self.ui.style().noninteractive();
        let mut stroke = vis.bg_stroke;
        let bar_color = stroke.color.linear_multiply(0.5);
        let step_even_color = stroke.color.linear_multiply(0.25);
        let step_odd_color = stroke.color.linear_multiply(0.125);
        let visible_len = self.tl.rect.width();
        let mut steps = ruler::Steps::new(info, visible_len, MIN_STEP_GAP);
        while let Some(step) = steps.next(info) {
            stroke.color = match step.index_in_bar {
                0 => bar_color,
                n if n % 2 == 0 => step_even_color,
                _ => step_odd_color,
            };
            let x = self.tl.rect.left() + step.x;
            let a = egui::Pos2::new(x, self.tl.rect.top());
            let b = egui::Pos2::new(x, self.tl.rect.bottom());
            self.ui.painter().line_segment([a, b], stroke);
        }
        self
    }

    /// Set some tracks that should be pinned to the top.
    ///
    /// Often useful for the ruler or other tracks that should always be visible.
    pub fn pinned_tracks(mut self, tracks: impl FnOnce(&TimelineCtx, &mut egui::Ui)) -> Self {
        let Self { ref mut ui, ref tl } = self;

        // Use no spacing by default so we can get exact position for line separator.
        let space_y = ui.style().spacing.item_spacing.y;
        ui.style_mut().spacing.item_spacing.y = 0.0;
        ui.scope(|ui| tracks(tl, ui));

        // Draw a line to mark end of the pinned tracks.
        let remaining = ui.available_rect_before_wrap_finite();
        let a = remaining.left_top();
        let b = remaining.right_top();
        let stroke = ui.style().visuals.noninteractive().bg_stroke;
        ui.painter().line_segment([a, b], stroke);

        // Add the exact space so the UI is aware.
        ui.add_space(stroke.width);

        // Return to default spacing.
        ui.style_mut().spacing.item_spacing.y = space_y;
        self.tl.rect = ui.available_rect_before_wrap_finite();
        self.ui.set_clip_rect(self.tl.rect);
        self
    }

    /// Set all remaining tracks for the timeline.
    ///
    /// These tracks will become vertically scrollable in the case that there are two many to fit
    /// on the view. The given `egui::Rect` is the viewport (visible area) relative to the
    /// timeline.
    pub fn tracks(
        mut self,
        tracks: impl FnOnce(&TimelineCtx, egui::Rect, &mut egui::Ui),
    ) -> SetPlayhead {
        let Self { ref mut ui, ref tl } = self;
        egui::ScrollArea::from_max_height(tl.rect.height())
            .enable_scrolling(!ui.input().modifiers.ctrl)
            .show_viewport(ui, |ui, view| tracks(tl, view, ui));
        let full_rect = tl.full_rect();
        SetPlayhead { full_rect }
    }
}

impl SetPlayhead {
    /// Instantiate the playhead over the top of the whole timeline.
    pub fn playhead(&self, ui: &mut egui::Ui, info: &mut dyn Playhead) -> egui::Response {
        playhead::set(ui, self.full_rect, info)
    }
}

impl TimelineCtx {
    /// Instantiate the playhead over the current area.
    ///
    /// Useful if you only want the playhead over some tracks. Otherwise, use the `SetPlayhead`
    /// context returned by the `tracks` method to set the playhead over the full timeline.
    pub fn playhead(&self, ui: &mut egui::Ui, info: &mut dyn Playhead) -> egui::Response {
        playhead::set(ui, self.rect, info)
    }

    /// The rectangle encompassing the current remaining area of the timeline.
    ///
    /// - For the `pinned_tracks` function, this is the whole timeline.
    /// - For the `tracks` function, this is the remainder of the timeline following the
    ///   `pinned_tracks` area.
    pub fn rect(&self) -> egui::Rect {
        self.rect
    }

    /// The rectangle encompassing the entire timeline area including both pinned and regular
    /// track areas.
    pub fn full_rect(&self) -> egui::Rect {
        self.full_rect
    }

    /// The number of visible ticks across the width of the timeline.
    pub fn visible_ticks(&self) -> f32 {
        self.visible_ticks
    }

    /// Short-hand for drawing a plot within the timeline UI.
    ///
    /// The same as `egui::plot::Plot::new`, but sets some useful defaults before returning.
    pub fn plot_ticks(&self, id_source: impl Hash, y: RangeInclusive<f32>) -> plot::Plot {
        let h = 64.0;
        plot::Plot::new(id_source)
            .allow_zoom(false)
            .allow_drag(false)
            .include_x(0.0)
            .include_x(self.visible_ticks)
            .include_y(*y.start())
            .include_y(*y.end())
            .show_x(false)
            .show_y(false)
            .legend(plot::Legend::default().position(plot::Corner::LeftTop))
            .show_background(false)
            .show_axes(false)
            .height(h)
    }
}

/// Set the timeline within the currently available rect.
pub fn set(ui: &mut egui::Ui, timeline: &mut dyn Timeline) -> Set {
    // The area of the timeline.
    let rect = ui.available_rect_before_wrap_finite();

    // Check whether or not we should scroll the timeline or zoom.
    if ui.rect_contains_pointer(rect) {
        let delta = ui.input().scroll_delta;
        if ui.input().raw.modifiers.ctrl {
            if delta.x != 0.0 || delta.y != 0.0 {
                timeline.zoom(delta.y - delta.x);
            }
        } else {
            if delta.x != 0.0 {
                let ticks_per_point = timeline.musical_ruler_info().ticks_per_point();
                timeline.shift_timeline_start(delta.x * ticks_per_point);
            }
        }
    }

    // Draw the background.
    let vis = ui.style().noninteractive();
    let bg_stroke = egui::Stroke {
        width: 0.0,
        ..vis.bg_stroke
    };
    ui.painter().rect(rect, 0.0, vis.bg_fill, bg_stroke);

    // The child widgets.
    let layout = egui::Layout::top_down(egui::Align::Min);
    let info = timeline.musical_ruler_info();
    let visible_ticks = info.ticks_per_point() * rect.width();
    let tl = TimelineCtx {
        full_rect: rect,
        rect,
        visible_ticks,
    };
    let ui = ui.child_ui(rect, layout);
    Set { tl, ui }
}
