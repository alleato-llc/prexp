use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use prexp_core::source::ProcessSource;

use super::app::{App, InputMode, KillState, MainView};

/// Poll for a crossterm event with the given timeout.
pub fn poll_event(timeout: Duration) -> Option<Event> {
    if event::poll(timeout).ok()? {
        event::read().ok()
    } else {
        None
    }
}

/// Handle a key event and update app state.
pub fn handle_key(app: &mut App, key: KeyEvent, source: &dyn ProcessSource) {
    match app.input_mode {
        InputMode::Normal => {
            if app.kill_state.is_some() {
                handle_kill_key(app, key);
            } else if app.file_kind_picker_open {
                handle_kind_picker_key(app, key);
            } else if app.chart_config_open {
                handle_chart_config_key(app, key);
            } else if app.info_open {
                handle_info_key(app, key);
            } else if app.help_open {
                handle_help_key(app, key);
            } else if app.theme_open {
                handle_theme_key(app, key);
            } else if app.config_open {
                handle_config_key(app, key);
            } else if app.detail_kind_picker_open {
                handle_detail_kind_picker_key(app, key);
            } else if app.detail_searching {
                handle_detail_search_key(app, key);
            } else if app.detail_open {
                handle_detail_key(app, key);
            } else {
                handle_main_key(app, key, source);
            }
        }
        InputMode::Search => handle_search_key(app, key),
        InputMode::ReverseLookup => handle_reverse_lookup_key(app, key, source),
    }
}

fn handle_main_key(app: &mut App, key: KeyEvent, source: &dyn ProcessSource) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => {
            app.clear_search();
            app.reverse_results.clear();
            app.status_message = None;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Enter => {
            if app.search_active {
                app.clear_search();
            } else {
                app.open_detail();
            }
        }
        KeyCode::Char('n') => app.next_search_match(),
        KeyCode::Char('/') => app.enter_search_mode(),
        KeyCode::Char('v') => app.toggle_view(),
        KeyCode::Char('c') => app.open_config(),
        KeyCode::Char('r') => {
            if app.main_view == MainView::Processes {
                app.enter_reverse_lookup_mode();
            }
        }
        KeyCode::Char('R') => {
            app.refresh(source);
            app.status_message = Some("Refreshed".into());
        }
        KeyCode::Char('i') => app.open_info(),
        KeyCode::Char('g') => app.toggle_summary(),
        KeyCode::Char('?') => app.open_help(),
        KeyCode::Char('t') => app.open_theme_picker(),
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Char('S') => app.reverse_sort(),
        KeyCode::Char('K') => app.open_kill_picker(),
        KeyCode::Char('a') => app.toggle_show_all(),
        KeyCode::Char('y') => {
            if app.main_view == MainView::Files {
                let msg = app.yank_selected_path();
                app.status_message = Some(msg);
            }
        }
        KeyCode::Char('f') => {
            if app.main_view == MainView::Files {
                app.open_kind_picker();
            }
        }
        _ => {}
    }
}

fn handle_kind_picker_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('f') => app.close_kind_picker(),
        KeyCode::Up | KeyCode::Char('k') => app.kind_picker_up(),
        KeyCode::Down | KeyCode::Char('j') => app.kind_picker_down(),
        KeyCode::Enter => app.kind_picker_select(),
        _ => {}
    }
}

fn handle_detail_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            if !app.detail_search.is_empty() {
                app.detail_search_clear();
            } else {
                app.close_detail();
            }
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Left | KeyCode::Char('h') => app.scroll_left(),
        KeyCode::Right | KeyCode::Char('l') => app.scroll_right(),
        KeyCode::Char('/') => app.detail_search_start(),
        KeyCode::Char('f') => {
            if app.main_view == MainView::Processes {
                app.open_detail_kind_picker();
            }
        }
        KeyCode::Char('y') => {
            let msg = app.yank_selected_path();
            app.status_message = Some(msg);
        }
        _ => {}
    }
}

fn handle_kill_key(app: &mut App, key: KeyEvent) {
    match &app.kill_state {
        Some(KillState::Picking { .. }) => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.close_kill(),
            KeyCode::Up | KeyCode::Char('k') => app.kill_pick_up(),
            KeyCode::Down | KeyCode::Char('j') => app.kill_pick_down(),
            KeyCode::Enter => app.kill_select(),
            _ => {}
        },
        Some(KillState::CustomInput { .. }) => match key.code {
            KeyCode::Esc => app.close_kill(),
            KeyCode::Enter => app.kill_custom_confirm(),
            KeyCode::Backspace => app.kill_custom_pop(),
            KeyCode::Char(c) => app.kill_custom_push(c),
            _ => {}
        },
        Some(KillState::Confirming { .. }) => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.kill_confirm(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.close_kill(),
            _ => {}
        },
        None => {}
    }
}

fn handle_detail_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.detail_search_clear(),
        KeyCode::Enter => app.detail_search_stop(),
        KeyCode::Backspace => app.detail_search_pop(),
        KeyCode::Char(c) => app.detail_search_push(c),
        _ => {}
    }
}

fn handle_detail_kind_picker_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('f') => app.close_detail_kind_picker(),
        KeyCode::Up | KeyCode::Char('k') => app.detail_kind_picker_up(),
        KeyCode::Down | KeyCode::Char('j') => app.detail_kind_picker_down(),
        KeyCode::Enter => app.detail_kind_picker_select(),
        _ => {}
    }
}

fn handle_chart_config_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('c') => app.close_chart_config(),
        KeyCode::Up | KeyCode::Char('k') => app.chart_config_move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.chart_config_move_down(),
        KeyCode::Enter | KeyCode::Char(' ') => app.chart_config_toggle_selected(),
        _ => {}
    }
}

fn handle_config_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('c') => app.close_config(),
        KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Up | KeyCode::Char('k') => app.config_move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.config_move_down(),
        KeyCode::Enter | KeyCode::Char(' ') => app.config_toggle_selected(),
        _ => {}
    }
}

fn handle_info_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('i') => app.close_info(),
        KeyCode::Char('1') => app.info_set_tab(0),
        KeyCode::Char('2') => app.info_set_tab(1),
        KeyCode::Char('3') => app.info_set_tab(2),
        KeyCode::Char('4') => app.info_set_tab(3),
        KeyCode::Tab => app.info_next_tab(),
        KeyCode::BackTab => app.info_prev_tab(),
        KeyCode::Up | KeyCode::Char('k') => app.info_scroll_up(),
        KeyCode::Down | KeyCode::Char('j') => app.info_scroll_down(),
        KeyCode::Char('y') => {
            let msg = app.yank_info_env();
            app.status_message = Some(msg);
        }
        KeyCode::Char('Y') => {
            let msg = app.yank_all_env();
            app.status_message = Some(msg);
        }
        KeyCode::Char('c') => app.open_chart_config(),
        _ => {}
    }
}

fn handle_help_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => app.close_help(),
        KeyCode::Up | KeyCode::Char('k') => app.help_scroll_up(),
        KeyCode::Down | KeyCode::Char('j') => app.help_scroll_down(),
        _ => {}
    }
}

fn handle_theme_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('t') | KeyCode::Enter => {
            app.close_theme_picker();
        }
        KeyCode::Up | KeyCode::Char('k') => app.theme_move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.theme_move_down(),
        _ => {}
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.clear_search();
            app.exit_input_mode();
        }
        KeyCode::Enter => app.confirm_search(),
        KeyCode::Backspace => app.pop_input_char(),
        KeyCode::Char(c) => app.push_input_char(c),
        _ => {}
    }
}

fn handle_reverse_lookup_key(app: &mut App, key: KeyEvent, source: &dyn ProcessSource) {
    match key.code {
        KeyCode::Esc => {
            app.reverse_lookup_text.clear();
            app.exit_input_mode();
        }
        KeyCode::Enter => app.perform_reverse_lookup(source),
        KeyCode::Backspace => app.pop_input_char(),
        KeyCode::Char(c) => app.push_input_char(c),
        _ => {}
    }
}
