use std::io;
use std::env;
use dotenv::dotenv;
use env_logger::Builder;
use log::LevelFilter;
use std::fs::File;
use env_logger::Target;

use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    app::{App, AppResult},
    event::{Event, EventHandler},
    handler::handle_key_events,
    tui::Tui,
};

pub mod app;
pub mod event;
pub mod handler;
pub mod tui;
pub mod ui;

#[tokio::main]
async fn main() -> AppResult<()> {
    // Load .env file
    dotenv().ok();

    // Setup logging if in debug mode
    if env::var("REEDY_ENV").unwrap_or_default() == "DEBUG" {
        let log_path = App::get_log_path();
        println!("Log file location: {}", log_path.display());
        let file = File::create(log_path).unwrap();
        
        Builder::new()
            .target(Target::Pipe(Box::new(file)))
            .filter_level(LevelFilter::Debug)
            .init();
    }

    // Create an application.
    let mut app = App::new();

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next().await? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app).await?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
