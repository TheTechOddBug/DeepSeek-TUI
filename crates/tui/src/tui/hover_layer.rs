//! Frame-scoped hover registry for transcript / diff / tool surfaces.
#![allow(dead_code)] // Public hover API; surfaces adopt pieces incrementally.
//!
//! Collects hit targets during render, resolves the pointer once, and applies
//! restrained aura / copy / link glow. Reuses [`super::hover_hit`] primitives
//! and context-menu hover-follow patterns without growing `ui.rs`.

use std::cell::RefCell;
use std::sync::Mutex;
use std::time::Instant;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::palette;
use crate::tui::hover_hit::{
    HoverHit, HoverTargetKind, copy_affordance, cursor_shape_for, hit_test, hover_aura_style,
    link_hover_style, tooltip_line,
};

/// Pointer position from the last mouse move (column, row).
static POINTER: Mutex<Option<(u16, u16)>> = Mutex::new(None);

// Targets registered for the current frame (thread-local for render path).
thread_local! {
    static FRAME_TARGETS: RefCell<Vec<HoverHit>> = const { RefCell::new(Vec::new()) };
    static FRAME_HOVER: RefCell<Option<HoverHit>> = const { RefCell::new(None) };
    static FRAME_START: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

/// Clear targets at the start of a draw.
pub fn begin_frame() {
    FRAME_TARGETS.with(|t| t.borrow_mut().clear());
    FRAME_HOVER.with(|h| *h.borrow_mut() = None);
    FRAME_START.with(|s| *s.borrow_mut() = Some(Instant::now()));
}

/// Record an interactive region for hit-testing this frame.
pub fn register(hit: HoverHit) {
    FRAME_TARGETS.with(|t| t.borrow_mut().push(hit));
}

/// Convenience: register a rectangular target.
pub fn register_rect(kind: HoverTargetKind, area: Rect, label: impl Into<String>, copyable: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    register(HoverHit {
        kind,
        area,
        label: label.into(),
        copyable,
    });
}

/// Update the shared pointer from mouse motion (call from mouse_ui).
pub fn set_pointer(column: u16, row: u16) {
    if let Ok(mut guard) = POINTER.lock() {
        *guard = Some((column, row));
    }
}

/// Clear pointer (e.g. leave alternate screen).
pub fn clear_pointer() {
    if let Ok(mut guard) = POINTER.lock() {
        *guard = None;
    }
}

/// Resolve hover after targets are registered; call once near end of draw.
pub fn resolve_hover() {
    let pointer = POINTER.lock().ok().and_then(|g| *g);
    let Some((col, row)) = pointer else {
        FRAME_HOVER.with(|h| *h.borrow_mut() = None);
        return;
    };
    FRAME_TARGETS.with(|t| {
        let targets = t.borrow();
        let hit = hit_test(col, row, &targets).cloned();
        FRAME_HOVER.with(|h| *h.borrow_mut() = hit);
    });
}

/// Current hover hit, if any.
#[must_use]
pub fn current_hover() -> Option<HoverHit> {
    FRAME_HOVER.with(|h| h.borrow().clone())
}

/// Preferred cursor shape for the active hover (best-effort string token).
#[must_use]
pub fn active_cursor_shape() -> Option<&'static str> {
    current_hover().map(|h| cursor_shape_for(h.kind))
}

/// Elapsed ms since frame begin for pulse math.
fn elapsed_ms() -> u128 {
    FRAME_START
        .with(|s| s.borrow().map(|t| t.elapsed().as_millis()))
        .unwrap_or(0)
}

/// Apply restrained aura to a hovered rect on the buffer.
pub fn paint_aura(
    buf: &mut Buffer,
    area: Rect,
    accent: ratatui::style::Color,
    reduced_motion: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let ms = elapsed_ms();
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            if x >= buf.area.x.saturating_add(buf.area.width)
                || y >= buf.area.y.saturating_add(buf.area.height)
            {
                continue;
            }
            let cell = &mut buf[(x, y)];
            let base = cell.bg;
            let style = hover_aura_style(base, accent, reduced_motion, ms);
            if let Some(bg) = style.bg {
                cell.set_bg(bg);
            }
        }
    }
}

/// Paint OSC-8 / file-ref underline glow on a hovered link span row.
pub fn paint_link_glow(
    buf: &mut Buffer,
    area: Rect,
    fg: ratatui::style::Color,
    reduced_motion: bool,
) {
    let ms = elapsed_ms();
    let style = link_hover_style(fg, reduced_motion, ms);
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            if x >= buf.area.x.saturating_add(buf.area.width)
                || y >= buf.area.y.saturating_add(buf.area.height)
            {
                continue;
            }
            let cell = &mut buf[(x, y)];
            if let Some(color) = style.fg {
                cell.set_fg(color);
            }
            cell.modifier.insert(Modifier::UNDERLINED);
        }
    }
}

/// Hover-only copy chip line for code/diff/text surfaces.
#[must_use]
pub fn copy_chip_line(max_width: u16) -> Line<'static> {
    let text = copy_affordance();
    let budget = usize::from(max_width.max(1));
    let label = if unicode_width::UnicodeWidthStr::width(text) > budget {
        "⧉".to_string()
    } else {
        text.to_string()
    };
    Line::from(Span::styled(
        label,
        Style::default()
            .fg(palette::TEXT_HINT)
            .add_modifier(Modifier::DIM),
    ))
}

/// Short tooltip for file refs / tool cards when hovered.
#[must_use]
pub fn hover_tooltip(max_width: usize) -> Option<String> {
    let hit = current_hover()?;
    if hit.label.trim().is_empty() {
        return None;
    }
    match hit.kind {
        HoverTargetKind::FileRef | HoverTargetKind::ToolCard | HoverTargetKind::Link => {
            Some(tooltip_line(&hit.label, max_width.max(8)))
        }
        _ => None,
    }
}

/// Apply all hover effects for the resolved target onto `buf`.
pub fn apply_resolved_effects(
    buf: &mut Buffer,
    reduced_motion: bool,
    accent: ratatui::style::Color,
) {
    resolve_hover();
    let Some(hit) = current_hover() else {
        return;
    };
    match hit.kind {
        HoverTargetKind::Link | HoverTargetKind::FileRef => {
            paint_link_glow(buf, hit.area, palette::WHALE_ACTION, reduced_motion);
        }
        HoverTargetKind::Code
        | HoverTargetKind::Diff
        | HoverTargetKind::ToolCard
        | HoverTargetKind::DiffAction
        | HoverTargetKind::Plain
        | HoverTargetKind::MenuRow => {
            paint_aura(buf, hit.area, accent, reduced_motion);
        }
    }
    // Hover-only copy chip on the trailing edge of copyable targets.
    if hit.copyable && hit.area.width > 8 {
        let chip = copy_affordance();
        let chip_w = unicode_width::UnicodeWidthStr::width(chip) as u16;
        if chip_w < hit.area.width {
            let x = hit
                .area
                .x
                .saturating_add(hit.area.width.saturating_sub(chip_w + 1));
            let y = hit.area.y;
            for (i, ch) in chip.chars().enumerate() {
                let cx = x.saturating_add(i as u16);
                if cx >= buf.area.x.saturating_add(buf.area.width) {
                    break;
                }
                let cell = &mut buf[(cx, y)];
                cell.set_symbol(&ch.to_string());
                cell.set_fg(palette::TEXT_HINT);
                cell.modifier.insert(Modifier::DIM);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Global POINTER is process-wide; serialize tests that touch it.
    static HOVER_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn register_and_resolve_hit() {
        let _guard = HOVER_TEST_LOCK.lock().unwrap();
        clear_pointer();
        begin_frame();
        set_pointer(5, 2);
        register_rect(
            HoverTargetKind::Code,
            Rect::new(0, 2, 20, 1),
            "fn main",
            true,
        );
        resolve_hover();
        let hit = current_hover().expect("hover");
        assert_eq!(hit.kind, HoverTargetKind::Code);
        assert!(hit.copyable);
        clear_pointer();
    }

    #[test]
    fn tooltip_only_for_file_and_tool() {
        let _guard = HOVER_TEST_LOCK.lock().unwrap();
        clear_pointer();
        begin_frame();
        set_pointer(1, 1);
        register_rect(
            HoverTargetKind::FileRef,
            Rect::new(0, 1, 10, 1),
            "src/lib.rs",
            false,
        );
        resolve_hover();
        let tip = hover_tooltip(40).expect("tooltip");
        assert!(tip.contains("lib.rs"), "{tip}");
        clear_pointer();
    }
}
