//! Console state and the one function that changes it.
//!
//! `App::update` is pure apart from the `Effect` it hands back, so the whole
//! interaction model is testable without a terminal: build an `App`, feed it
//! actions, assert on the blocks.

use crate::block::{Block, BlockBody};
use crate::engine::{help_body, welcome_body};

/// Prompt mode types; block mode navigates. Same split Warp uses to let one
/// key mean two things without a modifier soup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Prompt,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Insert(char),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Submit,
    EnterBlockMode,
    ExitBlockMode,
    FocusPrev,
    FocusNext,
    ToggleCollapse,
    Copy,
    Rerun,
    ScrollUp,
    ScrollDown,
    Quit,
}

/// Side effects `update` cannot perform itself. The runner does them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Copy(String),
    /// A block is on screen showing "running". Execute this and hand the
    /// result back through `finish_submit`.
    Run(String),
}

pub struct App {
    pub blocks: Vec<Block>,
    pub input: String,
    /// Caret position in characters, not bytes.
    pub cursor: usize,
    pub mode: Mode,
    pub focus: Option<usize>,
    pub scroll: u16,
    /// While true the view pins to the newest block, like a terminal.
    pub follow: bool,
    pub should_quit: bool,
    pub repo_label: String,
    pub stats_label: String,
    pub status_msg: Option<String>,
    /// Index of the block currently showing "running", if any.
    pending: Option<usize>,
    next_id: usize,
}

impl App {
    pub fn new(
        repo_label: impl Into<String>,
        stats_label: impl Into<String>,
        has_graph: bool,
    ) -> Self {
        let mut app = App {
            blocks: Vec::new(),
            input: String::new(),
            cursor: 0,
            mode: Mode::Prompt,
            focus: None,
            scroll: 0,
            follow: true,
            should_quit: false,
            repo_label: repo_label.into(),
            stats_label: stats_label.into(),
            status_msg: None,
            pending: None,
            next_id: 0,
        };
        app.push_block("", welcome_body(has_graph), 0);
        app
    }

    fn push_block(&mut self, input: impl Into<String>, body: BlockBody, elapsed_ms: u128) {
        let id = self.next_id;
        self.next_id += 1;
        self.blocks.push(Block::new(id, input, body, elapsed_ms));
        self.follow = true;
    }

    pub fn focused(&self) -> Option<&Block> {
        self.focus.and_then(|i| self.blocks.get(i))
    }

    /// Built-ins the engine never sees. Returns true if the line was handled.
    fn handle_builtin(&mut self, line: &str) -> bool {
        match line {
            "help" | "?" => {
                self.push_block(line, help_body(), 0);
                true
            }
            "clear" => {
                self.blocks.clear();
                self.focus = None;
                self.pending = None;
                true
            }
            "exit" | "quit" => {
                self.should_quit = true;
                true
            }
            _ => false,
        }
    }

    /// Phase one of a submit: put a running block on screen and ask the runner
    /// to execute. Verbs like `index build` take seconds, and a frozen screen
    /// with no block reads as a hang.
    fn submit(&mut self, line: String) -> Option<Effect> {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() || self.handle_builtin(&trimmed) {
            return None;
        }
        self.push_block(trimmed.clone(), BlockBody::Pending, 0);
        self.pending = Some(self.blocks.len() - 1);
        Some(Effect::Run(trimmed))
    }

    /// Phase two: swap the running block's body for the real answer.
    pub fn finish_submit(&mut self, body: BlockBody, elapsed_ms: u128) {
        if let Some(block) = self.pending.take().and_then(|i| self.blocks.get_mut(i)) {
            block.body = body;
            block.elapsed_ms = elapsed_ms;
        }
        self.follow = true;
    }

    pub fn update(&mut self, action: Action) -> Option<Effect> {
        self.status_msg = None;
        match action {
            Action::Quit => self.should_quit = true,

            Action::Insert(c) => {
                let byte = self.byte_at(self.cursor);
                self.input.insert(byte, c);
                self.cursor += 1;
            }
            Action::Backspace => {
                if self.cursor > 0 {
                    let start = self.byte_at(self.cursor - 1);
                    let end = self.byte_at(self.cursor);
                    self.input.replace_range(start..end, "");
                    self.cursor -= 1;
                }
            }
            Action::Delete => {
                if self.cursor < self.input.chars().count() {
                    let start = self.byte_at(self.cursor);
                    let end = self.byte_at(self.cursor + 1);
                    self.input.replace_range(start..end, "");
                }
            }
            Action::Left => self.cursor = self.cursor.saturating_sub(1),
            Action::Right => self.cursor = (self.cursor + 1).min(self.input.chars().count()),
            Action::Home => self.cursor = 0,
            Action::End => self.cursor = self.input.chars().count(),

            Action::Submit => {
                let line = std::mem::take(&mut self.input);
                self.cursor = 0;
                return self.submit(line);
            }

            Action::EnterBlockMode => {
                if !self.blocks.is_empty() {
                    self.mode = Mode::Block;
                    self.focus = Some(self.focus.unwrap_or(self.blocks.len() - 1));
                }
            }
            Action::ExitBlockMode => {
                self.mode = Mode::Prompt;
                self.focus = None;
            }
            Action::FocusPrev => {
                if let Some(i) = self.focus {
                    self.focus = Some(i.saturating_sub(1));
                    self.follow = false;
                }
            }
            Action::FocusNext => {
                if let Some(i) = self.focus {
                    self.focus = Some((i + 1).min(self.blocks.len().saturating_sub(1)));
                }
            }
            Action::ToggleCollapse => {
                if let Some(b) = self.focus.and_then(|i| self.blocks.get_mut(i)) {
                    b.collapsed = !b.collapsed;
                }
            }
            Action::Copy => {
                if let Some(b) = self.focused() {
                    let text = crate::render::block_plain_text(b);
                    self.status_msg = Some(format!("copied block {}", b.id));
                    return Some(Effect::Copy(text));
                }
            }
            Action::Rerun => {
                if let Some(input) = self.focused().map(|b| b.input.clone()) {
                    if input.is_empty() {
                        self.status_msg = Some("nothing to re-run".into());
                    } else {
                        self.mode = Mode::Prompt;
                        self.focus = None;
                        return self.submit(input);
                    }
                }
            }

            Action::ScrollUp => {
                self.scroll = self.scroll.saturating_sub(3);
                self.follow = false;
            }
            Action::ScrollDown => {
                self.scroll = self.scroll.saturating_add(3);
            }
        }
        None
    }

    fn byte_at(&self, char_idx: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.input.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockStatus;

    fn app() -> App {
        App::new("acme", "0 owners", true)
    }

    fn fake_body(input: &str) -> BlockBody {
        BlockBody::Notice {
            title: format!("ran: {input}"),
            lines: vec![],
        }
    }

    /// Drive both phases the way the runner does.
    fn run_effect(a: &mut App, effect: Option<Effect>) -> Option<Effect> {
        match effect {
            Some(Effect::Run(input)) => {
                a.finish_submit(fake_body(&input), 1);
                None
            }
            other => other,
        }
    }

    fn type_line(a: &mut App, s: &str) {
        for c in s.chars() {
            a.update(Action::Insert(c));
        }
        let e = a.update(Action::Submit);
        run_effect(a, e);
    }

    #[test]
    fn launches_with_a_welcome_block() {
        let a = app();
        assert_eq!(a.blocks.len(), 1);
        assert_eq!(a.blocks[0].status(), BlockStatus::Notice);
    }

    #[test]
    fn submitting_appends_a_block_and_clears_the_prompt() {
        let mut a = app();
        type_line(&mut a, "who owns billing");
        assert_eq!(a.blocks.len(), 2);
        assert_eq!(a.blocks[1].input, "who owns billing");
        assert!(a.input.is_empty());
        assert_eq!(a.cursor, 0);
    }

    #[test]
    fn blank_submit_does_nothing() {
        let mut a = app();
        let e = a.update(Action::Submit);
        run_effect(&mut a, e);
        assert_eq!(a.blocks.len(), 1);
    }

    #[test]
    fn a_running_block_is_on_screen_before_the_answer_arrives() {
        let mut a = app();
        for c in "index build".chars() {
            a.update(Action::Insert(c));
        }
        let effect = a.update(Action::Submit);

        // Phase one: the block exists and says it is running.
        assert_eq!(effect, Some(Effect::Run("index build".into())));
        assert_eq!(a.blocks.len(), 2);
        assert_eq!(a.blocks[1].status(), BlockStatus::Running);

        // Phase two: same block, now carrying the answer.
        let id = a.blocks[1].id;
        a.finish_submit(fake_body("index build"), 1200);
        assert_eq!(
            a.blocks.len(),
            2,
            "the answer replaced the block, not appended"
        );
        assert_eq!(a.blocks[1].id, id);
        assert_eq!(a.blocks[1].status(), BlockStatus::Notice);
        assert_eq!(a.blocks[1].elapsed_ms, 1200);
    }

    #[test]
    fn builtins_never_reach_the_engine() {
        let mut a = app();
        for c in "help".chars() {
            a.update(Action::Insert(c));
        }
        assert_eq!(a.update(Action::Submit), None);
    }

    #[test]
    fn backspace_respects_multibyte_characters() {
        let mut a = app();
        for c in "héllo".chars() {
            a.update(Action::Insert(c));
        }
        a.update(Action::Backspace);
        assert_eq!(a.input, "héll");
        a.update(Action::Home);
        a.update(Action::Delete);
        assert_eq!(a.input, "éll");
    }

    #[test]
    fn focus_clamps_at_both_ends() {
        let mut a = app();
        type_line(&mut a, "one");
        type_line(&mut a, "two");
        a.update(Action::EnterBlockMode);
        assert_eq!(a.focus, Some(2));
        a.update(Action::FocusNext);
        assert_eq!(a.focus, Some(2));
        for _ in 0..10 {
            a.update(Action::FocusPrev);
        }
        assert_eq!(a.focus, Some(0));
    }

    #[test]
    fn collapse_toggles_the_focused_block_only() {
        let mut a = app();
        type_line(&mut a, "one");
        a.update(Action::EnterBlockMode);
        a.update(Action::ToggleCollapse);
        assert!(a.blocks[1].collapsed);
        assert!(!a.blocks[0].collapsed);
        a.update(Action::ToggleCollapse);
        assert!(!a.blocks[1].collapsed);
    }

    #[test]
    fn copy_returns_the_focused_block_as_text() {
        let mut a = app();
        type_line(&mut a, "who owns billing");
        a.update(Action::EnterBlockMode);
        let effect = a.update(Action::Copy);
        match effect {
            Some(Effect::Copy(text)) => assert!(text.contains("who owns billing")),
            other => panic!("expected a copy effect, got {other:?}"),
        }
    }

    #[test]
    fn rerun_submits_the_focused_input_again() {
        let mut a = app();
        type_line(&mut a, "who owns billing");
        a.update(Action::EnterBlockMode);
        a.update(Action::Rerun);
        assert_eq!(a.blocks.len(), 3);
        assert_eq!(a.blocks[2].input, "who owns billing");
        assert_eq!(a.mode, Mode::Prompt);
    }

    #[test]
    fn rerun_on_the_welcome_block_is_a_no_op() {
        let mut a = app();
        a.update(Action::EnterBlockMode);
        a.update(Action::Rerun);
        assert_eq!(a.blocks.len(), 1);
        assert!(a.status_msg.is_some());
    }

    #[test]
    fn clear_drops_every_block_and_releases_focus() {
        let mut a = app();
        type_line(&mut a, "one");
        a.update(Action::EnterBlockMode);
        type_line(&mut a, "clear");
        assert!(a.blocks.is_empty());
        assert_eq!(a.focus, None);
    }

    #[test]
    fn exit_quits() {
        let mut a = app();
        type_line(&mut a, "exit");
        assert!(a.should_quit);
    }

    #[test]
    fn help_renders_as_a_block_not_a_verb_call() {
        let mut a = app();
        type_line(&mut a, "help");
        assert_eq!(a.blocks.len(), 2);
        match &a.blocks[1].body {
            BlockBody::Notice { title, lines } => {
                assert_eq!(title, "keys and verbs");
                assert!(lines.iter().any(|l| l.contains("block mode")));
                assert!(lines.iter().any(|l| l.contains("doctor")));
            }
            other => panic!("expected a notice, got {other:?}"),
        }
    }

    #[test]
    fn block_mode_needs_a_block_to_focus() {
        let mut a = app();
        type_line(&mut a, "clear");
        a.update(Action::EnterBlockMode);
        assert_eq!(a.mode, Mode::Prompt);
        assert_eq!(a.focus, None);
    }
}
