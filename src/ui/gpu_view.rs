use crate::data::types::{GpuDevice, GpuProcessInfo, SizeUnits, convert_swap};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph},
};

pub fn render_gpu_view(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    gpu_devices: &[GpuDevice],
    gpu_processes: &[GpuProcessInfo],
    gpu_available: bool,
    unit: &SizeUnits,
) {
    if !gpu_available || gpu_devices.is_empty() {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background))
            .title(Line::from(" GPU Info ").fg(theme.primary).bold());
        let msg = Paragraph::new("No NVIDIA GPU detected (nvidia-smi not available)")
            .block(block)
            .centered();
        frame.render_widget(msg, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_device_summary(frame, chunks[0], theme, gpu_devices, unit);
    render_gpu_process_list(frame, chunks[1], theme, gpu_processes, unit);
}

fn render_device_summary(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    devices: &[GpuDevice],
    unit: &SizeUnits,
) {
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        format!("{:>4}", "GPU").bold(),
        " | ".into(),
        format!("{:<24}", "NAME").bold(),
        " | ".into(),
        format!("{:>10}", "MEM TOTAL").bold(),
        " | ".into(),
        format!("{:>10}", "MEM USED").bold(),
        " | ".into(),
        format!("{:>10}", "MEM FREE").bold(),
        " | ".into(),
        format!("{:>5}", "TEMP").bold(),
        " | ".into(),
        format!("{:>6}", "NUMA").bold(),
    ]));

    for dev in devices {
        let total = format_mem(dev.memory_total_kb, unit);
        let used = format_mem(dev.memory_used_kb, unit);
        let free = format_mem(dev.memory_free_kb, unit);
        let temp = dev
            .temperature
            .map(|t| format!("{}°C", t))
            .unwrap_or_else(|| "-".into());
        let numa = dev
            .numa_node_id
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".into());

        lines.push(Line::from(vec![
            format!("{:>4}", dev.index).into(),
            " | ".into(),
            format!("{:<24}", truncate(&dev.name, 24)).into(),
            " | ".into(),
            format!("{:>10}", total).into(),
            " | ".into(),
            format!("{:>10}", used).into(),
            " | ".into(),
            format!("{:>10}", free).into(),
            " | ".into(),
            format!("{:>5}", temp).into(),
            " | ".into(),
            format!("{:>6}", numa).into(),
        ]));
    }

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background))
        .title(Line::from(" GPU Devices ").fg(theme.primary).bold());

    let para = Paragraph::new(lines).block(block).centered();
    frame.render_widget(para, area);
}

fn render_gpu_process_list(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    processes: &[GpuProcessInfo],
    unit: &SizeUnits,
) {
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        format!("{:>8}", "PID").bold(),
        " | ".into(),
        format!("{:<30}", "PROCESS").bold(),
        " | ".into(),
        format!("{:>4}", "GPU").bold(),
        " | ".into(),
        format!("{:>12}", "VRAM USED").bold(),
    ]));

    if processes.is_empty() {
        lines.push(Line::from("  No GPU processes running"));
    } else {
        for proc in processes {
            let mem = format_mem(proc.gpu_memory_used_kb, unit);
            lines.push(Line::from(vec![
                format!("{:>8}", proc.pid).into(),
                " | ".into(),
                format!("{:<30}", truncate(&proc.name, 30)).into(),
                " | ".into(),
                format!("{:>4}", proc.gpu_index).into(),
                " | ".into(),
                format!("{:>12}", mem).into(),
            ]));
        }
    }

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background))
        .title(Line::from(" GPU Processes ").fg(theme.primary).bold());

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
