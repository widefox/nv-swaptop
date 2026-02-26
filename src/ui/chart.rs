use crate::data::types::{SizeUnits, convert_swap};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    symbols::Marker,
    text::Line,
    widgets::{Axis, Block, BorderType, Chart, Dataset, GraphType},
};

const LINUX: bool = cfg!(target_os = "linux");

pub fn render_animated_chart(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    chart_data: &[(f64, f64)],
    time_window: [f64; 2],
    total_swap: u64,
    used_swap: u64,
    swap_size_unit: &SizeUnits,
    display_devices: bool,
) {
    let total = convert_swap(total_swap, swap_size_unit.clone());
    let used = convert_swap(used_swap, swap_size_unit.clone());

    let total_used_title: String = match swap_size_unit {
        SizeUnits::KB => format!("total: {} | used: {}", total, used),
        SizeUnits::MB => format!("total: {} | used: {:.2}", total.round(), used),
        SizeUnits::GB => format!("total: {:.2} | used: {:.2}", total, used),
    };

    let total_n_used_line = if display_devices {
        Line::from("").fg(theme.text).left_aligned()
    } else {
        Line::from(total_used_title).fg(theme.text).left_aligned()
    };

    let swap_usage_percent = used_swap as f64 / total_swap as f64 * 100.0;
    let datasets = vec![
        Dataset::default()
            .marker(Marker::Braille)
            .style(Style::default().fg(theme.primary))
            .graph_type(GraphType::Line)
            .data(chart_data),
    ];

    let bottom_title = if LINUX && !display_devices {
        "(h to show swap devices)"
    } else {
        ""
    };
    let chart = Chart::new(datasets)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(
                    Line::from(format!("swap usage {}%", swap_usage_percent.round() as u64))
                        .fg(theme.primary)
                        .bold()
                        .right_aligned(),
                )
                .title(total_n_used_line)
                .title_bottom(Line::from(bottom_title).left_aligned())
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.background)),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(theme.text))
                .bounds(time_window),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(theme.text))
                .bounds([0.0, total_swap as f64]),
        );

    frame.render_widget(chart, area);
}
