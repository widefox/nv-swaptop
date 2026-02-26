use crate::data::{DataProvider, SizeUnits, aggregate_processes};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

pub fn create_process_lines(
    provider: &dyn DataProvider,
    swap_size_unit: &SizeUnits,
    aggregated: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        format!("{:>12}", if aggregated { "COUNT" } else { "PID" }).bold(),
        " | ".into(),
        format!("{:30}", "PROCESS").bold(),
        " | ".into(),
        format!("{:10}", "USED").bold(),
    ]));

    if let Ok(mut processes) = provider.get_processes_swap(swap_size_unit) {
        processes.sort_by(|a, b| {
            b.swap_size
                .partial_cmp(&a.swap_size)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if aggregated {
            processes = aggregate_processes(processes);
        }

        for process in processes {
            let mut process_size: String = format!("{:.2}", process.swap_size);
            if let SizeUnits::KB = swap_size_unit {
                process_size = format!("{}", process.swap_size)
            }

            lines.push(Line::from(vec![
                format!("{:12}", process.pid).into(),
                " | ".into(),
                format!("{:30}", process.name).into(),
                " | ".into(),
                format!("{:10}", process_size).into(),
            ]));
        }
    }

    lines
}

pub fn render_processes_list(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    swap_size_unit: &SizeUnits,
    swap_processes_lines: &[Line<'static>],
    vertical_scroll: &mut usize,
    vertical_scroll_state: &mut ScrollbarState,
    visible_height: &mut usize,
) {
    let unit_buttons = match swap_size_unit {
        SizeUnits::KB => "▶KB◀─MB─GB",
        SizeUnits::MB => "KB─▶MB◀─GB",
        SizeUnits::GB => "KB─MB─▶GB◀",
    };

    *visible_height = area.height as usize;
    let content_height = swap_processes_lines.len() + 2;
    *vertical_scroll = (*vertical_scroll).min(content_height.saturating_sub(*visible_height));
    *vertical_scroll_state = vertical_scroll_state
        .content_length(content_height)
        .position(*vertical_scroll);

    let bottom_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background))
        .title(
            Line::from("(a to aggregate) (u/d|▲/▼|home/end|pgup/pgdown to scroll)")
                .fg(theme.text)
                .right_aligned(),
        )
        .title(
            Line::from(format!("unit (k/m/g to change): {}", unit_buttons))
                .fg(theme.secondary)
                .bold()
                .left_aligned(),
        );

    let process_paragraph = Paragraph::new(swap_processes_lines.to_vec())
        .alignment(Alignment::Center)
        .block(bottom_block)
        .scroll((*vertical_scroll as u16, 0));

    frame.render_widget(process_paragraph, area);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .style(Style::default().fg(theme.scrollbar))
            .thumb_style(Style::default().fg(theme.primary)),
        area,
        vertical_scroll_state,
    );
}
