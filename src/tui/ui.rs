use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;

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
    let footer_text = " Navigate: \u{2191}/\u{2193} | Quit: q / Esc ";
    let footer = Paragraph::new(Span::styled(
        footer_text,
        Style::default().fg(Color::DarkGray),
    ))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}
