mod app;
mod cli;
mod data;
mod theme;
mod ui;

#[cfg(test)]
mod smoke_tests;

use app::App;
use clap::Parser;
use data::ProcDataProvider;

fn main() -> color_eyre::Result<()> {
    let cli = cli::Cli::parse();
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new(Box::new(ProcDataProvider), cli.demo).run(terminal);
    ratatui::restore();
    result
}
