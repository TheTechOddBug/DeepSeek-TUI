//! Expandable activity shelf for concurrent sub-agent banners.
#![allow(dead_code)] // Public shelf API; transcript wiring uses a subset.
//!
//! Visual grammar (v0.9.1):
//! - Collapsed by default: one row `◇ N sub-agents · status · Enter expand`
//! - Expanded: per-agent rows reusing [`super::agent_card`] rendering
//! - Semantic status color only on lifecycle chips; magenta is identity only

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::palette;
use crate::tui::widgets::agent_card::{AgentLifecycle, DelegateCard, FanoutCard};

/// One live agent (or fanout group) projected onto the shelf.
#[derive(Debug, Clone)]
pub enum ShelfAgent {
    Delegate(DelegateCard),
    Fanout(FanoutCard),
}

impl ShelfAgent {
    #[must_use]
    pub fn lifecycle(&self) -> AgentLifecycle {
        match self {
            Self::Delegate(card) => card.status,
            Self::Fanout(card) => card.aggregate_status_public(),
        }
    }

    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Delegate(card) => {
                let role = readable_role(&card.agent_type);
                let short = crate::session_manager::truncate_id(&card.agent_id);
                if role.is_empty() {
                    short.to_string()
                } else {
                    format!("{role} · {short}")
                }
            }
            Self::Fanout(card) => {
                format!("{} ({} workers)", card.kind, card.worker_count())
            }
        }
    }

    #[must_use]
    pub fn render_lines(&self, width: u16) -> Vec<Line<'static>> {
        match self {
            Self::Delegate(card) => card.render_lines(width),
            Self::Fanout(card) => card.render_lines(width),
        }
    }
}

/// Collapsible shelf over multiple concurrent sub-agent cards.
#[derive(Debug, Clone, Default)]
pub struct ActivityShelf {
    pub agents: Vec<ShelfAgent>,
    pub expanded: bool,
}

impl ActivityShelf {
    #[must_use]
    pub fn new(agents: Vec<ShelfAgent>, expanded: bool) -> Self {
        Self { agents, expanded }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Aggregate lifecycle: failed > running > waiting > done > cancelled.
    #[must_use]
    pub fn aggregate_status(&self) -> AgentLifecycle {
        let mut any_running = false;
        let mut any_pending = false;
        let mut any_failed = false;
        let mut any_done = false;
        let mut any_interrupted = false;
        for agent in &self.agents {
            match agent.lifecycle() {
                AgentLifecycle::Failed | AgentLifecycle::Cancelled => any_failed = true,
                AgentLifecycle::Interrupted => any_interrupted = true,
                AgentLifecycle::Running => any_running = true,
                AgentLifecycle::Pending => any_pending = true,
                AgentLifecycle::Completed => any_done = true,
            }
        }
        if any_running {
            AgentLifecycle::Running
        } else if any_pending || any_interrupted {
            AgentLifecycle::Pending
        } else if any_failed && !any_done {
            AgentLifecycle::Failed
        } else if any_done {
            AgentLifecycle::Completed
        } else {
            AgentLifecycle::Pending
        }
    }

    /// Render collapsed summary or expanded per-agent rows.
    ///
    /// When there is only one agent, always expand so the shelf never adds a
    /// useless intermediate hop for single-delegate work.
    #[must_use]
    pub fn render_lines(&self, width: u16) -> Vec<Line<'static>> {
        if self.agents.is_empty() {
            return Vec::new();
        }
        if self.agents.len() == 1 || self.expanded {
            let mut lines = Vec::new();
            if self.agents.len() > 1 {
                lines.push(self.collapsed_header(width, true));
            }
            for agent in &self.agents {
                lines.extend(agent.render_lines(width));
            }
            return lines;
        }
        vec![self.collapsed_header(width, false)]
    }

    fn collapsed_header(&self, width: u16, expanded: bool) -> Line<'static> {
        let n = self.agents.len();
        let status = self.aggregate_status();
        let marker = if expanded { "◆" } else { "◇" };
        let noun = if n == 1 { "sub-agent" } else { "sub-agents" };
        let hint = if expanded {
            "Enter collapse"
        } else {
            "Enter expand"
        };
        let status_label = status.label();
        let status_color = status.color();
        let identity = AgentLifecycle::identity_color();

        // Count running / waiting for a quiet secondary chip.
        let (running, waiting, done, failed) = self.counts();
        let mut parts = vec![format!("{marker} {n} {noun}")];
        if running > 0 {
            parts.push(format!("{running} running"));
        }
        if waiting > 0 {
            parts.push(format!("{waiting} waiting"));
        }
        if done > 0 {
            parts.push(format!("{done} done"));
        }
        if failed > 0 {
            parts.push(format!("{failed} failed"));
        }
        let body = parts.join(" · ");
        let tail = format!(" · [{status_label}] · {hint}");
        let budget = usize::from(width).max(8);
        let body = truncate_width(
            &body,
            budget.saturating_sub(UnicodeWidthStr::width(tail.as_str())),
        );

        Line::from(vec![
            Span::styled(
                body,
                Style::default().fg(identity).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" · [{status_label}]"),
                Style::default().fg(status_color),
            ),
            Span::styled(
                format!(" · {hint}"),
                Style::default().fg(palette::TEXT_HINT),
            ),
        ])
    }

    fn counts(&self) -> (usize, usize, usize, usize) {
        let mut running = 0usize;
        let mut waiting = 0usize;
        let mut done = 0usize;
        let mut failed = 0usize;
        for agent in &self.agents {
            match agent.lifecycle() {
                AgentLifecycle::Running => running += 1,
                AgentLifecycle::Pending | AgentLifecycle::Interrupted => waiting += 1,
                AgentLifecycle::Completed => done += 1,
                AgentLifecycle::Failed | AgentLifecycle::Cancelled => failed += 1,
            }
        }
        (running, waiting, done, failed)
    }
}

fn readable_role(agent_type: &str) -> String {
    match agent_type.to_ascii_lowercase().as_str() {
        "general" => "worker".to_string(),
        "explore" => "scout".to_string(),
        "plan" => "planner".to_string(),
        "review" => "reviewer".to_string(),
        "implementer" => "builder".to_string(),
        "verifier" => "verifier".to_string(),
        "custom" => "specialist".to_string(),
        "delegate" => String::new(),
        other => other.to_string(),
    }
}

fn truncate_width(text: &str, max: usize) -> String {
    if UnicodeWidthStr::width(text) <= max {
        return text.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    let mut out = String::new();
    let mut w = 0usize;
    let limit = max.saturating_sub(1);
    for ch in text.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > limit {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}

/// Collect live sub-agent cards from history indices for shelf rendering.
#[must_use]
pub fn shelf_from_history_cells(
    cells: impl IntoIterator<Item = ShelfAgent>,
    expanded: bool,
) -> ActivityShelf {
    ActivityShelf::new(cells.into_iter().collect(), expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_shelf_is_one_row_for_multiple_agents() {
        let agents = vec![
            ShelfAgent::Delegate({
                let mut c = DelegateCard::new("a1", "explore");
                c.status = AgentLifecycle::Running;
                c
            }),
            ShelfAgent::Delegate({
                let mut c = DelegateCard::new("a2", "plan");
                c.status = AgentLifecycle::Pending;
                c
            }),
        ];
        let shelf = ActivityShelf::new(agents, false);
        let lines = shelf.render_lines(80);
        assert_eq!(lines.len(), 1, "collapsed shelf is a single row");
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("2 sub-agents"), "{text}");
        assert!(text.contains("Enter expand"), "{text}");
        assert!(
            text.contains("running") || text.contains("[running]"),
            "{text}"
        );
    }

    #[test]
    fn expanded_shelf_renders_per_agent_rows() {
        let agents = vec![
            ShelfAgent::Delegate(DelegateCard::new("a1", "explore")),
            ShelfAgent::Delegate(DelegateCard::new("a2", "plan")),
        ];
        let shelf = ActivityShelf::new(agents, true);
        let lines = shelf.render_lines(80);
        assert!(lines.len() > 2, "expanded includes header + agent rows");
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Enter collapse"), "{joined}");
        assert!(
            joined.contains("scout") || joined.contains("explore"),
            "{joined}"
        );
    }

    #[test]
    fn single_agent_skips_collapse_hop() {
        let agents = vec![ShelfAgent::Delegate({
            let mut c = DelegateCard::new("solo", "general");
            c.status = AgentLifecycle::Running;
            c
        })];
        let shelf = ActivityShelf::new(agents, false);
        let lines = shelf.render_lines(80);
        assert!(!lines.is_empty());
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect::<Vec<_>>()
            .join("");
        assert!(
            !text.contains("Enter expand"),
            "single agent should not require expand: {text}"
        );
    }

    #[test]
    fn aggregate_prefers_running_over_waiting() {
        let agents = vec![
            ShelfAgent::Delegate({
                let mut c = DelegateCard::new("a1", "explore");
                c.status = AgentLifecycle::Pending;
                c
            }),
            ShelfAgent::Delegate({
                let mut c = DelegateCard::new("a2", "plan");
                c.status = AgentLifecycle::Running;
                c
            }),
        ];
        let shelf = ActivityShelf::new(agents, false);
        assert_eq!(shelf.aggregate_status(), AgentLifecycle::Running);
    }

    #[test]
    fn status_colors_are_semantic_not_identity() {
        assert_eq!(AgentLifecycle::Running.color(), palette::WHALE_LIVE);
        assert_eq!(AgentLifecycle::Pending.color(), palette::STATUS_WARNING);
        assert_eq!(AgentLifecycle::Completed.color(), palette::STATUS_SUCCESS);
        assert_eq!(AgentLifecycle::Failed.color(), palette::STATUS_ERROR);
        assert_eq!(AgentLifecycle::identity_color(), palette::MODE_OPERATE);
        assert_ne!(
            AgentLifecycle::Pending.color(),
            AgentLifecycle::identity_color(),
            "waiting must not paint magenta identity"
        );
    }
}
