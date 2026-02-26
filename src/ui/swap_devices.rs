use crate::data::types::{InfoSwap, SizeUnits, convert_swap};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph},
};

#[cfg(target_os = "linux")]
use crate::data::swap::find_mount_device;

#[cfg(target_os = "linux")]
pub fn render_swap_devices(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    swap_devices: &[InfoSwap],
    swap_size_unit: &SizeUnits,
    total_swap: u64,
    used_swap: u64,
    display_devices: bool,
) {
    let total = convert_swap(total_swap, swap_size_unit.clone());
    let used = convert_swap(used_swap, swap_size_unit.clone());

    let total_used_title: String = match swap_size_unit {
        SizeUnits::KB => format!("total: {} | used: {}", total, used),
        SizeUnits::MB => format!("total: {} | used: {:.2}", total.round(), used),
        SizeUnits::GB => format!("total: {:.2} | used: {:.2}", total, used),
    };

    let total_n_used_line = if !display_devices {
        Line::from("").fg(theme.text).left_aligned()
    } else {
        Line::from(total_used_title).fg(theme.text).left_aligned()
    };

    let name_width = swap_devices
        .iter()
        .map(|d| d.name.len())
        .max()
        .unwrap_or(10)
        .max(10);

    let source_width = swap_devices
        .iter()
        .map(|d| {
            let src = find_mount_device(std::path::Path::new(&d.name))
                .unwrap_or_else(|| "RAM".into());
            src.len()
        })
        .max()
        .unwrap_or(4)
        .max(4);

    let wide = area.width >= 80;
    let mut lines = Vec::new();

    if wide {
        lines.push(Line::from(format!(
            "{:<source_width$} | {:<name_width$} | {:<10} | {:>8} | {:>10} | {:>10}",
            "disk", "path", "type", "priority", "total", "used"
        )));
    } else {
        lines.push(Line::from(format!(
            "{:<source_width$} | {:<name_width$} | {:<10} | {:>10}",
            "disk", "path", "total", "used"
        )));
    }

    for device in swap_devices {
        let used = match swap_size_unit {
            SizeUnits::KB => device.used_kb.to_string(),
            _ => format!("{:.2}", device.used_kb),
        };

        let source = find_mount_device(std::path::Path::new(&device.name))
            .unwrap_or_else(|| "RAM".into());

        let total = match swap_size_unit {
            SizeUnits::KB => device.size_kb.to_string(),
            _ => format!("{:.2}", device.size_kb),
        };

        let row = if wide {
            format!(
                "{:<source_width$} | {:<name_width$} | {:<10} | {:>8} | {:>10} | {:>10}",
                source, device.name, device.kind, device.priority, total, used
            )
        } else {
            format!(
                "{:<source_width$} | {:<name_width$} | {:<10} | {:>10}",
                source, device.name, total, used
            )
        };
        lines.push(Line::from(row));
    }

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background))
        .title(total_n_used_line.right_aligned())
        .title(Line::from("swap devices").fg(theme.text).left_aligned())
        .title_bottom(Line::from("(h to hide swap devices)").left_aligned());

    let para = Paragraph::new(lines).block(block).centered();
    frame.render_widget(para, area);
}
