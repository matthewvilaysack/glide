//! Every renderer is a pure function of data. Nothing here touches a terminal,
//! which is what lets the snapshot tests run on a `TestBackend`.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Mode};
use crate::block::{Block, BlockBody, BlockStatus};
use crate::theme;

/// Width of the `"  │ "` gutter that prefixes every body line.
const GUTTER_WIDTH: u16 = 4;

fn dim(s: impl Into<String>) -> Span<'static> {
    Span::styled(s.into(), Style::default().fg(theme::DIM))
}

fn body(s: impl Into<String>) -> Span<'static> {
    Span::styled(s.into(), Style::default().fg(theme::BODY))
}

fn accent(s: impl Into<String>) -> Span<'static> {
    Span::styled(s.into(), Style::default().fg(theme::SAFFRON))
}

/// What the block's header shows. Notices have no typed input, so they carry
/// their title there instead.
fn header_text(block: &Block) -> String {
    if !block.input.is_empty() {
        return block.input.clone();
    }
    match &block.body {
        BlockBody::Notice { title, .. } => title.clone(),
        other => other.kind().to_string(),
    }
}

fn status_glyph(status: BlockStatus) -> Span<'static> {
    match status {
        BlockStatus::Running => Span::styled("…", Style::default().fg(theme::SAFFRON)),
        BlockStatus::Ok => Span::styled("✓", Style::default().fg(theme::PROMPT)),
        BlockStatus::Failed => Span::styled("✗", Style::default().fg(theme::ERROR)),
        BlockStatus::Notice => Span::styled("·", Style::default().fg(theme::DIM)),
    }
}

pub fn who_owns_lines(r: &glide_core::WhoOwnsResponse) -> Vec<Line<'static>> {
    if r.hits.is_empty() {
        return vec![
            Line::from(dim(format!("no owner in the graph for {}", r.query))),
            Line::from(dim("try a narrower path, or re-run `glide index build`")),
        ];
    }
    let mut out = Vec::new();
    for h in &r.hits {
        out.push(Line::from(vec![
            accent(format!("@{}", h.owner_handle)),
            dim(format!("   {:.2}  via {}", h.weight, h.source)),
        ]));
        if r.include_evidence {
            if let Some(e) = &h.evidence {
                out.push(Line::from(dim(format!("    why  {e}"))));
            }
        }
    }
    out
}

pub fn plan_lines(r: &glide_core::PlanResponse) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for (i, section) in r.sections.iter().enumerate() {
        if i > 0 {
            out.push(Line::from(""));
        }
        out.push(Line::from(Span::styled(
            section.title.clone(),
            Style::default().fg(theme::INK).add_modifier(Modifier::BOLD),
        )));
        for b in &section.bullets {
            out.push(Line::from(vec![dim("· "), body(b.clone())]));
        }
    }
    out.push(Line::from(""));
    out.push(Line::from(dim(r.status.to_string())));
    out
}

pub fn request_lines(r: &glide_core::RequestResponse) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![dim("permission  "), accent(r.permission.clone())]),
        Line::from(""),
        Line::from(body(r.draft_message.clone())),
        Line::from(""),
        Line::from(dim(format!("send to  {}", r.channels.join("  ")))),
        Line::from(dim(r.status.to_string())),
    ]
}

pub fn digest_lines(r: &glide_core::FrictionDigestResponse) -> Vec<Line<'static>> {
    if r.events.is_empty() {
        return vec![Line::from(dim(format!(
            "no friction logged since {}",
            r.since
        )))];
    }
    let mut out = vec![Line::from(dim(format!(
        "{} event{} since {}",
        r.events.len(),
        if r.events.len() == 1 { "" } else { "s" },
        r.since
    )))];
    for e in &r.events {
        out.push(Line::from(vec![
            accent(format!("sev{}", e.severity)),
            dim(format!("  {:<18}", e.category)),
            body(e.subject.clone()),
        ]));
    }
    if !r.by_category.is_empty() {
        let summary = r
            .by_category
            .iter()
            .map(|(c, n)| format!("{c} {n}"))
            .collect::<Vec<_>>()
            .join("   ");
        out.push(Line::from(""));
        out.push(Line::from(dim(summary)));
    }
    out
}

pub fn doctor_lines(r: &glide_core::DoctorReport) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = r
        .checks
        .iter()
        .map(|c| {
            let glyph = match c.state {
                glide_core::CheckState::Pass => {
                    Span::styled("✓", Style::default().fg(theme::PROMPT))
                }
                glide_core::CheckState::Fail => {
                    Span::styled("✗", Style::default().fg(theme::ERROR))
                }
            };
            Line::from(vec![
                glyph,
                Span::raw(" "),
                accent(format!("{:<10}", c.name)),
                body(c.detail.clone()),
            ])
        })
        .collect();

    let failures = r.failures();
    out.push(Line::from(""));
    out.push(Line::from(if failures == 0 {
        Span::styled("all checks pass", Style::default().fg(theme::PROMPT))
    } else {
        Span::styled(
            format!("{failures} check(s) failed"),
            Style::default().fg(theme::ERROR),
        )
    }));
    out
}

pub fn stats_lines(s: &glide_graph::GraphStats) -> Vec<Line<'static>> {
    let row =
        |label: &str, value: String| Line::from(vec![dim(format!("{label:<16}")), body(value)]);
    vec![
        row("people", s.people.to_string()),
        row("teams", s.teams.to_string()),
        row("paths", s.paths.to_string()),
        row("ownership rows", s.ownership_rows.to_string()),
        row("friction events", s.friction_events.to_string()),
        row(
            "last indexed",
            match (s.last_index_at.as_deref(), s.last_index_ok) {
                (Some(t), Some(true)) => t.to_string(),
                (Some(t), Some(false)) => format!("{t} (FAILED)"),
                _ => "never — run `index build`".to_string(),
            },
        ),
    ]
}

pub fn indexed_lines(r: &glide_graph::index::IndexReport) -> Vec<Line<'static>> {
    let row =
        |label: &str, value: String| Line::from(vec![dim(format!("{label:<18}")), body(value)]);
    let mut out = vec![
        row("codeowners rules", r.codeowners_rules.to_string()),
        row("git buckets", r.git_history_buckets.to_string()),
        row("team-doc mentions", r.team_doc_mentions.to_string()),
        row(
            "scanned",
            format!(
                "{} commits across {} files",
                r.commits_scanned, r.files_scanned
            ),
        ),
    ];
    out.push(if r.repomix.ran {
        row(
            "repomix",
            format!(
                "{:.1} KB via {}",
                r.repomix.bytes as f64 / 1024.0,
                r.repomix.mode
            ),
        )
    } else {
        row(
            "repomix",
            format!(
                "skipped — {}",
                r.repomix
                    .skipped_reason
                    .as_deref()
                    .unwrap_or("no reason given")
            ),
        )
    });
    out
}

pub fn models_lines(r: &glide_core::ModelsReport) -> Vec<Line<'static>> {
    let mut out = vec![Line::from(vec![
        dim("llm  "),
        body(if r.enabled { "enabled" } else { "disabled" }),
        dim(format!("   default {}", r.default_provider)),
    ])];
    for p in &r.providers {
        out.push(Line::from(vec![
            accent(p.id.clone()),
            dim(format!("  {}", p.model)),
        ]));
        out.push(Line::from(vec![
            dim(format!("    key {} ", p.api_key_env)),
            if p.key_present {
                Span::styled("set", Style::default().fg(theme::PROMPT))
            } else {
                Span::styled("missing", Style::default().fg(theme::ERROR))
            },
            if p.feature_compiled {
                dim("")
            } else {
                dim("   [feature not compiled]")
            },
        ]));
    }
    out
}

pub fn tools_lines(s: &glide_common::tools::RepomixStatus) -> Vec<Line<'static>> {
    use glide_common::tools::RepomixCmd;
    if !s.available {
        return vec![
            Line::from(vec![
                Span::styled("✗ ", Style::default().fg(theme::ERROR)),
                body("repomix not available"),
            ]),
            Line::from(dim("quit and run `glide tools install repomix`")),
        ];
    }
    let mode = match s.cmd {
        Some(RepomixCmd::Global) => "global",
        Some(RepomixCmd::Npx) => "npx",
        None => "?",
    };
    let mut out = vec![Line::from(vec![
        Span::styled("✓ ", Style::default().fg(theme::PROMPT)),
        accent("repomix"),
        dim(format!(
            "  {}  ({mode})",
            s.version.as_deref().unwrap_or("version unknown")
        )),
    ])];
    if let Some(p) = &s.bin_path {
        out.push(Line::from(dim(format!("    {p}"))));
    }
    out
}

pub fn friction_logged_lines(r: &glide_core::FrictionLogResponse) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            accent(format!("#{}", r.id)),
            dim(format!("  {}  sev{}", r.category, r.severity)),
        ]),
        Line::from(body(r.subject.clone())),
        Line::from(dim(r.occurred_at.clone())),
    ]
}

pub fn body_lines(b: &BlockBody) -> Vec<Line<'static>> {
    match b {
        BlockBody::Pending => vec![Line::from(dim("running…"))],
        BlockBody::Doctor(r) => doctor_lines(r),
        BlockBody::Stats(s) => stats_lines(s),
        BlockBody::Indexed(r) => indexed_lines(r),
        BlockBody::Models(r) => models_lines(r),
        BlockBody::Tools(s) => tools_lines(s),
        BlockBody::FrictionLogged(r) => friction_logged_lines(r),
        BlockBody::Toml(text) => text
            .lines()
            .map(|l| {
                if l.starts_with('[') {
                    Line::from(accent(l.to_string()))
                } else {
                    Line::from(body(l.to_string()))
                }
            })
            .collect(),
        BlockBody::WhoOwns(r) => who_owns_lines(r),
        BlockBody::Plan(r) => plan_lines(r),
        BlockBody::Request(r) => request_lines(r),
        BlockBody::Digest(r) => digest_lines(r),
        BlockBody::Notice { lines, .. } => {
            lines.iter().map(|l| Line::from(body(l.clone()))).collect()
        }
        BlockBody::Error { message } => vec![Line::from(Span::styled(
            message.clone(),
            Style::default().fg(theme::ERROR),
        ))],
        BlockBody::Unrouted {
            reason,
            suggestions,
        } => {
            let mut out = vec![Line::from(dim(reason.clone())), Line::from("")];
            out.push(Line::from(dim("try")));
            for s in suggestions {
                out.push(Line::from(Span::styled(
                    format!("  {s}"),
                    Style::default().fg(theme::PROMPT),
                )));
            }
            out
        }
    }
}

/// A notice's title becomes the block header only when there was no typed
/// input. Otherwise the header is the typed line and the title has to move into
/// the body, or it vanishes — which is how a `--safe` refusal lost the sentence
/// explaining why it was refused. Skipped when the input already says it, as
/// with `config get <key>`.
fn body_lines_in_context(block: &Block) -> Vec<Line<'static>> {
    let lines = body_lines(&block.body);
    match &block.body {
        BlockBody::Notice { title, .. }
            if !block.input.is_empty() && !block.input.contains(title.as_str()) =>
        {
            let mut out = vec![Line::from(Span::styled(
                title.clone(),
                Style::default().fg(theme::INK).add_modifier(Modifier::BOLD),
            ))];
            out.extend(lines);
            out
        }
        _ => lines,
    }
}

/// Re-flow one line to `width`, keeping every span's style and re-indenting
/// continuations. Done here rather than by `Paragraph`'s own wrap so that the
/// block gutter can be prepended to each wrapped line, and so the scroll math
/// can count rendered lines exactly.
pub fn wrap_line(line: Line<'static>, width: usize) -> Vec<Line<'static>> {
    let total: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
    if width == 0 || total <= width {
        return vec![line];
    }

    let indent: String = line
        .spans
        .first()
        .map(|s| s.content.chars().take_while(|c| *c == ' ').collect())
        .unwrap_or_default();
    let indent = if indent.chars().count() >= width / 2 {
        String::new()
    } else {
        indent
    };

    // A break point falls on a space, so the finished line would otherwise end
    // in one and overshoot the width by a column.
    fn flush(spans: &mut Vec<Span<'static>>, out: &mut Vec<Line<'static>>) {
        if let Some(last) = spans.last_mut() {
            let trimmed = last.content.trim_end_matches(' ').to_string();
            *last = Span::styled(trimmed, last.style);
        }
        out.push(Line::from(std::mem::take(spans)));
    }

    let mut out: Vec<Line<'static>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;

    for span in line.spans {
        for chunk in span.content.split_inclusive(' ') {
            let visible = chunk.trim_end_matches(' ').chars().count();
            if used + visible > width && !current.is_empty() {
                flush(&mut current, &mut out);
                used = indent.chars().count();
                if !indent.is_empty() {
                    current.push(Span::raw(indent.clone()));
                }
            }
            used += chunk.chars().count();
            current.push(Span::styled(chunk.to_string(), span.style));
        }
    }
    if !current.is_empty() {
        flush(&mut current, &mut out);
    }
    out
}

/// One block, gutter and all. `focused` swaps the gutter for the saffron bar.
/// `width` is the room available for the body, gutter included.
pub fn block_lines(block: &Block, focused: bool, width: u16) -> Vec<Line<'static>> {
    let caret = if block.collapsed { "▸" } else { "▾" };
    let caret_style = if focused {
        Style::default().fg(theme::SAFFRON)
    } else {
        Style::default().fg(theme::DIM)
    };
    let header_style = if focused {
        Style::default().fg(theme::INK).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::BODY)
    };

    let mut meta = format!("  {}", block.body.kind());
    if block.elapsed_ms > 0 {
        meta.push_str(&format!("  {}ms", block.elapsed_ms));
    }

    let mut out = vec![Line::from(vec![
        Span::styled(format!("{caret} "), caret_style),
        Span::styled(header_text(block), header_style),
        dim(meta),
        Span::raw("  "),
        status_glyph(block.status()),
    ])];

    if !block.collapsed {
        let gutter = if focused {
            Span::styled("  ┃ ", Style::default().fg(theme::SAFFRON))
        } else {
            Span::styled("  │ ", Style::default().fg(theme::DIM))
        };
        let body_width = width.saturating_sub(GUTTER_WIDTH) as usize;
        for line in body_lines_in_context(block) {
            for wrapped in wrap_line(line, body_width) {
                let mut spans = vec![gutter.clone()];
                spans.extend(wrapped.spans);
                out.push(Line::from(spans));
            }
        }
    }
    out.push(Line::from(""));
    out
}

/// The block as plain text, for the clipboard.
pub fn block_plain_text(block: &Block) -> String {
    let mut lines = vec![header_text(block)];
    for l in body_lines_in_context(block) {
        lines.push(
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
                .trim_end()
                .to_string(),
        );
    }
    lines.join("\n")
}

fn all_lines(app: &App, width: u16) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for (i, b) in app.blocks.iter().enumerate() {
        out.extend(block_lines(b, app.focus == Some(i), width));
    }
    out
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(theme::SHELL)),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    draw_top_bar(f, app, chunks[0]);
    draw_blocks(f, app, chunks[1]);
    draw_prompt(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);
}

fn draw_top_bar(f: &mut Frame, app: &App, area: Rect) {
    let line = Line::from(vec![
        accent(" ◜ glide "),
        dim(" · "),
        body(app.repo_label.clone()),
        dim(" · "),
        dim(app.stats_label.clone()),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BAR)),
        area,
    );
}

fn draw_blocks(f: &mut Frame, app: &mut App, area: Rect) {
    // Lines are pre-wrapped to the area width, so the count below is exactly
    // how many rows will be drawn and the scroll offset is never a guess.
    let lines = all_lines(app, area.width);
    let max_scroll = (lines.len() as u16).saturating_sub(area.height);

    if app.follow {
        app.scroll = max_scroll;
    } else {
        app.scroll = app.scroll.min(max_scroll);
    }

    f.render_widget(Paragraph::new(lines).scroll((app.scroll, 0)), area);
}

fn draw_prompt(f: &mut Frame, app: &App, area: Rect) {
    let caret = Span::styled(" ❯ ", Style::default().fg(theme::SAFFRON));
    let text = Span::styled(app.input.clone(), Style::default().fg(theme::BODY));
    f.render_widget(Paragraph::new(Line::from(vec![caret, text])), area);

    if app.mode == Mode::Prompt {
        let x = area.x + 3 + app.input.chars().take(app.cursor).count() as u16;
        f.set_cursor_position((x.min(area.right().saturating_sub(1)), area.y));
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let keys = match app.mode {
        Mode::Prompt => " enter run   ^k blocks   ^u ^d scroll   ^c quit ",
        Mode::Block => " j k focus   enter fold   y copy   r re-run   esc prompt ",
    };
    let mut spans = vec![match app.mode {
        Mode::Prompt => Span::styled(
            " prompt ",
            Style::default().bg(theme::PROMPT).fg(theme::SHELL),
        ),
        Mode::Block => Span::styled(
            " blocks ",
            Style::default().bg(theme::SAFFRON).fg(theme::SHELL),
        ),
    }];
    spans.push(dim(keys));
    if let Some(msg) = &app.status_msg {
        spans.push(accent(format!("  {msg}")));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::BAR)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;
    use glide_graph::{OwnerHit, OwnerKind};

    fn hit(handle: &str, weight: f64) -> OwnerHit {
        OwnerHit {
            owner_handle: handle.into(),
            owner_kind: OwnerKind::Person,
            weight,
            source: "codeowners".into(),
            evidence: Some(".github/CODEOWNERS:12".into()),
            matched_pattern: "billing/*".into(),
        }
    }

    fn who_owns_block() -> Block {
        Block::new(
            0,
            "who owns billing/charge.py",
            BlockBody::WhoOwns(Box::new(glide_core::WhoOwnsResponse {
                query: "billing/charge.py".into(),
                hits: vec![hit("priya", 0.62), hit("marvin", 0.31)],
                include_evidence: true,
            })),
            3,
        )
    }

    #[test]
    fn a_who_owns_block_shows_every_owner_with_its_evidence() {
        let text = block_plain_text(&who_owns_block());
        assert!(text.contains("who owns billing/charge.py"));
        assert!(text.contains("@priya"));
        assert!(text.contains("@marvin"));
        assert!(text.contains("why  .github/CODEOWNERS:12"));
    }

    #[test]
    fn a_collapsed_block_renders_its_header_and_nothing_else() {
        let mut b = who_owns_block();
        b.collapsed = true;
        let lines = block_lines(&b, false, 80);
        // header + the trailing spacer
        assert_eq!(lines.len(), 2);
        let header: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(header.starts_with("▸ "));
        assert!(header.contains("who-owns"));
        assert!(!header.contains("@priya"));
    }

    #[test]
    fn the_focused_block_gets_the_saffron_gutter() {
        let b = who_owns_block();
        let unfocused = block_lines(&b, false, 80);
        let focused = block_lines(&b, true, 80);
        assert_eq!(unfocused[1].spans[0].content.as_ref(), "  │ ");
        assert_eq!(focused[1].spans[0].content.as_ref(), "  ┃ ");
        assert_eq!(focused[1].spans[0].style.fg, Some(theme::SAFFRON));
    }

    #[test]
    fn an_empty_result_says_so_instead_of_rendering_nothing() {
        let r = glide_core::WhoOwnsResponse {
            query: "nowhere.rs".into(),
            hits: vec![],
            include_evidence: true,
        };
        let text: String = who_owns_lines(&r)
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("no owner in the graph for nowhere.rs"));
        assert!(text.contains("glide index build"));
    }

    #[test]
    fn every_wrapped_body_line_keeps_its_gutter() {
        let long = "Hi team — could we get pomelo access for the new hire? \
                    Blocking ramp; happy to provide context.";
        let block = Block::new(
            0,
            "i need access to pomelo",
            BlockBody::Request(Box::new(glide_core::RequestResponse {
                permission: "pomelo".into(),
                for_person: None,
                draft_message: long.into(),
                channels: vec!["slack:#access-requests".into()],
                status: "draft",
            })),
            1,
        );
        let lines = block_lines(&block, false, 48);
        assert!(lines.len() > 4, "the long draft should have wrapped");
        for line in lines.iter().skip(1) {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                text.is_empty() || text.starts_with("  │ "),
                "line lost its gutter: {text:?}"
            );
            assert!(
                text.chars().count() <= 48,
                "line ran past the width: {text:?}"
            );
        }
    }

    #[test]
    fn wrapping_preserves_the_style_of_every_span() {
        let line = Line::from(vec![
            accent("@someone-with-a-long-handle"),
            dim("   0.97  via git-history and a very long trailing explanation"),
        ]);
        let wrapped = wrap_line(line, 30);
        assert!(wrapped.len() > 1);
        assert_eq!(wrapped[0].spans[0].style.fg, Some(theme::SAFFRON));
        let dim_spans = wrapped
            .iter()
            .flat_map(|l| l.spans.iter())
            .filter(|s| s.style.fg == Some(theme::DIM))
            .count();
        assert!(dim_spans > 0, "the dim half of the line lost its style");
    }

    #[test]
    fn a_notice_answering_a_typed_line_keeps_its_title_visible() {
        // The header shows what was typed, so a refusal's reason has to move
        // into the body or the block only says "restart without --safe".
        let block = Block::new(
            0,
            "friction log blocked on staging",
            BlockBody::Notice {
                title: "friction log needs writes, and this session is --safe".into(),
                lines: vec!["Restart without `--safe` to allow it.".into()],
            },
            0,
        );
        let text = block_plain_text(&block);
        assert!(text.contains("friction log blocked on staging"));
        assert!(text.contains("needs writes"));
    }

    #[test]
    fn a_notice_the_header_already_states_is_not_repeated() {
        let block = Block::new(
            0,
            "config get llm.anthropic.model",
            BlockBody::Notice {
                title: "llm.anthropic.model".into(),
                lines: vec!["claude-haiku-4-5".into()],
            },
            0,
        );
        let body_line_count = block_lines(&block, false, 80).len();
        // header + one value line + trailing spacer, with no repeated key
        assert_eq!(body_line_count, 3);
    }

    #[test]
    fn a_short_line_is_returned_untouched() {
        let line = Line::from(body("short"));
        assert_eq!(wrap_line(line, 40).len(), 1);
    }

    #[test]
    fn the_console_draws_a_full_frame_without_panicking() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut app = App::new("acme-monorepo", "12 owners", true);
        for c in "who owns billing".chars() {
            app.update(crate::app::Action::Insert(c));
        }
        app.update(crate::app::Action::Submit);
        app.finish_submit(
            BlockBody::Notice {
                title: "t".into(),
                lines: vec!["l".into()],
            },
            3,
        );

        let mut term = Terminal::new(TestBackend::new(64, 20)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();

        let rendered: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(rendered.contains("glide"));
        assert!(rendered.contains("acme-monorepo"));
        assert!(rendered.contains("prompt"));
    }
}
