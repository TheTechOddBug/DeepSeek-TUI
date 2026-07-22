//! Terminal frame snapshot built from the PTY output stream.
//!
//! Wraps `vt100::Parser` so tests can feed bytes incrementally and ask
//! questions about the current screen contents (visible text, individual rows,
//! does-it-contain-this).

use std::time::Instant;

pub struct Frame {
    parser: vt100::Parser,
    captured_at: Option<Instant>,
}

impl Frame {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            parser: vt100::Parser::new(rows, cols, 0),
            captured_at: None,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.parser.process(bytes);
        self.captured_at = Some(Instant::now());
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.screen_mut().set_size(rows, cols);
    }

    pub fn rows(&self) -> u16 {
        self.parser.screen().size().0
    }

    pub fn cols(&self) -> u16 {
        self.parser.screen().size().1
    }

    /// Full visible screen as a single string with a `\n` between rows.
    /// Trailing whitespace on each row is preserved so column-position
    /// assertions stay meaningful.
    pub fn text(&self) -> String {
        self.parser.screen().contents()
    }

    /// Single row of the screen, 0-indexed from the top, trimmed at the
    /// right edge. Returns the empty string for out-of-range rows.
    ///
    /// Use `Screen::rows` rather than concatenating `Cell::contents()`.
    /// Untouched blank cells have no contents in `vt100`, but they still
    /// occupy a real terminal column between painted cells. Concatenating
    /// only cell contents therefore turns truthful PTY output such as
    /// `read running` into the misleading debug string `readrunning`.
    /// `Screen::rows` preserves those interior columns and skips the hidden
    /// continuation cell of a wide glyph.
    pub fn row(&self, y: u16) -> String {
        if y >= self.rows() {
            return String::new();
        }
        self.parser
            .screen()
            .rows(0, self.cols())
            .nth(usize::from(y))
            .unwrap_or_default()
    }

    pub fn contains(&self, needle: &str) -> bool {
        self.text().contains(needle)
    }

    /// First visible coordinate of `needle`, using terminal display columns.
    /// PTY mouse tests use this to click the row the renderer actually painted
    /// instead of hard-coding layout coordinates.
    pub fn find_text(&self, needle: &str) -> Option<(u16, u16)> {
        for row in 0..self.rows() {
            if let Some(col) = self.find_text_in_row(row, needle) {
                return Some((row, col));
            }
        }
        None
    }

    /// Locate text on one parsed terminal row without collapsing blank cells.
    /// `vt100::Cell::contents()` is empty for untouched spaces, which matters
    /// for transparent themes whose styled gaps do not need to be emitted.
    pub fn find_text_in_row(&self, row: u16, needle: &str) -> Option<u16> {
        if row >= self.rows() || needle.is_empty() {
            return None;
        }
        for start in 0..self.cols() {
            let mut col = start;
            let mut matched = true;
            for ch in needle.chars() {
                let Some(cell) = self.parser.screen().cell(row, col) else {
                    matched = false;
                    break;
                };
                let contents = cell.contents();
                let mut encoded = [0_u8; 4];
                let expected = ch.encode_utf8(&mut encoded);
                if if ch == ' ' {
                    !contents.is_empty() && contents != " "
                } else {
                    contents != expected
                } {
                    matched = false;
                    break;
                }
                let width = unicode_width::UnicodeWidthChar::width(ch)
                    .unwrap_or(0)
                    .max(1);
                let Ok(width) = u16::try_from(width) else {
                    return None;
                };
                col = col.saturating_add(width);
            }
            if matched {
                return Some(start);
            }
        }
        None
    }

    /// Foreground/background colors for one terminal cell. Theme QA uses the
    /// parsed ANSI result rather than trusting a screenshot renderer's own
    /// palette or accessibility environment.
    pub fn colors_at(&self, row: u16, col: u16) -> Option<(vt100::Color, vt100::Color)> {
        self.parser
            .screen()
            .cell(row, col)
            .map(|cell| (cell.fgcolor(), cell.bgcolor()))
    }

    /// Colors on the first cell whose terminal contents equal `symbol`.
    pub fn first_symbol_colors(&self, symbol: &str) -> Option<(vt100::Color, vt100::Color)> {
        for row in 0..self.rows() {
            for col in 0..self.cols() {
                let Some(cell) = self.parser.screen().cell(row, col) else {
                    continue;
                };
                if cell.contents() == symbol {
                    return Some((cell.fgcolor(), cell.bgcolor()));
                }
            }
        }
        None
    }

    /// Whether any row of the screen has non-blank content. Used to detect a
    /// fully detached / blank viewport.
    pub fn any_visible_text(&self) -> bool {
        self.text().chars().any(|c| !c.is_whitespace())
    }

    /// Cursor position as (row, col). Useful for asserting the composer
    /// owns the cursor (#1073) or that it is not at row 0 mid-frame.
    pub fn cursor(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    /// Render the screen to a string for diagnostic dumps when an
    /// assertion fails.
    pub fn debug_dump(&self) -> String {
        let (rows, cols) = (self.rows(), self.cols());
        let mut out = String::new();
        out.push_str(&format!(
            "== frame {rows}x{cols} cursor={:?} ==\n",
            self.cursor()
        ));
        for y in 0..rows {
            out.push_str(&format!("{y:>3} | {}\n", self.row(y).trim_end()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Frame;

    #[test]
    fn row_preserves_unpainted_interior_terminal_columns() {
        let mut frame = Frame::new(1, 12);
        frame.feed(b"read\x1b[6Grunning");

        assert_eq!(frame.row(0), "read running");
    }

    #[test]
    fn row_does_not_expand_wide_glyph_continuation_cells() {
        let mut frame = Frame::new(1, 12);
        frame.feed("界 read".as_bytes());

        assert_eq!(frame.row(0), "界 read");
    }
}
