use super::Bar;

/// Access to musical information required by the timeline.
pub trait MusicalInfo {
    /// The number of ticks per beat, also known as PPQN (parts per quarter note).
    fn ticks_per_beat(&self) -> u32;
    /// The bar at the given tick offset starting from the beginning (left) of the timeline view.
    fn bar_at_ticks(&self, tick: f32) -> Bar;
    /// Affects how "zoomed" the timeline is. By default, uses 16 points per beat.
    fn ticks_per_point(&self) -> f32 {
        self.ticks_per_beat() as f32 / 16.0
    }
}

/// Respond to when the user clicks on the ruler.
pub trait MusicalInteract {
    /// The given tick location was clicked
    fn click_at_tick(&mut self, tick: f32);
}

/// The required API for the musical ruler widget.
pub trait MusicalRuler {
    fn info(&self) -> &dyn MusicalInfo;
    fn interact(&mut self) -> &mut dyn MusicalInteract;
}

/// Instantiate a musical ruler widget, showing bars and meters.
pub fn musical(ui: &mut egui::Ui, api: &mut dyn MusicalRuler) -> egui::Response {
    // Allocate space for the ruler.
    let h = ui.spacing().interact_size.y;
    let w = ui.available_width();
    let desired_size = egui::Vec2::new(w, h);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

    // Check for clicks.
    let w = rect.width();
    let ticks_per_point = api.info().ticks_per_point();
    let visible_ticks = w * ticks_per_point;
    if response.clicked() || response.dragged() {
        if let Some(pt) = response.interact_pointer_pos() {
            let tick = (((pt.x - rect.min.x) / w) * visible_ticks).max(0.0);
            api.interact().click_at_tick(tick);
            response.mark_changed();
        }
    }

    // Time to draw things.
    let vis = ui.style().noninteractive();

    // Draw each of the step lines.
    let mut stroke = vis.fg_stroke;
    let bar_color = stroke.color.linear_multiply(0.5);
    let step_color = stroke.color.linear_multiply(0.125);
    let bar_y = rect.center().y;
    let step_even_y = rect.top() + rect.height() * 0.25;
    let step_odd_y = rect.top() + rect.height() * 0.125;

    // Iterate over the steps of the ruler to draw them.
    let visible_len = w;
    let info = api.info();
    let mut steps = Steps::new(info, visible_len, super::MIN_STEP_GAP);
    while let Some(step) = steps.next(info) {
        let (y, color) = match step.index_in_bar {
            0 => (bar_y, bar_color),
            n if n % 2 == 0 => (step_even_y, step_color),
            _ => (step_odd_y, step_color),
        };
        stroke.color = color;
        let x = rect.left() + step.x;
        let a = egui::Pos2::new(x, rect.top());
        let b = egui::Pos2::new(x, y);
        ui.painter().line_segment([a, b], stroke);
    }

    response
}

#[derive(Copy, Clone, Debug)]
pub struct Step {
    /// The index of the step within the bar.
    ///
    /// The first step always indicates the start of the bar.
    pub index_in_bar: usize,
    /// The position of the step in ticks from the beginning of the start of the visible area.
    pub ticks: f32,
    /// The location of the step along the x axis from the start of the ruler.
    pub x: f32,
}

#[derive(Clone, Debug)]
pub struct Steps {
    ticks_per_beat: f32,
    ticks_per_point: f32,
    visible_ticks: f32,
    min_step_ticks: f32,
    index_in_bar: usize,
    step_ticks: f32,
    bar: Bar,
    ticks: f32,
}

impl Steps {
    /// Create a new `Steps`.
    pub fn new(api: &dyn MusicalInfo, visible_len: f32, min_step_gap: f32) -> Self {
        let ticks_per_beat = api.ticks_per_beat() as f32;
        let ticks_per_point = api.ticks_per_point();
        let visible_ticks = ticks_per_point * visible_len;
        let min_step_ticks = ticks_per_point * min_step_gap;
        Self {
            ticks_per_beat,
            ticks_per_point,
            visible_ticks,
            min_step_ticks,
            index_in_bar: 0,
            step_ticks: 0.0,
            bar: api.bar_at_ticks(0.0),
            ticks: 0.0,
        }
    }

    /// Produce the next `Step`.
    pub fn next(&mut self, api: &dyn MusicalInfo) -> Option<Step> {
        'bars: loop {
            // If this is the first step of the bar, update step interval.
            if self.index_in_bar == 0 {
                self.ticks = self.bar.tick_range.start;
                let mut beat_subdivs = self.bar.time_sig.bottom / 4;
                self.step_ticks = self.ticks_per_beat as f32 / beat_subdivs as f32;
                if self.step_ticks >= self.min_step_ticks {
                    loop {
                        let new_beat_subdivs = beat_subdivs * 2;
                        let new_step_ticks = self.ticks_per_beat as f32 / new_beat_subdivs as f32;
                        if new_step_ticks <= self.min_step_ticks {
                            break;
                        }
                        beat_subdivs = new_beat_subdivs;
                        self.step_ticks = new_step_ticks;
                    }
                } else {
                    self.step_ticks = self.bar.tick_range.end - self.bar.tick_range.start;
                }
            }

            'ticks: loop {
                if self.ticks > self.visible_ticks {
                    return None;
                }
                if self.ticks >= self.bar.tick_range.end {
                    self.index_in_bar = 0;
                    self.bar = api.bar_at_ticks(self.bar.tick_range.end + 0.5);
                    continue 'bars;
                }
                let index_in_bar = self.index_in_bar;
                let ticks = self.ticks;
                self.index_in_bar += 1;
                self.ticks += self.step_ticks;
                if ticks < 0.0 {
                    continue 'ticks;
                }
                let x = ticks / self.ticks_per_point;
                let step = Step { index_in_bar, ticks, x };
                return Some(step);
            }
        }
    }
}
