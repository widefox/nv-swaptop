use crate::data::{ActiveView, DataProvider, GpuDevice, GpuProcessInfo, SizeUnits, SwapUpdate, UnifiedProcessInfo};
#[cfg(target_os = "linux")]
use crate::data::{NumaNode, ProcessNumaInfo};
use crate::theme::{Theme, ThemeType};
use crate::ui;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, ScrollbarState},
};
use std::time::{Duration, Instant};

const LINUX: bool = cfg!(target_os = "linux");

// Cache TTLs
const NUMA_TOPOLOGY_TTL: Duration = Duration::from_secs(30);
const NUMA_MAPS_TTL: Duration = Duration::from_secs(5);
const GPU_DEVICES_TTL: Duration = Duration::from_secs(10);
const GPU_PROCESSES_TTL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortColumn {
    Swap,
    GpuMem,
    Name,
    #[cfg(target_os = "linux")]
    NumaNode,
}

impl SortColumn {
    fn next(self) -> Self {
        match self {
            SortColumn::Swap => SortColumn::GpuMem,
            #[cfg(target_os = "linux")]
            SortColumn::GpuMem => SortColumn::NumaNode,
            #[cfg(not(target_os = "linux"))]
            SortColumn::GpuMem => SortColumn::Name,
            #[cfg(target_os = "linux")]
            SortColumn::NumaNode => SortColumn::Name,
            SortColumn::Name => SortColumn::Swap,
        }
    }

    fn label(self) -> &'static str {
        match self {
            SortColumn::Swap => "swap",
            SortColumn::GpuMem => "gpu_mem",
            SortColumn::Name => "name",
            #[cfg(target_os = "linux")]
            SortColumn::NumaNode => "numa",
        }
    }
}

pub struct App {
    provider: Box<dyn DataProvider>,
    running: bool,
    display_devices: bool,
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
    pub swap_size_unit: SizeUnits,
    pub swap_processes_lines: Vec<Line<'static>>,
    pub last_update: Option<Instant>,
    pub chart_info: SwapUpdate,
    pub aggregated: bool,
    current_theme: ThemeType,
    time_window: [f64; 2],
    chart_data: Vec<(f64, f64)>,
    timeout: u64,
    visible_height: usize,
    active_view: ActiveView,
    #[cfg(target_os = "linux")]
    numa_nodes: Vec<NumaNode>,
    #[cfg(target_os = "linux")]
    process_numa_infos: Vec<ProcessNumaInfo>,
    gpu_devices: Vec<GpuDevice>,
    gpu_processes: Vec<GpuProcessInfo>,
    unified_procs: Vec<UnifiedProcessInfo>,
    sort_column: SortColumn,
    // Cache timestamps
    #[cfg(target_os = "linux")]
    numa_topology_last: Option<Instant>,
    #[cfg(target_os = "linux")]
    numa_maps_last: Option<Instant>,
    gpu_devices_last: Option<Instant>,
    gpu_processes_last: Option<Instant>,
}

impl App {
    pub fn new(provider: Box<dyn DataProvider>) -> Self {
        Self {
            provider,
            running: false,
            display_devices: false,
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
            swap_size_unit: SizeUnits::KB,
            swap_processes_lines: Vec::new(),
            last_update: None,
            chart_info: SwapUpdate::default(),
            aggregated: false,
            current_theme: ThemeType::Dracula,
            time_window: [0.0, 60.0],
            chart_data: Vec::new(),
            timeout: 1000,
            visible_height: 0,
            active_view: ActiveView::default(),
            #[cfg(target_os = "linux")]
            numa_nodes: Vec::new(),
            #[cfg(target_os = "linux")]
            process_numa_infos: Vec::new(),
            gpu_devices: Vec::new(),
            gpu_processes: Vec::new(),
            unified_procs: Vec::new(),
            sort_column: SortColumn::Swap,
            #[cfg(target_os = "linux")]
            numa_topology_last: None,
            #[cfg(target_os = "linux")]
            numa_maps_last: None,
            gpu_devices_last: None,
            gpu_processes_last: None,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        self.swap_processes_lines = ui::process_list::create_process_lines(
            self.provider.as_ref(),
            &self.swap_size_unit,
            self.aggregated,
        );
        self.chart_info = self.provider.get_swap_info(&self.swap_size_unit)?;
        self.refresh_numa_data();
        self.refresh_gpu_data();
        self.last_update = Some(Instant::now());

        while self.running {
            if event::poll(Duration::from_millis(100))? {
                self.handle_crossterm_events()?;
            }

            if let Some(last_update) = self.last_update
                && last_update.elapsed() >= Duration::from_millis(self.timeout)
            {
                self.chart_info = self.provider.get_swap_info(&self.swap_size_unit)?;
                self.update_chart_data();
                self.last_update = Some(Instant::now());
                self.swap_processes_lines = ui::process_list::create_process_lines(
                    self.provider.as_ref(),
                    &self.swap_size_unit,
                    self.aggregated,
                );
                if self.active_view == ActiveView::Numa {
                    self.refresh_numa_data();
                }
                if self.active_view == ActiveView::Gpu || self.active_view == ActiveView::Unified {
                    self.refresh_gpu_data();
                }
                if self.active_view == ActiveView::Unified {
                    self.refresh_unified_data();
                }
            }

            terminal.draw(|frame| self.render(frame))?;
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        self.swap_processes_lines = ui::process_list::create_process_lines(
            self.provider.as_ref(),
            &self.swap_size_unit,
            self.aggregated,
        );
        self.chart_info = self.provider.get_swap_info(&self.swap_size_unit)?;
        self.last_update = Some(Instant::now());

        while self.running {
            if event::poll(Duration::from_millis(100))? {
                self.handle_crossterm_events()?;
            }

            if let Some(last_update) = self.last_update
                && last_update.elapsed() >= Duration::from_millis(self.timeout)
            {
                self.chart_info = self.provider.get_swap_info(&self.swap_size_unit)?;
                self.update_chart_data();
                self.last_update = Some(Instant::now());
                self.swap_processes_lines = ui::process_list::create_process_lines(
                    self.provider.as_ref(),
                    &self.swap_size_unit,
                    self.aggregated,
                );
            }

            terminal.draw(|frame| self.render(frame))?;
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn refresh_numa_data(&mut self) {
        if !self.provider.is_numa_available() {
            return;
        }

        // Topology: 30s TTL
        let should_refresh_topology = self
            .numa_topology_last
            .map(|t| t.elapsed() >= NUMA_TOPOLOGY_TTL)
            .unwrap_or(true);

        if should_refresh_topology {
            if let Ok(nodes) = self.provider.get_numa_topology() {
                self.numa_nodes = nodes;
            }
            self.numa_topology_last = Some(Instant::now());
        }

        // NUMA maps: 5s TTL, only when NUMA view active
        if self.active_view != ActiveView::Numa {
            return;
        }

        let should_refresh_maps = self
            .numa_maps_last
            .map(|t| t.elapsed() >= NUMA_MAPS_TTL)
            .unwrap_or(true);

        if should_refresh_maps {
            let mut infos = Vec::new();
            if let Ok(mut procs) = self.provider.get_processes_swap(&self.swap_size_unit) {
                procs.sort_by(|a, b| {
                    b.swap_size
                        .partial_cmp(&a.swap_size)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                for proc in procs.iter().take(20) {
                    if let Ok(mut info) = self.provider.get_process_numa_maps(proc.pid, &proc.name) {
                        info.cpu_node = proc.last_cpu
                            .and_then(|cpu| crate::data::numa::cpu_to_numa_node(cpu, &self.numa_nodes));
                        infos.push(info);
                    }
                }
            }
            self.process_numa_infos = infos;
            self.numa_maps_last = Some(Instant::now());
        }
    }

    fn refresh_gpu_data(&mut self) {
        if !self.provider.is_gpu_available() {
            return;
        }

        // Devices: 10s TTL
        let should_refresh_devices = self
            .gpu_devices_last
            .map(|t| t.elapsed() >= GPU_DEVICES_TTL)
            .unwrap_or(true);

        if should_refresh_devices {
            if let Ok(devices) = self.provider.get_gpu_devices() {
                self.gpu_devices = devices;
            }
            self.gpu_devices_last = Some(Instant::now());
        }

        // Processes: 1s TTL
        let should_refresh_procs = self
            .gpu_processes_last
            .map(|t| t.elapsed() >= GPU_PROCESSES_TTL)
            .unwrap_or(true);

        if should_refresh_procs {
            if let Ok(procs) = self.provider.get_gpu_processes() {
                self.gpu_processes = procs;
            }
            self.gpu_processes_last = Some(Instant::now());
        }
    }

    #[cfg(target_os = "linux")]
    fn refresh_unified_data(&mut self) {
        let swap_procs = self
            .provider
            .get_processes_swap(&self.swap_size_unit)
            .unwrap_or_default();
        self.unified_procs = crate::data::merge_process_data(
            &swap_procs,
            &self.gpu_processes,
            &self.process_numa_infos,
            &self.numa_nodes,
        );
        self.sort_unified_procs();
    }

    #[cfg(not(target_os = "linux"))]
    fn refresh_unified_data(&mut self) {
        let swap_procs = self
            .provider
            .get_processes_swap(&self.swap_size_unit)
            .unwrap_or_default();
        self.unified_procs = crate::data::merge_process_data(
            &swap_procs,
            &self.gpu_processes,
        );
        self.sort_unified_procs();
    }

    fn sort_unified_procs(&mut self) {
        match self.sort_column {
            SortColumn::Swap => {
                self.unified_procs.sort_by(|a, b| b.swap_kb.cmp(&a.swap_kb));
            }
            SortColumn::GpuMem => {
                self.unified_procs.sort_by(|a, b| {
                    b.gpu_memory_kb.unwrap_or(0).cmp(&a.gpu_memory_kb.unwrap_or(0))
                });
            }
            SortColumn::Name => {
                self.unified_procs.sort_by(|a, b| a.name.cmp(&b.name));
            }
            #[cfg(target_os = "linux")]
            SortColumn::NumaNode => {
                self.unified_procs.sort_by(|a, b| a.numa_node.cmp(&b.numa_node));
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn render(&mut self, frame: &mut Frame) {
        let theme = Theme::from(self.current_theme);

        let main_block = self.create_main_block(&theme);
        let main_area = main_block.inner(frame.area());

        match self.active_view {
            ActiveView::Swap => self.render_swap_view(frame, main_area, &theme),
            ActiveView::Numa => {
                ui::numa_view::render_numa_view(
                    frame,
                    main_area,
                    &theme,
                    &self.numa_nodes,
                    &self.process_numa_infos,
                    self.provider.is_numa_available(),
                );
            }
            ActiveView::Gpu => {
                ui::gpu_view::render_gpu_view(
                    frame,
                    main_area,
                    &theme,
                    &self.gpu_devices,
                    &self.gpu_processes,
                    self.provider.is_gpu_available(),
                    &self.swap_size_unit,
                );
            }
            ActiveView::Unified => {
                ui::unified_view::render_unified_view(
                    frame,
                    main_area,
                    &theme,
                    &self.unified_procs,
                    &self.swap_size_unit,
                );
            }
        }

        frame.render_widget(main_block, frame.area());
    }

    #[cfg(target_os = "linux")]
    fn render_swap_view(&mut self, frame: &mut Frame, main_area: ratatui::layout::Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(main_area);

        if self.display_devices {
            let upper_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(chunks[0]);

            ui::chart::render_animated_chart(
                frame,
                upper_chunks[1],
                theme,
                &self.chart_data,
                self.time_window,
                self.chart_info.total_swap,
                self.chart_info.used_swap,
                &self.swap_size_unit,
                self.display_devices,
            );
            ui::process_list::render_processes_list(
                frame,
                chunks[1],
                theme,
                &self.swap_size_unit,
                &self.swap_processes_lines,
                &mut self.vertical_scroll,
                &mut self.vertical_scroll_state,
                &mut self.visible_height,
            );
            ui::swap_devices::render_swap_devices(
                frame,
                upper_chunks[0],
                theme,
                &self.chart_info.swap_devices,
                &self.swap_size_unit,
                self.chart_info.total_swap,
                self.chart_info.used_swap,
                self.display_devices,
            );
        } else {
            ui::chart::render_animated_chart(
                frame,
                chunks[0],
                theme,
                &self.chart_data,
                self.time_window,
                self.chart_info.total_swap,
                self.chart_info.used_swap,
                &self.swap_size_unit,
                self.display_devices,
            );
            ui::process_list::render_processes_list(
                frame,
                chunks[1],
                theme,
                &self.swap_size_unit,
                &self.swap_processes_lines,
                &mut self.vertical_scroll,
                &mut self.vertical_scroll_state,
                &mut self.visible_height,
            );
        }
    }

    #[cfg(target_os = "windows")]
    fn render(&mut self, frame: &mut Frame) {
        let theme = Theme::from(self.current_theme);

        let main_block = self.create_main_block(&theme);
        let main_area = main_block.inner(frame.area());

        match self.active_view {
            ActiveView::Gpu => {
                ui::gpu_view::render_gpu_view(
                    frame,
                    main_area,
                    &theme,
                    &self.gpu_devices,
                    &self.gpu_processes,
                    self.provider.is_gpu_available(),
                    &self.swap_size_unit,
                );
            }
            ActiveView::Unified => {
                ui::unified_view::render_unified_view(
                    frame,
                    main_area,
                    &theme,
                    &self.unified_procs,
                    &self.swap_size_unit,
                );
            }
            _ => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                    .split(main_area);

                ui::chart::render_animated_chart(
                    frame,
                    chunks[0],
                    &theme,
                    &self.chart_data,
                    self.time_window,
                    self.chart_info.total_swap,
                    self.chart_info.used_swap,
                    &self.swap_size_unit,
                    self.display_devices,
                );
                ui::process_list::render_processes_list(
                    frame,
                    chunks[1],
                    &theme,
                    &self.swap_size_unit,
                    &self.swap_processes_lines,
                    &mut self.vertical_scroll,
                    &mut self.vertical_scroll_state,
                    &mut self.visible_height,
                );
            }
        }

        frame.render_widget(main_block, frame.area());
    }

    fn create_main_block(&self, theme: &Theme) -> Block<'static> {
        let view_label = match self.active_view {
            ActiveView::Swap => "Swap",
            #[cfg(target_os = "linux")]
            ActiveView::Numa => "NUMA",
            ActiveView::Gpu => "GPU",
            ActiveView::Unified => "Unified",
        };

        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(
                Line::from(format!(" nv-swaptop [{}] sort:{} ", view_label, self.sort_column.label()))
                    .bold()
                    .fg(theme.primary)
                    .left_aligned(),
            )
            .title(
                Line::from(format!("theme (t): {:?}", self.current_theme))
                    .bold()
                    .fg(theme.primary)
                    .right_aligned(),
            )
            .title(
                Line::from(format!(" < {:?}ms >  Tab/1-4:view  s:sort ", self.timeout))
                    .bold()
                    .fg(theme.primary)
                    .centered(),
            )
            .style(Style::default().bg(theme.background).fg(theme.text))
    }

    fn update_chart_data(&mut self) {
        let timestamp = self.time_window[1];
        let swap_usage = self.chart_info.used_swap as f64;
        self.chart_data.push((timestamp, swap_usage));
        if self.chart_data.len() > 60 {
            self.chart_data.drain(0..1);
        }
        self.time_window[0] += 1.0;
        self.time_window[1] += 1.0;
    }

    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    fn cycle_view(&mut self) {
        self.active_view = match self.active_view {
            ActiveView::Swap => {
                #[cfg(target_os = "linux")]
                { ActiveView::Numa }
                #[cfg(not(target_os = "linux"))]
                { ActiveView::Gpu }
            }
            #[cfg(target_os = "linux")]
            ActiveView::Numa => ActiveView::Gpu,
            ActiveView::Gpu => ActiveView::Unified,
            ActiveView::Unified => ActiveView::Swap,
        };
    }

    #[cfg(target_os = "linux")]
    fn on_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),

            // View switching
            KeyCode::Tab => self.cycle_view(),
            KeyCode::Char('1') => self.active_view = ActiveView::Swap,
            KeyCode::Char('2') => self.active_view = ActiveView::Numa,
            KeyCode::Char('3') => self.active_view = ActiveView::Gpu,
            KeyCode::Char('4') => self.active_view = ActiveView::Unified,

            KeyCode::Char('d') | KeyCode::Down => {
                self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::Char('u') | KeyCode::Up => {
                self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::End => {
                self.vertical_scroll = self.swap_processes_lines.len();
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::Home => {
                self.vertical_scroll = 0;
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }

            KeyCode::PageDown => {
                let page_size = self.visible_height.saturating_sub(4);
                self.vertical_scroll = self
                    .vertical_scroll
                    .saturating_add(page_size)
                    .min(self.swap_processes_lines.len().saturating_sub(1));
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::PageUp => {
                let page_size = self.visible_height.saturating_sub(4);
                self.vertical_scroll = self.vertical_scroll.saturating_sub(page_size);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }

            KeyCode::Char('k') => self.change_unit(SizeUnits::KB),
            KeyCode::Char('m') => self.change_unit(SizeUnits::MB),
            KeyCode::Char('g') => self.change_unit(SizeUnits::GB),

            KeyCode::Char('a') => self.aggregated = !self.aggregated,
            KeyCode::Char('t') => self.cycle_theme(),
            KeyCode::Char('s') => self.sort_column = self.sort_column.next(),
            KeyCode::Char('h') => {
                if LINUX {
                    self.display_devices = !self.display_devices
                }
            }
            KeyCode::Left | KeyCode::Right => self.change_timout(key.code),

            _ => {}
        }
    }

    #[cfg(target_os = "windows")]
    fn on_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.quit(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(),

            // View switching
            KeyCode::Tab => self.cycle_view(),
            KeyCode::Char('1') => self.active_view = ActiveView::Swap,
            KeyCode::Char('3') => self.active_view = ActiveView::Gpu,
            KeyCode::Char('4') => self.active_view = ActiveView::Unified,

            KeyCode::Char('d') | KeyCode::Down => {
                self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::Char('u') | KeyCode::Up => {
                self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::End => {
                self.vertical_scroll = self.swap_processes_lines.len();
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::Home => {
                self.vertical_scroll = 0;
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }

            KeyCode::PageDown => {
                let page_size = self.visible_height.saturating_sub(4);
                self.vertical_scroll = self
                    .vertical_scroll
                    .saturating_add(page_size)
                    .min(self.swap_processes_lines.len().saturating_sub(1));
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::PageUp => {
                let page_size = self.visible_height.saturating_sub(4);
                self.vertical_scroll = self.vertical_scroll.saturating_sub(page_size);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }

            KeyCode::Char('k') => self.change_unit(SizeUnits::KB),
            KeyCode::Char('m') => self.change_unit(SizeUnits::MB),
            KeyCode::Char('g') => self.change_unit(SizeUnits::GB),

            KeyCode::Char('a') => self.aggregated = !self.aggregated,
            KeyCode::Char('t') => self.cycle_theme(),
            KeyCode::Char('s') => self.sort_column = self.sort_column.next(),
            KeyCode::Char('h') => {
                if LINUX {
                    self.display_devices = !self.display_devices
                }
            }
            KeyCode::Left | KeyCode::Right => self.change_timout(key.code),

            _ => {}
        }
    }

    fn change_unit(&mut self, unit: SizeUnits) {
        self.swap_size_unit = unit;
        if let Ok(info) = self.provider.get_swap_info(&self.swap_size_unit) {
            self.chart_info = info;
            self.swap_processes_lines = ui::process_list::create_process_lines(
                self.provider.as_ref(),
                &self.swap_size_unit,
                self.aggregated,
            );
        }
    }

    fn cycle_theme(&mut self) {
        self.current_theme = match self.current_theme {
            ThemeType::Default => ThemeType::Solarized,
            ThemeType::Solarized => ThemeType::Monokai,
            ThemeType::Monokai => ThemeType::Dracula,
            ThemeType::Dracula => ThemeType::Nord,
            ThemeType::Nord => ThemeType::Default,
        };
        self.swap_processes_lines = ui::process_list::create_process_lines(
            self.provider.as_ref(),
            &self.swap_size_unit,
            self.aggregated,
        );
    }

    fn change_timout(&mut self, action: KeyCode) {
        match action {
            KeyCode::Left => {
                self.timeout = self.timeout.saturating_sub(100).max(1);
            }
            KeyCode::Right => {
                self.timeout = self.timeout.saturating_add(100).min(10000);
            }
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
