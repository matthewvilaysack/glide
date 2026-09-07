//! Key mapping. Pure, so the whole keyboard contract is covered by tests.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Action, Mode};

pub fn action_for(key: KeyEvent, mode: Mode) -> Option<Action> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    if ctrl {
        return match key.code {
            KeyCode::Char('c') => Some(Action::Quit),
            KeyCode::Char('k') => Some(Action::EnterBlockMode),
            KeyCode::Char('u') => Some(Action::ScrollUp),
            KeyCode::Char('d') => Some(Action::ScrollDown),
            _ => None,
        };
    }

    match mode {
        Mode::Prompt => match key.code {
            KeyCode::Enter => Some(Action::Submit),
            KeyCode::Esc => Some(Action::EnterBlockMode),
            KeyCode::Backspace => Some(Action::Backspace),
            KeyCode::Delete => Some(Action::Delete),
            KeyCode::Left => Some(Action::Left),
            KeyCode::Right => Some(Action::Right),
            KeyCode::Up => Some(Action::EnterBlockMode),
            KeyCode::Home => Some(Action::Home),
            KeyCode::End => Some(Action::End),
            KeyCode::Char(c) => Some(Action::Insert(c)),
            _ => None,
        },
        Mode::Block => match key.code {
            KeyCode::Esc | KeyCode::Char('i') => Some(Action::ExitBlockMode),
            KeyCode::Char('j') | KeyCode::Down => Some(Action::FocusNext),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::FocusPrev),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Action::ToggleCollapse),
            KeyCode::Char('y') => Some(Action::Copy),
            KeyCode::Char('r') => Some(Action::Rerun),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }
    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }
    fn code(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    #[test]
    fn letters_type_in_prompt_mode_and_navigate_in_block_mode() {
        assert_eq!(
            action_for(key('j'), Mode::Prompt),
            Some(Action::Insert('j'))
        );
        assert_eq!(action_for(key('j'), Mode::Block), Some(Action::FocusNext));
    }

    #[test]
    fn ctrl_keys_work_in_both_modes() {
        for mode in [Mode::Prompt, Mode::Block] {
            assert_eq!(action_for(ctrl('c'), mode), Some(Action::Quit));
            assert_eq!(action_for(ctrl('k'), mode), Some(Action::EnterBlockMode));
            assert_eq!(action_for(ctrl('u'), mode), Some(Action::ScrollUp));
            assert_eq!(action_for(ctrl('d'), mode), Some(Action::ScrollDown));
        }
    }

    #[test]
    fn enter_submits_at_the_prompt_and_folds_in_block_mode() {
        assert_eq!(
            action_for(code(KeyCode::Enter), Mode::Prompt),
            Some(Action::Submit)
        );
        assert_eq!(
            action_for(code(KeyCode::Enter), Mode::Block),
            Some(Action::ToggleCollapse)
        );
    }

    #[test]
    fn esc_moves_between_the_two_modes() {
        assert_eq!(
            action_for(code(KeyCode::Esc), Mode::Prompt),
            Some(Action::EnterBlockMode)
        );
        assert_eq!(
            action_for(code(KeyCode::Esc), Mode::Block),
            Some(Action::ExitBlockMode)
        );
    }

    #[test]
    fn unmapped_keys_are_ignored_rather_than_guessed_at() {
        assert_eq!(action_for(code(KeyCode::F(5)), Mode::Prompt), None);
        assert_eq!(action_for(key('z'), Mode::Block), None);
    }
}
