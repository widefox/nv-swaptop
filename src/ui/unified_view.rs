use crate::data::types::{ProcessLocation, SizeUnits, UnifiedProcessInfo, convert_swap};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
};

pub fn render_unified_view(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    unified_procs: &[UnifiedProcessInfo],
    unit: &SizeUnits,
) {
    let mut lines = Vec::new();

    // Header
    let mut header_spans = vec![
        format!("{:>8}", "PID").bold(),
        Span::from(" | "),
        format!("{:<20}", "NAME").bold(),
        Span::from(" | "),
        format!("{:>10}", "SWAP").bold(),
        Span::from(" | "),
        format!("{:>10}", "GPU MEM").bold(),
        Span::from(" | "),
    ];

    #[cfg(target_os = "linux")]
    {
        header_spans.push(format!("{:>6}", "NUMA").bold());
        header_spans.push(Span::from(" | "));
    }

    header_spans.push(format!("{:>10}", "LOCATION").bold());

    lines.push(Line::from(header_spans));

    if unified_procs.is_empty() {
        lines.push(Line::from("  No process data available"));
    } else {
        for proc in unified_procs {
            let swap_str = format_mem(proc.swap_kb, unit);
            let gpu_str = proc
                .gpu_memory_kb
                .map(|kb| format_mem(kb, unit))
                .unwrap_or_else(|| "-".into());

            let (location_str, location_color) = match proc.location {
                ProcessLocation::CpuOnly => ("CPU", theme.text),
                ProcessLocation::GpuOnly => ("GPU", Color::Rgb(118, 185, 0)), // green
                ProcessLocation::CpuAndGpu => ("CPU+GPU", Color::Rgb(255, 183, 77)), // orange/amber
            };

            let mut spans: Vec<Span> = vec![
                format!("{:>8}", proc.pid).into(),
                " | ".into(),
                format!("{:<20}", truncate(&proc.name, 20)).into(),
                " | ".into(),
                format!("{:>10}", swap_str).into(),
                " | ".into(),
                format!("{:>10}", gpu_str).into(),
                " | ".into(),
            ];

            #[cfg(target_os = "linux")]
            {
                let numa_str = proc
                    .numa_node
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".into());
                spans.push(format!("{:>6}", numa_str).into());
                spans.push(" | ".into());
            }

            spans.push(
                Span::styled(
                    format!("{:>10}", location_str),
                    Style::default().fg(location_color),
                ),
            );

            lines.push(Line::from(spans));
        }
    }

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background))
        .title(
            Line::from(" Unified CPU+GPU+NUMA View ")
                .fg(theme.primary)
                .bold(),
        )
        .title(
            Line::from(" (orange = HBM migration detected) ")
                .fg(Color::Rgb(255, 183, 77))
                .right_aligned(),
        );

    let para = Paragraph::new(lines).block(block).centered();
    frame.render_widget(para, area);
}

fn format_mem(kb: u64, unit: &SizeUnits) -> String {
    let val = convert_swap(kb, unit.clone());
    let suffix = match unit {
        SizeUnits::KB => "KB",
        SizeUnits::MB => "MB",
        SizeUnits::GB => "GB",
    };
    match unit {
        SizeUnits::KB => format!("{} {}", val as u64, suffix),
        _ => format!("{:.2} {}", val, suffix),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
