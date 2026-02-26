mod app;
mod data;
mod theme;
mod ui;

use app::App;
use data::ProcDataProvider;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new(Box::new(ProcDataProvider)).run(terminal);
    ratatui::restore();
    result
}
