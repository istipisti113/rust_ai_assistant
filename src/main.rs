mod config;
mod ui;

use crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use crossterm::event::{self, EnableMouseCapture, DisableMouseCapture, KeyCode};
use ratatui::{DefaultTerminal, Terminal, prelude::CrosstermBackend};

use crate::{ui::App};
use std::{io::{self, Write}, vec};

use tokio_util::sync::CancellationToken;



#[tokio::main]
async fn main() -> anyhow::Result<()>{

    match dotenvy::from_filename("/home/istipisti113/config/variables/raa.env") {
        Ok(_a) => {}
        Err(_e) => {
            eprintln!(".env file could not be loaded.");
            return Ok(());
        }
    };
    dotenvy::dotenv().ok();
    let (base_url, api_key, model) = config::get_config();
    let client = config::create_client::<serde_json::Value>(&base_url, &api_key);

    let token = CancellationToken::new();
    let mut app = App::new("raa", token.clone());
    app.model = Some(model);
    app.client = Some(client);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend =  CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    tokio::select! {
        _= app.run(&mut terminal, token.clone()) => {}
        _= token.cancelled() =>{}
    }
    Ok(())
}
