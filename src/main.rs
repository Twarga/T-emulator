mod config;
mod adapters;
mod scanner;

use anyhow::Result;

slint::include_modules!();

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let app = App::new()?;
    app.run()?;
    Ok(())
}
