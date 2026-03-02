use crate::data::types::{NumaNode, NumaNodeType, SizeUnits, UnifiedProcessInfo, convert_swap};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
};

// NUMA memory locality colour constants
const COLOR_LOCAL_GREEN: Color = Color::Rgb(80, 200, 120);
const COLOR_REMOTE_ORANGE: Color = Color::Rgb(255, 183, 77);
const COLOR_HBM_RED: Color = Color::Rgb(255, 85, 85);

pub fn render_unified_view(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    unified_procs: &[UnifiedProcessInfo],
    unit: &SizeUnits,
    numa_nodes: &[NumaNode],
) {
    let mut lines = Vec::new();

    // Build sorted list of NUMA node IDs for dynamic columns
    let mut node_ids: Vec<u32> = numa_nodes.iter().map(|n| n.id).collect();
    node_ids.sort();

    // Build header labels for each NUMA node (e.g. "N0", "N1", "N2(HBM)")
    let node_labels: Vec<String> = node_ids
        .iter()
        .map(|&id| {
            numa_nodes
                .iter()
                .find(|n| n.id == id)
                .map(|n| match &n.node_type {
                    NumaNodeType::GpuHbm { .. } => format!("N{}(HBM)", id),
                    _ => format!("N{}", id),
                })
                .unwrap_or_else(|| format!("N{}", id))
        })
        .collect();

    // Header
    let mut header_spans: Vec<Span> = vec![
        format!("{:>8}", "PID").bold(),
        Span::from(" "),
        format!("{:<16}", "NAME").bold(),
        Span::from(" "),
        format!("{:>5}", "CPU→N").bold(),
        Span::from(" "),
        format!("{:>5}", "GPU→N").bold(),
    ];

    // Dynamic per-node columns
    for label in &node_labels {
        header_spans.push(Span::from(" "));
        header_spans.push(format!("{:>9}", label).bold());
    }

    header_spans.push(Span::from(" "));
    header_spans.push(format!("{:>10}", "SWAP").bold());
    header_spans.push(Span::from(" "));
    header_spans.push(format!("{:>10}", "GPU MEM").bold());

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

            let cpu_n_str = if proc.cpu_nodes.is_empty() {
                "-".to_string()
            } else {
                proc.cpu_nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
            };

            let gpu_n_str = if proc.gpu_nodes.is_empty() {
                "-".to_string()
            } else {
                proc.gpu_nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
            };

            let mut spans: Vec<Span> = vec![
                format!("{:>8}", proc.pid).into(),
                " ".into(),
                format!("{:<16}", truncate(&proc.name, 16)).into(),
                " ".into(),
                format!("{:>5}", cpu_n_str).into(),
                " ".into(),
                format!("{:>5}", gpu_n_str).into(),
            ];

            // Per-node memory columns with locality colouring:
            //   green = local CPU node, orange = remote CPU node, red = GPU HBM
            for &node_id in &node_ids {
                let kb = proc.kb_per_node.get(&node_id).copied().unwrap_or(0);
                let cell = if kb > 0 {
                    format_mem(kb, unit)
                } else {
                    "-".to_string()
                };
                spans.push(" ".into());
                if kb > 0 {
                    let node_type = numa_nodes.iter().find(|n| n.id == node_id).map(|n| &n.node_type);
                    let color = match node_type {
                        Some(NumaNodeType::GpuHbm { .. }) => COLOR_HBM_RED,
                        Some(NumaNodeType::Cpu) if proc.cpu_nodes.contains(&node_id) => COLOR_LOCAL_GREEN,
                        _ => COLOR_REMOTE_ORANGE,
                    };
                    spans.push(Span::styled(
                        format!("{:>9}", cell),
                        Style::default().fg(color),
                    ));
                } else {
                    spans.push(format!("{:>9}", cell).into());
                }
            }

            spans.push(" ".into());
            spans.push(format!("{:>10}", swap_str).into());
            spans.push(" ".into());
            spans.push(format!("{:>10}", gpu_str).into());

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
            Line::from(vec![
                Span::raw(" "),
                Span::styled("local", Style::default().fg(COLOR_LOCAL_GREEN)),
                Span::raw("  "),
                Span::styled("remote", Style::default().fg(COLOR_REMOTE_ORANGE)),
                Span::raw("  "),
                Span::styled("GPU HBM", Style::default().fg(COLOR_HBM_RED)),
                Span::raw(" "),
            ])
                .right_aligned(),
        );

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

pub(crate) fn format_mem(kb: u64, unit: &SizeUnits) -> String {
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
