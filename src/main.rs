mod api;
mod config;
mod tui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tui::run()?;
    Ok(())
}
