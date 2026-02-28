mod app;
mod data;
mod theme;
mod ui;

use app::App;
use data::ProcDataProvider;

fn main() -> color_eyre::Result<()> {
    let demo = std::env::args().any(|arg| arg == "--demo");
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new(Box::new(ProcDataProvider), demo).run(terminal);
    ratatui::restore();
    result
}
