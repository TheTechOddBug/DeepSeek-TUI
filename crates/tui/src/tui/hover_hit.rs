//! Shared hover-hit abstraction for interactive terminal surfaces.
//!
//! Reused by the context menu, transcript cells, diff footers, OSC-8 links,
//! code blocks, file references, and tool cards. Keeps a cheap hit-test alive
//! while streaming without forcing expensive transcript reflow.

// Public API surface; hover_layer + mouse_ui consume these primitives.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use unicode_width::UnicodeWidthStr;

use crate::tui::ocean;

/// Kind of interactive surface under the pointer.
///
/// Surfaces adopt kinds incrementally (link regions first; menu/code/diff as
/// they land). Variants stay exhaustive for cursor + aura match arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // reserved kinds emit as more surfaces register hits
pub enum HoverTargetKind {
    Plain,
    Link,
    Code,
    Diff,
    FileRef,
    ToolCard,
    MenuRow,
    DiffAction,
}

/// Result of a hover hit-test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverHit {
    pub kind: HoverTargetKind,
    pub area: Rect,
    /// Optional label for tooltips / copy affordances.
    pub label: String,
    /// Whether a hover-only `copy` chip should be shown.
    pub copyable: bool,
}

/// Aura style for a hovered interactive cell.
#[must_use]
pub fn hover_aura_style(
    base_bg: Color,
    accent: Color,
    reduced_motion: bool,
    elapsed_ms: u128,
) -> Style {
    let amount = if reduced_motion {
        0.18
    } else {
        // Gentle ~1 Hz pulse frozen under reduced motion.
        let phase = (elapsed_ms % 1_000) as f32 / 1_000.0;
        let s = (phase * std::f32::consts::TAU).sin();
        0.14 + s.abs() * 0.08
    };
    let bg = ocean::mix_colors(base_bg, accent, amount);
    Style::default().bg(bg)
}

/// Underline + glow for OSC-8 / file links under the pointer.
#[must_use]
pub fn link_hover_style(fg: Color, reduced_motion: bool, elapsed_ms: u128) -> Style {
    let scale = if reduced_motion {
        1.15
    } else {
        let phase = (elapsed_ms % 1_200) as f32 / 1_200.0;
        1.10 + (phase * std::f32::consts::TAU).sin().abs() * 0.12
    };
    Style::default()
        .fg(ocean::scale_color(fg, scale))
        .add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
}

/// Hover-only `copy` chip text (display width fixed).
#[must_use]
pub fn copy_affordance() -> &'static str {
    "⧉ copy"
}

/// Whether `column,row` hits `area`.
#[must_use]
pub fn point_in_rect(column: u16, row: u16, area: Option<Rect>) -> bool {
    let Some(area) = area else {
        return false;
    };
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

/// Hit-test a list of rectangular targets; returns the topmost match.
#[must_use]
pub fn hit_test(column: u16, row: u16, targets: &[HoverHit]) -> Option<&HoverHit> {
    targets
        .iter()
        .rev()
        .find(|t| point_in_rect(column, row, Some(t.area)))
}

/// Preferred terminal cursor shape for a hover target (best-effort hint).
#[must_use]
pub fn cursor_shape_for(kind: HoverTargetKind) -> &'static str {
    match kind {
        HoverTargetKind::Link | HoverTargetKind::FileRef | HoverTargetKind::MenuRow => "pointer",
        HoverTargetKind::Code | HoverTargetKind::Diff | HoverTargetKind::Plain => "text",
        HoverTargetKind::ToolCard | HoverTargetKind::DiffAction => "pointer",
    }
}

/// Build a compact tooltip line that fits `max_width`.
#[must_use]
pub fn tooltip_line(label: &str, max_width: usize) -> String {
    let trimmed = label.trim();
    if UnicodeWidthStr::width(trimmed) <= max_width {
        return trimmed.to_string();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let mut out = String::new();
    let mut w = 0usize;
    let limit = max_width.saturating_sub(3);
    for ch in trimmed.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > limit {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_returns_topmost() {
        let targets = vec![
            HoverHit {
                kind: HoverTargetKind::Plain,
                area: Rect::new(0, 0, 10, 1),
                label: "a".into(),
                copyable: false,
            },
            HoverHit {
                kind: HoverTargetKind::Link,
                area: Rect::new(2, 0, 4, 1),
                label: "b".into(),
                copyable: true,
            },
        ];
        let hit = hit_test(3, 0, &targets).expect("hit");
        assert_eq!(hit.kind, HoverTargetKind::Link);
    }

    #[test]
    fn copy_affordance_is_stable() {
        assert_eq!(copy_affordance(), "⧉ copy");
    }
}
