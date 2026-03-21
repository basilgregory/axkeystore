use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Frame,
};

use crate::tui::app::{App, InputMode};

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(size);

    let title = Paragraph::new(Span::styled(
        " AxKeyStore Vault ",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(chunks[1]);

    // Construct the list of items
    let mut items = Vec::new();
    let mut current_category: Option<String> = None;

    let mut item_index = 0;
    for (category, pairs) in &app.entries {
        if *category != current_category {
            let cat_name = match category {
                Some(c) => format!("[{}]", c),
                None => "(uncategorized)".to_string(),
            };
            items.push(ListItem::new(Line::from(Span::styled(
                cat_name,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))));
            current_category = category.clone();
        }

        for (name, _) in pairs {
            let mut style = Style::default().fg(Color::White);
            if item_index == app.selected_index {
                // Highlight the selected item
                style = style.fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD);
            }

            items.push(ListItem::new(Line::from(Span::styled(
                format!("  {}", name),
                style,
            ))));

            item_index += 1;
        }
    }

    let keys_list = List::new(items)
        .block(Block::default().title("Keys").borders(Borders::ALL));

    f.render_widget(keys_list, body_chunks[0]);

    // Detail view
    let detail_text = if !app.flat_entries.is_empty() {
        let selected = &app.flat_entries[app.selected_index];
        let cat_display = match &selected.0 {
            Some(c) => c.clone(),
            None => "(uncategorized)".to_string(),
        };

        vec![
            Line::from(vec![
                Span::styled("Category: ", Style::default().fg(Color::Gray)),
                Span::raw(cat_display),
            ]),
            Line::from(vec![
                Span::styled("Key:      ", Style::default().fg(Color::Gray)),
                Span::raw(&selected.1),
            ]),
            Line::from(""),
            Line::from(Span::styled("Value:", Style::default().fg(Color::Gray))),
            Line::from(Span::styled(&selected.2, Style::default().fg(Color::Green))),
        ]
    } else {
        vec![Line::from(Span::raw("No keys found in this profile."))]
    };

    let detail_view = Paragraph::new(detail_text)
        .block(Block::default().title("Details").borders(Borders::ALL));

    f.render_widget(detail_view, body_chunks[1]);

    // Footer
    let footer_text = match app.input_mode {
        InputMode::Normal => " Navigate: \u{2191}/\u{2193} | Add Key: a | Quit: q / Esc ",
        _ => " Type your input | Enter to submit | Esc to cancel "
    };
    let footer = Paragraph::new(Span::styled(
        footer_text,
        Style::default().fg(Color::DarkGray),
    ))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);

    // Handle Input Popups
    match app.input_mode {
        InputMode::Normal => {}
        InputMode::AddingCategory => {
            draw_input_popup(f, "Enter Category (Optional)", &app.category_input, false);
        }
        InputMode::AddingName => {
            draw_input_popup(f, "Enter Key Name", &app.name_input, false);
        }
        InputMode::AddingValue => {
            draw_input_popup(f, "Enter Value", &app.value_input, true);
        }
        InputMode::Processing => {
            draw_msg_popup(f, "Processing...", "Saving your key securely.");
        }
        InputMode::Error(ref msg) => {
            draw_msg_popup(f, "Error", msg);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn draw_input_popup(f: &mut Frame, title: &str, input: &str, mask: bool) {
    let area = centered_rect(50, 20, f.area());
    f.render_widget(Clear, area);

    let display_text = if mask {
        "*".repeat(input.len())
    } else {
        input.to_string()
    };

    let block = Block::default().title(title).borders(Borders::ALL);
    let paragraph = Paragraph::new(display_text).block(block);
    f.render_widget(paragraph, area);

    let x = area.x.saturating_add(1).saturating_add(input.len() as u16);
    let y = area.y.saturating_add(1);
    
    // Only set cursor if it's within bounds
    if x < area.x + area.width && y < area.y + area.height {
        f.set_cursor_position((x, y));
    }
}

fn draw_msg_popup(f: &mut Frame, title: &str, msg: &str) {
    let area = centered_rect(50, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default().title(title).borders(Borders::ALL);
    let paragraph = Paragraph::new(msg).block(block);
    f.render_widget(paragraph, area);
}
