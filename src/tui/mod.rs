use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{io, time::Duration};

pub mod app;
pub mod ui;

use app::App;
use crate::storage::Storage;

pub type TuiTerminal = Terminal<CrosstermBackend<std::io::Stdout>>;

pub fn init_terminal() -> Result<TuiTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(Into::into)
}

pub fn restore_terminal(mut terminal: TuiTerminal) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn draw_loading(terminal: &mut TuiTerminal, msg: &str) -> Result<()> {
    terminal.draw(|f| {
        use ratatui::{
            style::{Color, Style},
            widgets::{Block, Borders, Paragraph, Clear},
        };
        let area = ui::centered_rect(50, 20, f.area());
        f.render_widget(Clear, area);
        let block = Block::default().title("Loading").borders(Borders::ALL);
        let paragraph = Paragraph::new(msg).block(block).style(Style::default().fg(Color::Cyan));
        f.render_widget(paragraph, area);
    })?;
    Ok(())
}

pub async fn run(mut terminal: TuiTerminal, storage: Storage, master_key: String) -> Result<()> {
    // create app and run it
    let mut app = App::new(storage, master_key).await?;
    let res = run_app(&mut terminal, &mut app).await;

    // restore terminal
    restore_terminal(terminal)?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // non-blocking event read
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    app::InputMode::Normal => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Up => app.previous(),
                            KeyCode::Down => app.next(),
                            KeyCode::Char('a') => app.start_add_key(),
                            KeyCode::Char('p') => app.start_switch_profile(),
                            _ => {}
                        }
                    }
                    app::InputMode::AddingCategory 
                    | app::InputMode::AddingName 
                    | app::InputMode::AddingValue => {
                        match key.code {
                            KeyCode::Char(c) => app.handle_char(c),
                            KeyCode::Backspace => app.handle_backspace(),
                            KeyCode::Enter => {
                                if app.handle_enter() {
                                    // Draw "Processing..." popup before starting async operation
                                    terminal.draw(|f| ui::draw(f, app))?;
                                    if let Err(e) = app.save_new_key().await {
                                        app.input_mode = app::InputMode::Error(format!("Fatal error: {}", e));
                                    }
                                }
                            }
                            KeyCode::Esc => app.cancel_input(),
                            _ => {}
                        }
                    }
                    app::InputMode::SelectingProfile => {
                        match key.code {
                            KeyCode::Up => app.previous_profile(),
                            KeyCode::Down => app.next_profile(),
                            KeyCode::Enter => app.select_profile(),
                            KeyCode::Char('c') => app.start_create_profile(),
                            KeyCode::Char('d') => app.start_delete_profile(),
                            KeyCode::Esc => app.cancel_input(),
                            _ => {}
                        }
                    }
                    app::InputMode::AddingProfileName 
                    | app::InputMode::AddingProfileRepo 
                    | app::InputMode::AddingProfilePassword => {
                        match key.code {
                            KeyCode::Char(c) => app.handle_create_profile_char(c),
                            KeyCode::Backspace => app.handle_create_profile_backspace(),
                            KeyCode::Enter => {
                                if app.handle_create_profile_enter() {
                                    terminal.draw(|f| ui::draw(f, app))?;
                                    if let Err(e) = app.execute_create_profile().await {
                                        app.input_mode = app::InputMode::Error(format!("Fatal error: {}", e));
                                    }
                                }
                            }
                            KeyCode::Esc => app.start_switch_profile(),
                            _ => {}
                        }
                    }
                    app::InputMode::ConfirmingDeleteProfile => {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                terminal.draw(|f| ui::draw(f, app))?;
                                if let Err(e) = app.execute_delete_profile().await {
                                    app.input_mode = app::InputMode::Error(format!("Fatal error: {}", e));
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                app.start_switch_profile();
                            }
                            _ => {}
                        }
                    }
                    app::InputMode::EnteringPasswordForProfile => {
                        match key.code {
                            KeyCode::Char(c) => app.handle_password_char(c),
                            KeyCode::Backspace => app.handle_password_backspace(),
                            KeyCode::Enter => {
                                terminal.draw(|f| ui::draw(f, app))?;
                                if let Err(e) = app.submit_profile_switch().await {
                                    app.input_mode = app::InputMode::Error(format!("Fatal error: {}", e));
                                }
                            }
                            KeyCode::Esc => app.cancel_input(),
                            _ => {}
                        }
                    }
                    app::InputMode::Processing => {
                        // User shouldn't really be able to trigger this unless event loop continues polling
                        // while we draw "Processing...". But because save_new_key is awaited inline above,
                        // this state is effectively transitionary and we won't poll events here.
                    }
                    app::InputMode::Error(_) => {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => app.cancel_input(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
