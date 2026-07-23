//! Ambient ocean life for the underwater transcript field.
//!
//! One clear owner for schools of fish, jellyfish, kelp, bubbles, bio-luminescence,
//! and the rare whale cameo. Motion stays inside the existing delta/interpolation
//! path: this module never requests frames on its own.
//!
//! Under reduced motion, entities remain visible but static.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::tui::ocean::{self, OceanColumn};

/// Depth layers for parallax. Nearer life is larger, faster, and more visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Depth {
    Background,
    Midground,
    Foreground,
}

impl Depth {
    #[must_use]
    fn speed_scale(self) -> f64 {
        match self {
            Self::Background => 0.55,
            Self::Midground => 1.0,
            Self::Foreground => 1.45,
        }
    }

    #[must_use]
    fn ink_index(self) -> usize {
        match self {
            Self::Background => 1,
            Self::Midground | Self::Foreground => 0,
        }
    }
}

/// Creature density tier mirrored from shell width/height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeDensity {
    Sparse,
    Normal,
    Rich,
}

impl LifeDensity {
    #[must_use]
    pub fn from_area(area: Rect) -> Self {
        if area.width < 56 || area.height < 12 {
            Self::Sparse
        } else if area.width < 88 || area.height < 20 {
            Self::Normal
        } else {
            Self::Rich
        }
    }

    #[must_use]
    fn school_count(self) -> usize {
        // One cohesive school reads better than many lone arrows.
        match self {
            Self::Sparse => 1,
            Self::Normal => 1,
            Self::Rich => 2,
        }
    }

    #[must_use]
    fn school_size(self) -> usize {
        match self {
            Self::Sparse => 2,
            Self::Normal => 3,
            Self::Rich => 4,
        }
    }

    #[must_use]
    fn jellyfish_count(self) -> usize {
        // Jellies are easy to misread as noise — keep them rare and soft.
        match self {
            Self::Sparse => 0,
            Self::Normal => 1,
            Self::Rich => 1,
        }
    }

    #[must_use]
    fn kelp_count(self) -> usize {
        // Fewer denser fronds beat a forest of antenna stalks.
        match self {
            Self::Sparse => 2,
            Self::Normal => 3,
            Self::Rich => 4,
        }
    }

    #[must_use]
    fn bubble_streams(self) -> usize {
        match self {
            Self::Sparse => 1,
            Self::Normal => 2,
            Self::Rich => 2,
        }
    }

    #[must_use]
    fn bio_particles(self) -> usize {
        match self {
            Self::Sparse => 1,
            Self::Normal => 2,
            Self::Rich => 3,
        }
    }
}

/// Lower floors so smaller windows still retain some life (was 68×15).
/// Keep in sync with [`crate::tui::ocean::AMBIENT_MIN_WIDTH`].
pub const AMBIENT_MIN_WIDTH: u16 = crate::tui::ocean::AMBIENT_MIN_WIDTH;
pub const AMBIENT_MIN_HEIGHT: u16 = crate::tui::ocean::AMBIENT_MIN_HEIGHT;

/// Whale cameo state: brief breach → spout → fluke → submerge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhaleCameoPhase {
    Hidden,
    Breach,
    Spout,
    Fluke,
    Submerge,
}

/// Snapshot of ambient positions for one frame (memoized once per draw).
#[derive(Debug, Clone)]
struct FrameMarks {
    marks: Vec<AmbientMark>,
}

#[derive(Debug, Clone, Copy)]
struct AmbientMark {
    x: u16,
    y: u16,
    glyph: &'static str,
    depth: Depth,
    style_mod: Option<Modifier>,
}

/// Optional pointer reaction for fish dart / bubble rise.
#[derive(Debug, Clone, Copy, Default)]
pub struct AmbientCursor {
    pub column: u16,
    pub row: u16,
    /// When set, fish flee from this point for ~800 ms of shared ocean clock.
    pub flee_elapsed_ms: Option<u128>,
}

/// Optional whale cameo trigger (e.g. successful turn completion).
#[derive(Debug, Clone, Copy, Default)]
pub struct WhaleCameo {
    pub elapsed_ms: Option<u128>,
    /// Anchor column within the field (composer / center).
    pub anchor_x: u16,
    pub anchor_y: u16,
}

const WHALE_CAMEO_MS: u128 = 2_400;

/// Render ambient life into empty water cells of `area`.
pub fn render_ambient_life(
    area: Rect,
    buf: &mut Buffer,
    inks: (Color, Color),
    lines: &[Line<'static>],
    elapsed_ms: u128,
    animated: bool,
    cursor: AmbientCursor,
    whale: WhaleCameo,
) {
    if area.width < AMBIENT_MIN_WIDTH || area.height < AMBIENT_MIN_HEIGHT {
        return;
    }

    let density = LifeDensity::from_area(area);
    let frame = build_frame_marks(area, elapsed_ms, animated, density, cursor, whale);
    paint_marks(area, buf, inks, lines, &frame);
}

fn build_frame_marks(
    area: Rect,
    elapsed_ms: u128,
    animated: bool,
    density: LifeDensity,
    cursor: AmbientCursor,
    whale: WhaleCameo,
) -> FrameMarks {
    let mut marks = Vec::with_capacity(48);
    let t = if animated { elapsed_ms } else { 0 };

    // Leave the empty-state brand band (center third) mostly clear so life
    // frames the room instead of littering the hero whale + status lines.
    let quiet_top = (area.height / 5).max(2);
    let quiet_mid_lo = area.height.saturating_mul(2) / 5;
    let quiet_mid_hi = area.height.saturating_mul(3) / 5;

    // --- Cohesive fish schools (not lone arrows) ---
    // Leader + trailers share one path; every body uses the same fish glyph
    // family so it reads as a school, not random `>` punctuation.
    let school_n = density.school_count();
    let school_size = density.school_size();
    for i in 0..school_n {
        let depth = if i == 0 {
            Depth::Foreground
        } else {
            Depth::Midground
        };
        let span = ((area.width as f64 / (4.5 + f64::from(i as u16))) as u16).clamp(8, 28);
        let phase = (i as u128).saturating_mul(2_400);
        let step = (480.0 / depth.speed_scale()) as u128;
        let (drift, forward) = if animated {
            eased_drift(t, step.max(220), span, phase)
        } else {
            ((span / 3).max(1), i % 2 == 0)
        };
        let bob = if animated {
            sine_bob(t, 2_200 + phase, 1)
        } else {
            0
        };
        // Prefer lower / upper thirds — avoid the empty-state copy band.
        let mut base_y = match i {
            0 => area.height.saturating_mul(3) / 4,
            _ => (area.height / 5).max(1),
        }
        .saturating_add(bob)
        .min(area.height.saturating_sub(1));
        if base_y > quiet_mid_lo && base_y < quiet_mid_hi {
            base_y = if i == 0 {
                quiet_mid_hi
                    .saturating_add(1)
                    .min(area.height.saturating_sub(1))
            } else {
                quiet_mid_lo.saturating_sub(1)
            };
        }
        let base_x = match i {
            0 => area.width / 10,
            _ => area.width.saturating_mul(3) / 4,
        };
        let faces_right = if i == 0 { forward } else { !forward };
        let mut lead_x = if faces_right {
            base_x.saturating_add(drift)
        } else {
            base_x.saturating_add(span).saturating_sub(drift)
        };
        if let Some(flee_ms) = cursor.flee_elapsed_ms {
            let flee = fish_flee_offset(flee_ms);
            let ptr = cursor.column.saturating_sub(area.x);
            if lead_x.abs_diff(ptr) < 16 {
                if lead_x >= ptr {
                    lead_x = lead_x.saturating_add(flee);
                } else {
                    lead_x = lead_x.saturating_sub(flee);
                }
            }
        }
        let body = fish_body(faces_right, depth);
        let body_w = UnicodeWidthStr::width(body) as u16;
        let max_x = area.width.saturating_sub(body_w.max(1));
        lead_x = lead_x.min(max_x);

        // Spacing along the swim direction: tight school, slight vertical stagger.
        for m in 0..school_size {
            let gap = 3u16.saturating_add((m as u16) % 2); // 3–4 cols between bodies
            let ox = if faces_right {
                lead_x.saturating_sub(gap.saturating_mul(m as u16))
            } else {
                lead_x
                    .saturating_add(gap.saturating_mul(m as u16))
                    .min(max_x)
            };
            let oy = base_y
                .saturating_add(if m % 2 == 0 { 0 } else { 1 })
                .min(area.height.saturating_sub(1));
            let member_depth = if m == 0 { depth } else { Depth::Midground };
            marks.push(AmbientMark {
                x: ox,
                y: oy,
                glyph: fish_body(faces_right, member_depth),
                depth: member_depth,
                style_mod: if m == 0 { None } else { Some(Modifier::DIM) },
            });
        }
    }

    // --- Soft jellyfish (bell + thin trail — no hard │/⎞ junk) ---
    for j in 0..density.jellyfish_count() {
        let phase = 3_800u128.saturating_add((j as u128) * 2_900);
        let span = (area.width / 12).clamp(3, 10);
        let (drift, _) = if animated {
            eased_drift(t, 1_100, span, phase)
        } else {
            (span / 2, true)
        };
        let bob = if animated {
            sine_bob(t, 2_800 + phase, 1)
        } else {
            0
        };
        // Park jellies off to a side, above the quiet mid band.
        let x = (area.width.saturating_mul(4) / 5 + drift).min(area.width.saturating_sub(1));
        let y = quiet_top
            .saturating_add(bob)
            .min(quiet_mid_lo.saturating_sub(2));
        let bell = if animated {
            match (t.saturating_add(phase) / 700) % 2 {
                0 => "°",
                _ => "˚",
            }
        } else {
            "°"
        };
        marks.push(AmbientMark {
            x,
            y,
            glyph: bell,
            depth: Depth::Midground,
            style_mod: Some(Modifier::DIM),
        });
        if y + 1 < area.height {
            marks.push(AmbientMark {
                x,
                y: y + 1,
                glyph: "·",
                depth: Depth::Background,
                style_mod: Some(Modifier::DIM),
            });
        }
    }

    // --- Bottom-anchored seaweed (wavy fronds, not radio antennas) ---
    // Each frond is a short stack of soft wave glyphs that lean with sway.
    for k in 0..density.kelp_count() {
        let phase = (k as u128).saturating_mul(1_300);
        let sway_phase = if animated {
            ((t.saturating_add(phase) % 2_600) as f64 / 2_600.0) * std::f64::consts::TAU
        } else {
            0.0
        };
        // Keep fronds near the edges so the center brand stays clean.
        let edge_bias = if k % 2 == 0 {
            area.width / (density.kelp_count() as u16 * 2 + 2)
        } else {
            area.width
                .saturating_sub(area.width / (density.kelp_count() as u16 * 2 + 2))
        };
        let base_x = edge_bias
            .saturating_add((k as u16) * 2)
            .min(area.width.saturating_sub(2));
        let frond_h = 3u16.saturating_add((k % 2) as u16); // 3–4 cells tall
        for h in 0..frond_h {
            // Higher segments lean more with the current.
            let lean = (sway_phase.sin() * (0.4 + 0.35 * f64::from(h))).round() as i16;
            let x = if lean >= 0 {
                base_x.saturating_add(lean as u16)
            } else {
                base_x.saturating_sub((-lean) as u16)
            }
            .min(area.width.saturating_sub(1));
            let y = area.height.saturating_sub(1 + h);
            marks.push(AmbientMark {
                x,
                y,
                glyph: seaweed_glyph(h, frond_h, lean),
                depth: Depth::Background,
                style_mod: Some(Modifier::DIM),
            });
        }
    }

    // --- Rising bubble streams (quiet ·/˚ only) ---
    for b in 0..density.bubble_streams() {
        let phase = (b as u128).saturating_mul(1_900);
        // Edge columns — avoid center brand.
        let column = if b % 2 == 0 {
            area.width / 8
        } else {
            area.width.saturating_mul(7) / 8
        };
        let rise_period = 3_200u128.saturating_add(phase % 900);
        let rise = if animated {
            let cycle = (t.saturating_add(phase) % rise_period) as f64 / rise_period as f64;
            let max_rise = area.height.saturating_sub(3) as f64;
            (cycle * max_rise) as u16
        } else {
            area.height / 4
        };
        let boost = if cursor.flee_elapsed_ms.is_some()
            && column.abs_diff(cursor.column.saturating_sub(area.x)) < 10
        {
            2
        } else {
            0
        };
        let y = area
            .height
            .saturating_sub(2)
            .saturating_sub(rise.saturating_add(boost))
            .max(quiet_top);
        // Skip the empty-state text band.
        if y > quiet_mid_lo && y < quiet_mid_hi {
            continue;
        }
        let glyph = if animated {
            ["·", "˚", "·", "°"][((t.saturating_add(phase)) / 320) as usize % 4]
        } else {
            "·"
        };
        marks.push(AmbientMark {
            x: column.min(area.width.saturating_sub(1)),
            y,
            glyph,
            depth: Depth::Foreground,
            style_mod: Some(Modifier::DIM),
        });
    }

    // --- Very sparse bioluminescent dust (edges only) ---
    for p in 0..density.bio_particles() {
        let seed = (p as u128).saturating_mul(9973).saturating_add(13);
        let x = if p % 2 == 0 {
            ((seed.wrapping_mul(17).wrapping_add(t / 5_000)) % u128::from((area.width / 4).max(1)))
                as u16
        } else {
            area.width.saturating_sub(
                1 + ((seed.wrapping_mul(19).wrapping_add(t / 5_000))
                    % u128::from((area.width / 4).max(1))) as u16,
            )
        };
        let y = quiet_top.saturating_add(
            ((seed.wrapping_mul(31).wrapping_add(t / 6_000))
                % u128::from(area.height.saturating_sub(quiet_top + 2).max(1))) as u16,
        );
        if y > quiet_mid_lo && y < quiet_mid_hi {
            continue;
        }
        let twinkle = if animated {
            ((t.saturating_add(seed) / 800) % 5) < 2
        } else {
            p == 0
        };
        if !twinkle {
            continue;
        }
        marks.push(AmbientMark {
            x: x.min(area.width.saturating_sub(1)),
            y: y.min(area.height.saturating_sub(1)),
            glyph: "·",
            depth: Depth::Background,
            style_mod: Some(Modifier::DIM),
        });
    }

    // --- Rare whale cameo (completion only) ---
    if let Some(cameo_ms) = whale.elapsed_ms.filter(|ms| *ms < WHALE_CAMEO_MS) {
        let phase = whale_cameo_phase(cameo_ms);
        if phase != WhaleCameoPhase::Hidden {
            let ax = whale
                .anchor_x
                .saturating_sub(area.x)
                .min(area.width.saturating_sub(4));
            let ay = whale
                .anchor_y
                .saturating_sub(area.y)
                .min(area.height.saturating_sub(2));
            let (glyph, y_off) = match phase {
                WhaleCameoPhase::Breach => ("≈≈>", 0u16),
                WhaleCameoPhase::Spout => ("≈≈>", 0),
                WhaleCameoPhase::Fluke => ("～", 1),
                WhaleCameoPhase::Submerge => ("·", 1),
                WhaleCameoPhase::Hidden => ("", 0),
            };
            if !glyph.is_empty() {
                marks.push(AmbientMark {
                    x: ax,
                    y: ay.saturating_add(y_off).min(area.height.saturating_sub(1)),
                    glyph,
                    depth: Depth::Foreground,
                    style_mod: None,
                });
                if phase == WhaleCameoPhase::Spout && ay > 0 {
                    marks.push(AmbientMark {
                        x: ax.saturating_add(1).min(area.width.saturating_sub(1)),
                        y: ay.saturating_sub(1),
                        glyph: "˚",
                        depth: Depth::Foreground,
                        style_mod: Some(Modifier::DIM),
                    });
                }
            }
        }
    }

    FrameMarks { marks }
}

/// Soft seaweed frond segment. Stacked parentheses/curves lean with current —
/// classic terminal kelp, not `|` poles topped with carets.
fn seaweed_glyph(segment_from_bottom: u16, frond_h: u16, lean: i16) -> &'static str {
    let is_tip = segment_from_bottom + 1 == frond_h;
    let is_base = segment_from_bottom == 0;
    // Tip feathers; base roots; mid is a soft S-curve of `)` / `(`.
    if is_tip {
        return if lean >= 0 { ")" } else { "(" };
    }
    if is_base {
        return "~";
    }
    match (lean.signum(), segment_from_bottom % 2) {
        (1 | 0, 0) => ")",
        (1 | 0, _) => ")",
        (-1, 0) => "(",
        _ => "(",
    }
}

fn paint_marks(
    area: Rect,
    buf: &mut Buffer,
    inks: (Color, Color),
    lines: &[Line<'static>],
    frame: &FrameMarks,
) {
    for mark in &frame.marks {
        let protected = lines
            .get(usize::from(mark.y))
            .and_then(occupied_text_bounds);
        let mark_width = UnicodeWidthStr::width(mark.glyph);
        let collides = protected.is_some_and(|(start, end)| {
            usize::from(mark.x) < end.saturating_add(1)
                && usize::from(mark.x) + mark_width > start.saturating_sub(1)
        });
        if collides || mark.x.saturating_add(mark_width as u16) > area.width {
            continue;
        }
        let fg = if mark.depth.ink_index() == 1 {
            inks.1
        } else {
            inks.0
        };
        let mut style = Style::default().fg(fg);
        if let Some(m) = mark.style_mod {
            style = style.add_modifier(m);
        }
        for (offset, ch) in mark.glyph.chars().enumerate() {
            let cell = &mut buf[(area.x + mark.x + offset as u16, area.y + mark.y)];
            cell.set_symbol(&ch.to_string());
            cell.set_style(style);
        }
    }
}

/// Width-only occupied-text measurement (no per-line String allocation).
#[must_use]
pub fn occupied_text_bounds(line: &Line<'_>) -> Option<(usize, usize)> {
    if line.spans.is_empty() {
        return None;
    }
    let mut total = 0usize;
    let mut leading = 0usize;
    let mut seen_non_ws = false;
    let mut trailing_run = 0usize;

    for span in &line.spans {
        for ch in span.content.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0);
            total = total.saturating_add(w);
            if ch.is_whitespace() {
                if !seen_non_ws {
                    leading = leading.saturating_add(w);
                } else {
                    trailing_run = trailing_run.saturating_add(w);
                }
            } else {
                seen_non_ws = true;
                trailing_run = 0;
            }
        }
    }
    if !seen_non_ws {
        return None;
    }
    Some((leading, total.saturating_sub(trailing_run)))
}

/// Cosine-eased drift (replaces mechanical linear ping-pong feel).
#[must_use]
pub fn eased_drift(elapsed_ms: u128, step_ms: u128, span: u16, phase_ms: u128) -> (u16, bool) {
    if span == 0 || step_ms == 0 {
        return (0, true);
    }
    let leg_ms = step_ms.saturating_mul(u128::from(span));
    let period_ms = leg_ms.saturating_mul(2);
    let phase = (elapsed_ms.saturating_add(phase_ms)) % period_ms;
    let (leg_elapsed, forward) = if phase <= leg_ms {
        (phase, true)
    } else {
        (phase.saturating_sub(leg_ms), false)
    };
    let progress = leg_elapsed as f64 / leg_ms as f64;
    let eased = (1.0 - (progress * std::f64::consts::PI).cos()) * 0.5;
    let position = if forward { eased } else { 1.0 - eased };
    ((position * f64::from(span)).round() as u16, forward)
}

#[must_use]
fn sine_bob(elapsed_ms: u128, period_ms: u128, amplitude: u16) -> u16 {
    if period_ms == 0 || amplitude == 0 {
        return 0;
    }
    let phase = (elapsed_ms % period_ms) as f64 / period_ms as f64;
    let s = (phase * std::f64::consts::TAU).sin();
    // Map [-1,1] → [0, amplitude]
    (((s + 1.0) * 0.5) * f64::from(amplitude)).round() as u16
}

/// One-shot flee arc keyed to Working transition / pointer motion.
#[must_use]
pub fn fish_flee_offset(elapsed_ms: u128) -> u16 {
    let progress = elapsed_ms.min(800) as f32 / 800.0;
    let excursion = (progress * std::f32::consts::PI).sin() * 9.0;
    excursion.round().clamp(0.0, 9.0) as u16
}

/// One fish silhouette family for the whole school. Depth is color/dim only —
/// never mix lone `>` with `><>` (that read as broken punctuation).
#[must_use]
fn fish_body(facing_right: bool, _depth: Depth) -> &'static str {
    if facing_right { "><>" } else { "<><" }
}

#[must_use]
pub fn whale_cameo_phase(elapsed_ms: u128) -> WhaleCameoPhase {
    match elapsed_ms {
        0..400 => WhaleCameoPhase::Breach,
        400..1_000 => WhaleCameoPhase::Spout,
        1_000..1_700 => WhaleCameoPhase::Fluke,
        1_700..WHALE_CAMEO_MS => WhaleCameoPhase::Submerge,
        _ => WhaleCameoPhase::Hidden,
    }
}

/// Subtle caustic shimmer applied to empty water cells when the field would
/// otherwise read as a static ramp. Cheap: one phase lookup per cell, only
/// when `animated` and density allows.
pub fn apply_caustic_shimmer(
    area: Rect,
    buf: &mut Buffer,
    column: &OceanColumn,
    elapsed_ms: u128,
    animated: bool,
    lines: &[Line<'static>],
) {
    if !animated || area.width < AMBIENT_MIN_WIDTH || area.height < AMBIENT_MIN_HEIGHT {
        return;
    }
    // Sparse sampling: every 3rd column on every other row near the surface.
    let band = (area.height / 3).max(2);
    for local_y in 0..band {
        let protected = lines
            .get(usize::from(local_y))
            .and_then(occupied_text_bounds);
        let ramp = frame_ocean_ramp(
            column,
            area.height,
            area.y,
            elapsed_ms,
            column.phase_tag(),
            column.ramp_fingerprint(),
        );
        let row_bg = ramp
            .get(usize::from(local_y))
            .copied()
            .unwrap_or_else(|| column.color_at_y(area.y.saturating_add(local_y)));
        for local_x in (0..area.width).step_by(3) {
            if protected.is_some_and(|(start, end)| {
                usize::from(local_x) >= start && usize::from(local_x) < end
            }) {
                continue;
            }
            let phase = ((elapsed_ms / 80)
                .wrapping_add(u128::from(local_x))
                .wrapping_add(u128::from(local_y) * 3))
                % 12;
            if phase > 2 {
                continue;
            }
            let cell = &mut buf[(area.x + local_x, area.y + local_y)];
            // Soften toward ambient ink without replacing semantic glyphs.
            if cell.symbol() == " " || cell.symbol().is_empty() {
                let shimmer = ocean::scale_color(row_bg, 1.08);
                cell.set_bg(shimmer);
            }
        }
    }
}

/// Cached ocean row colors invalidated only when phase/dimensions/palette/breath tick.
/// Shared across widgets that paint the same [`OceanColumn`] within a frame.
#[derive(Debug, Clone, Default)]
pub struct OceanRampCache {
    colors: Vec<Color>,
    height: u16,
    top: u16,
    elapsed_bucket: u128,
    phase_tag: u8,
    ramp_fingerprint: u64,
}

impl OceanRampCache {
    /// Return a per-row color ramp, recomputing only when inputs change.
    pub fn colors_for(
        &mut self,
        column: &OceanColumn,
        height: u16,
        top: u16,
        elapsed_ms: u128,
        phase_tag: u8,
        ramp_fingerprint: u64,
    ) -> &[Color] {
        // Breath cycle is 90s; bucket at ~80ms atmosphere cadence so we don't
        // recompute every draw when nothing visible changed.
        let bucket = elapsed_ms / 80;
        if self.colors.len() == usize::from(height)
            && self.height == height
            && self.top == top
            && self.elapsed_bucket == bucket
            && self.phase_tag == phase_tag
            && self.ramp_fingerprint == ramp_fingerprint
        {
            return &self.colors;
        }
        self.colors.clear();
        self.colors.reserve(usize::from(height));
        for local_y in 0..height {
            self.colors
                .push(column.color_at_y(top.saturating_add(local_y)));
        }
        self.height = height;
        self.top = top;
        self.elapsed_bucket = bucket;
        self.phase_tag = phase_tag;
        self.ramp_fingerprint = ramp_fingerprint;
        &self.colors
    }
}

thread_local! {
    static FRAME_RAMP: std::cell::RefCell<OceanRampCache> =
        const { std::cell::RefCell::new(OceanRampCache {
            colors: Vec::new(),
            height: 0,
            top: 0,
            elapsed_bucket: 0,
            phase_tag: 0,
            ramp_fingerprint: 0,
        }) };
}

/// Process-local per-frame ocean ramp shared by chat field, caustics, and
/// other widgets that paint the same column.
#[must_use]
pub fn frame_ocean_ramp(
    column: &OceanColumn,
    height: u16,
    top: u16,
    elapsed_ms: u128,
    phase_tag: u8,
    ramp_fingerprint: u64,
) -> Vec<Color> {
    FRAME_RAMP.with(|cache| {
        cache
            .borrow_mut()
            .colors_for(column, height, top, elapsed_ms, phase_tag, ramp_fingerprint)
            .to_vec()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Span;

    #[test]
    fn ambient_min_dimensions_allow_small_windows() {
        assert!(AMBIENT_MIN_WIDTH < 68);
        assert!(AMBIENT_MIN_HEIGHT < 15);
    }

    #[test]
    fn eased_drift_is_continuous_and_settles() {
        let span = 12u16;
        let step = 400u128;
        let (a, _) = eased_drift(0, step, span, 0);
        let (b, _) = eased_drift(step * 6, step, span, 0);
        assert!(a <= span);
        assert!(b <= span);
    }

    #[test]
    fn occupied_text_bounds_skips_string_join() {
        let line = Line::from(vec![Span::raw("  hello  "), Span::raw("world  ")]);
        let (start, end) = occupied_text_bounds(&line).expect("bounds");
        assert_eq!(start, 2);
        assert!(end > start);
    }

    #[test]
    fn whale_cameo_is_brief() {
        assert_eq!(whale_cameo_phase(0), WhaleCameoPhase::Breach);
        assert_eq!(whale_cameo_phase(500), WhaleCameoPhase::Spout);
        assert_eq!(whale_cameo_phase(1_200), WhaleCameoPhase::Fluke);
        assert_eq!(whale_cameo_phase(2_000), WhaleCameoPhase::Submerge);
        assert_eq!(whale_cameo_phase(3_000), WhaleCameoPhase::Hidden);
    }

    #[test]
    fn density_scales_with_area() {
        assert_eq!(
            LifeDensity::from_area(Rect::new(0, 0, 40, 10)),
            LifeDensity::Sparse
        );
        assert_eq!(
            LifeDensity::from_area(Rect::new(0, 0, 100, 30)),
            LifeDensity::Rich
        );
    }

    #[test]
    fn fish_school_uses_one_silhouette_family() {
        // Never mix lone `>` with full fish bodies.
        for depth in [Depth::Foreground, Depth::Midground, Depth::Background] {
            assert_eq!(fish_body(true, depth), "><>");
            assert_eq!(fish_body(false, depth), "<><");
        }
    }

    #[test]
    fn seaweed_is_not_antenna_poles() {
        // Old execution used `│` + `⌃` and read as radio masts.
        for h in 0..4 {
            let g = seaweed_glyph(h, 4, 1);
            assert_ne!(g, "│", "segment {h}");
            assert_ne!(g, "⌃", "segment {h}");
            assert_ne!(g, "|", "segment {h}");
        }
        assert_eq!(seaweed_glyph(0, 4, 0), "~");
        assert!(matches!(seaweed_glyph(3, 4, 1), ")" | "("));
    }
}
