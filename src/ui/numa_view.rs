use crate::data::types::{NumaNode, NumaNodeType, ProcessNumaInfo};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
};

pub fn render_numa_view(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    numa_nodes: &[NumaNode],
    process_numa_infos: &[ProcessNumaInfo],
    numa_available: bool,
) {
    if !numa_available || numa_nodes.is_empty() {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background))
            .title(Line::from(" NUMA Topology ").fg(theme.primary).bold());
        let msg = Paragraph::new("NUMA not available on this system")
            .block(block)
            .centered();
        frame.render_widget(msg, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_topology_table(frame, chunks[0], theme, numa_nodes);
    render_process_numa_distribution(frame, chunks[1], theme, process_numa_infos, numa_nodes);
}

fn render_topology_table(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    numa_nodes: &[NumaNode],
) {
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        format!("{:>6}", "NODE").bold(),
        " | ".into(),
        format!("{:<10}", "TYPE").bold(),
        " | ".into(),
        format!("{:>12}", "MEM TOTAL").bold(),
        " | ".into(),
        format!("{:>12}", "MEM USED").bold(),
        " | ".into(),
        format!("{:<20}", "CPUs").bold(),
    ]));

    for node in numa_nodes {
        let type_str = match &node.node_type {
            NumaNodeType::Cpu => "CPU".to_string(),
            NumaNodeType::GpuHbm { gpu_index } => format!("GPU HBM {}", gpu_index),
            NumaNodeType::Unknown => "Unknown".to_string(),
        };

        let mem_used_kb = node.memory_total_kb.saturating_sub(node.memory_free_kb);
        let total_mb = node.memory_total_kb as f64 / 1024.0;
        let used_mb = mem_used_kb as f64 / 1024.0;

        let cpu_str = if node.cpus.is_empty() {
            "-".to_string()
        } else if node.cpus.len() <= 8 {
            node.cpus
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(",")
        } else {
            format!("{}-{} ({})", node.cpus[0], node.cpus[node.cpus.len() - 1], node.cpus.len())
        };

        lines.push(Line::from(vec![
            format!("{:>6}", node.id).into(),
            " | ".into(),
            format!("{:<10}", type_str).into(),
            " | ".into(),
            format!("{:>10.0} MB", total_mb).into(),
            " | ".into(),
            format!("{:>10.0} MB", used_mb).into(),
            " | ".into(),
            format!("{:<20}", cpu_str).into(),
        ]));
    }

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background))
        .title(Line::from(" NUMA Topology ").fg(theme.primary).bold());

    let para = Paragraph::new(lines).block(block).centered();
    frame.render_widget(para, area);
}

fn render_process_numa_distribution(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    process_infos: &[ProcessNumaInfo],
    numa_nodes: &[NumaNode],
) {
    let mut lines = Vec::new();

    // Header: PID | PROCESS | CPU | TOTAL PG | N0 | N1 | ...
    let mut header_spans = vec![
        format!("{:>8}", "PID").bold(),
        " | ".into(),
        format!("{:<20}", "PROCESS").bold(),
        " | ".into(),
        format!("{:>3}", "CPU").bold(),
        " | ".into(),
        format!("{:>10}", "TOTAL PG").bold(),
    ];
    for node in numa_nodes {
        header_spans.push(" | ".into());
        header_spans.push(format!("{:>8}", format!("N{}", node.id)).bold());
    }
    lines.push(Line::from(header_spans));

    for info in process_infos.iter().take(20) {
        // Determine dominant memory node (node with most pages)
        let dominant_node = info
            .pages_per_node
            .iter()
            .max_by_key(|(_, v)| **v)
            .map(|(k, _)| *k);

        // Check misalignment: cpu_node differs from dominant memory node
        let misaligned = match (info.cpu_node, dominant_node) {
            (Some(cpu), Some(mem)) => cpu != mem,
            _ => false,
        };

        let cpu_str = match info.cpu_node {
            Some(n) => format!("{:>3}", n),
            None => format!("{:>3}", "-"),
        };
        let cpu_span: Span = if misaligned {
            Span::styled(cpu_str, Style::default().fg(Color::Rgb(255, 183, 77)))
        } else {
            cpu_str.into()
        };

        let mut spans = vec![
            format!("{:>8}", info.pid).into(),
            " | ".into(),
            format!("{:<20}", truncate_name(&info.name, 20)).into(),
            " | ".into(),
            cpu_span,
            " | ".into(),
            format!("{:>10}", info.total_pages).into(),
        ];
        for node in numa_nodes {
            let pages = info.pages_per_node.get(&node.id).copied().unwrap_or(0);
            spans.push(" | ".into());
            spans.push(format!("{:>8}", pages).into());
        }
        lines.push(Line::from(spans));
    }

    if process_infos.is_empty() {
        lines.push(Line::from("  No process NUMA data available"));
    }

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background))
        .title(
            Line::from(" Per-Process NUMA Distribution (top 20 swap consumers) ")
                .fg(theme.primary)
                .bold(),
        );

    let para = Paragraph::new(lines).block(block).centered();
    frame.render_widget(para, area);
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}…", &name[..max_len - 1])
    }
}
