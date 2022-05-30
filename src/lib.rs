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
pub trait TimelineApi {
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

/// The top-level timeline widget.
pub struct Timeline {
    /// A optional side panel with track headers.
    ///
    /// Can be useful for labelling tracks or providing convenient volume, mute, solo, etc style
    /// widgets.
    header: Option<f32>,
}

/// The result of setting the timeline, ready to start laying out tracks.
pub struct Show {
    tracks: TracksCtx,
    ui: egui::Ui,
}

/// A context for instantiating tracks, either pinned or unpinned.
pub struct TracksCtx {
    /// The rectangle encompassing the entire widget area including both header and timeline and
    /// both pinned and unpinned track areas.
    pub full_rect: egui::Rect,
    /// The rect encompassing the left-hand-side track headers including pinned and unpinned.
    pub header_full_rect: Option<egui::Rect>,
    /// Context specific to the timeline (non-header) area.
    pub timeline: TimelineCtx,
}

/// Some context for the timeline, providing short-hand for setting some useful widgets.
pub struct TimelineCtx {
    /// The total visible rect of the timeline area including pinned and unpinned tracks.
    pub full_rect: egui::Rect,
    /// The total number of ticks visible on the timeline area.
    pub visible_ticks: f32,
}

/// Context for instantiating the playhead after all tracks have been set.
pub struct SetPlayhead {
    timeline_rect: egui::Rect,
}

impl Timeline {
    /// Begin building the timeline widget.
    pub fn new() -> Self {
        Self { header: None }
    }

    /// A optional track header side panel.
    ///
    /// Can be useful for labelling tracks or providing convenient volume, mute, solo, etc style
    /// widgets.
    pub fn header(mut self, width: f32) -> Self {
        self.header = Some(width);
        self
    }

    /// Set the timeline within the currently available rect.
    pub fn show(self, ui: &mut egui::Ui, timeline: &mut dyn TimelineApi) -> Show {
        // The full area including both headers and timeline.
        let full_rect = ui.available_rect_before_wrap();
        // The area occupied by the timeline.
        let mut timeline_rect = full_rect;
        // The area occupied by track headers.
        let header_rect = self.header.map(|header_w| {
            let mut r = full_rect;
            r.set_width(header_w);
            timeline_rect.min.x = r.right();
            r
        });

        // Check whether or not we should scroll the timeline or zoom.
        if ui.rect_contains_pointer(timeline_rect) {
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
        ui.painter().rect(full_rect, 0.0, vis.bg_fill, bg_stroke);

        // The child widgets.
        let layout = egui::Layout::top_down(egui::Align::Min);
        let info = timeline.musical_ruler_info();
        let visible_ticks = info.ticks_per_point() * timeline_rect.width();
        let timeline = TimelineCtx {
            full_rect: timeline_rect,
            visible_ticks,
        };
        let tracks = TracksCtx {
            full_rect,
            header_full_rect: header_rect,
            timeline,
        };
        let ui = ui.child_ui(full_rect, layout);
        Show { tracks, ui }
    }
}

/// Relevant information for displaying a background for the timeline.
pub struct BackgroundCtx<'a> {
    pub header_full_rect: Option<egui::Rect>,
    pub timeline: &'a TimelineCtx,
}

impl Show {
    /// Allows for drawing some widgets in the background before showing the grid.
    ///
    /// Can be useful for subtly colouring different ranges, etc.
    pub fn background(mut self, background: impl FnOnce(&BackgroundCtx, &mut egui::Ui)) -> Self {
        let Show {
            ref mut ui,
            ref tracks,
        } = self;
        let bg = BackgroundCtx {
            header_full_rect: tracks.header_full_rect,
            timeline: &tracks.timeline,
        };
        background(&bg, ui);
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
        let tl_rect = self.tracks.timeline.full_rect;
        let visible_len = tl_rect.width();
        let mut steps = ruler::Steps::new(info, visible_len, MIN_STEP_GAP);
        while let Some(step) = steps.next(info) {
            stroke.color = match step.index_in_bar {
                0 => bar_color,
                n if n % 2 == 0 => step_even_color,
                _ => step_odd_color,
            };
            let x = tl_rect.left() + step.x;
            let a = egui::Pos2::new(x, tl_rect.top());
            let b = egui::Pos2::new(x, tl_rect.bottom());
            self.ui.painter().line_segment([a, b], stroke);
        }
        self
    }

    /// Set some tracks that should be pinned to the top.
    ///
    /// Often useful for the ruler or other tracks that should always be visible.
    pub fn pinned_tracks(mut self, tracks_fn: impl FnOnce(&TracksCtx, &mut egui::Ui)) -> Self {
        let Self {
            ref mut ui,
            ref tracks,
        } = self;

        // Use no spacing by default so we can get exact position for line separator.
        ui.scope(|ui| tracks_fn(tracks, ui));

        // Draw a line to mark end of the pinned tracks.
        let remaining = ui.available_rect_before_wrap();
        let a = remaining.left_top();
        let b = remaining.right_top();
        let stroke = ui.style().visuals.noninteractive().bg_stroke;
        ui.painter().line_segment([a, b], stroke);

        // Add the exact space so the UI is aware.
        ui.add_space(stroke.width);

        // Return to default spacing.
        let rect = ui.available_rect_before_wrap();
        self.ui.set_clip_rect(rect);
        self
    }

    /// Set all remaining tracks for the timeline.
    ///
    /// These tracks will become vertically scrollable in the case that there are two many to fit
    /// on the view. The given `egui::Rect` is the viewport (visible area) relative to the
    /// timeline.
    pub fn tracks(
        mut self,
        tracks_fn: impl FnOnce(&TracksCtx, egui::Rect, &mut egui::Ui),
    ) -> SetPlayhead {
        let Self {
            ref mut ui,
            ref tracks,
        } = self;
        let rect = ui.available_rect_before_wrap();
        let enable_scrolling = !ui.input().modifiers.ctrl;
        egui::ScrollArea::vertical()
            .max_height(rect.height())
            .enable_scrolling(enable_scrolling)
            .show_viewport(ui, |ui, view| tracks_fn(tracks, view, ui));
        let timeline_rect = tracks.timeline.full_rect;
        SetPlayhead { timeline_rect }
    }
}

impl SetPlayhead {
    /// Instantiate the playhead over the top of the whole timeline.
    pub fn playhead(&self, ui: &mut egui::Ui, info: &mut dyn Playhead) -> egui::Response {
        playhead::set(ui, self.timeline_rect, info)
    }
}

/// A type used to assist with setting a track with an optional `header`.
pub struct TrackCtx<'a> {
    tracks: &'a TracksCtx,
    ui: &'a mut egui::Ui,
    available_rect: egui::Rect,
    header_height: f32,
}

impl<'a> TrackCtx<'a> {
    /// UI for the track's header.
    pub fn header(mut self, header: impl FnOnce(&mut egui::Ui)) -> Self {
        let header_h = self
            .tracks
            .header_full_rect
            .map(|mut rect| {
                rect.min.y = self.available_rect.min.y;
                let ui = &mut self.ui.child_ui(rect, *self.ui.layout());
                header(ui);
                ui.min_rect().height()
            })
            .unwrap_or(0.0);
        self.header_height = header_h;
        self
    }

    /// Set the track, with a function for instantiating contents for the timeline.
    pub fn show(self, track: impl FnOnce(&TimelineCtx, &mut egui::Ui)) {
        // The UI and area for the track timeline.
        let track_h = {
            let mut rect = self.tracks.timeline.full_rect;
            rect.min.y = self.available_rect.min.y;
            let ui = &mut self.ui.child_ui(rect, *self.ui.layout());
            track(&self.tracks.timeline, ui);
            ui.min_rect().height()
        };
        // Manually add space occuppied by the child UIs, otherwise `ScrollArea` won't consider the
        // space occuppied. TODO: Is there a better way to handle this?
        let w = self.tracks.full_rect.width();
        let h = self.header_height.max(track_h);
        self.ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.spacing_mut().interact_size.y = 0.0;
            ui.horizontal(|ui| ui.add_space(w));
            ui.add_space(h);
        });
    }
}

impl TracksCtx {
    /// Begin showing the next `Track`.
    pub fn next<'a>(&'a self, ui: &'a mut egui::Ui) -> TrackCtx<'a> {
        let available_rect = ui.available_rect_before_wrap();
        TrackCtx {
            tracks: self,
            ui,
            available_rect,
            header_height: 0.0,
        }
    }
}

impl TimelineCtx {
    /// The number of visible ticks across the width of the timeline.
    pub fn visible_ticks(&self) -> f32 {
        self.visible_ticks
    }

    /// Short-hand for drawing a plot within the timeline UI.
    ///
    /// The same as `egui::plot::Plot::new`, but sets some useful defaults before returning.
    pub fn plot_ticks(&self, id_source: impl Hash, y: RangeInclusive<f32>) -> plot::Plot {
        let h = 72.0;
        plot::Plot::new(id_source)
            .allow_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false)
            .include_x(0.0)
            .include_x(self.visible_ticks)
            .include_y(*y.start())
            .include_y(*y.end())
            .show_x(false)
            .show_y(false)
            .legend(plot::Legend::default().position(plot::Corner::LeftTop))
            .show_background(false)
            .show_axes([false; 2])
            .height(h)
    }
}
