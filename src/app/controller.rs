use crate::app::history::History;
use crate::app::state::State;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn poll_next_event<'a>(state: State<'a>, history: &'a History) -> Result<Option<State<'a>>> {
    match event::read()? {
        Event::Key(event)
            if event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat =>
        {
            match event {
                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }
                | KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }
                | KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                } => Ok(None),
                KeyEvent {
                    code: KeyCode::Left,
                    ..
                } => Ok(Some(state.backward_commit(history))),
                KeyEvent {
                    code: KeyCode::Right,
                    ..
                } => Ok(Some(state.forward_commit(history))),
                KeyEvent {
                    code: KeyCode::Up, ..
                } => Ok(Some(state.scroll_line_up())),
                KeyEvent {
                    code: KeyCode::Down,
                    ..
                } => Ok(Some(state.scroll_line_down())),
                KeyEvent {
                    code: KeyCode::PageUp,
                    ..
                } => Ok(Some(state.scroll_page_up())),
                KeyEvent {
                    code: KeyCode::PageDown,
                    ..
                } => Ok(Some(state.scroll_page_down())),
                KeyEvent {
                    code: KeyCode::Home,
                    ..
                } => Ok(Some(state.scroll_to_top())),
                KeyEvent {
                    code: KeyCode::End, ..
                } => Ok(Some(state.scroll_to_bottom())),
                _ => Ok(Some(state)),
            }
        }
        Event::Resize(_width, height) => {
            Ok(Some(state.update_terminal_height(usize::from(height))))
        }
        _ => Ok(Some(state)),
    }
}
