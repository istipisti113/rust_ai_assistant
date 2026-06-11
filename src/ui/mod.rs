use ratatui::Frame;
use ratatui::symbols::border;
use tokio_util::sync::CancellationToken;
use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Borders, Widget};
use ratatui::style::{Style, Stylize, Color};
use ratatui::text::{Line, Span, Text};
use ratatui::layout::Alignment;
use ratatui::widgets::{Block, Paragraph, Wrap};
use crossterm::event::{self, EnableMouseCapture, DisableMouseCapture, KeyCode};
use crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use serde_json::Value;
use std::{io::{self, Write}, vec};
use async_openai::{Client, config::OpenAIConfig};

use crate::config::send_message;

pub struct App<'a> {
    pub counter: Option<u32>,
    pub title: &'a str,
    pub running: bool,
    pub messages: Vec<Value>,
    pub model: Option<String>,
    pub input: String,
    pub client: Option<Client<OpenAIConfig>>,
    pub readable_messages: Vec<String>,
    pub reading_window_start: u16, //offset so scrolling can be implemented
}

impl<'a> App<'a>{
    pub fn new(title: &'a str) -> Self{
        App { counter: None, title, running: true, model: None, messages: vec![], 
            input: String::new(), client: None, readable_messages: vec![], reading_window_start: 0 }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal, token: CancellationToken) -> io::Result<()> {
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().await?;
        }
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        terminal.show_cursor()?;
        token.cancel();
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        //frame.render_widget(self, frame.area());
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(6),
            ]).split(frame.area());

        //let title = Line::from("hello");
        let header = Paragraph::new(Line::from(vec![
            Span::styled("rust ai assistant", Style::default().fg(Color::Blue))
        ])).block(Block::default().borders(Borders::ALL));

        let messages_parsed = self.readable_messages[(self.reading_window_start as usize)..]
            .iter().map(|mess| {
                Line::styled(mess, Style::default().fg(Color::Blue))
        }).collect::<Vec<_>>();

        let readable_messages = Paragraph::new(messages_parsed)
            .block(Block::default().borders(Borders::ALL));

        let input_field = Paragraph::new(Line::from(vec![
            Span::from(&self.input)
        ])).block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, chunks[0]);
        frame.render_widget(readable_messages, chunks[1]);
        frame.render_widget(input_field, chunks[2]);
    }

    async fn handle_events(&mut self) -> io::Result<()>{
        if let Some(key) = event::read()?.as_key_press_event(){
            match key.code {
                KeyCode::Esc => {
                    self.running = false
                }
                KeyCode::Char(character)=> {
                    self.input.push(character);
                }
                KeyCode::Enter => {
                    self.readable_messages.push(self.input.clone());
                    let reply = send_message(self).await.unwrap();
                    for line in reply.split("\n"){
                        self.readable_messages.push(line.to_owned());
                    }
                    self.input = String::new();
                }

                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Up => {
                    if self.reading_window_start!=0{
                        self.reading_window_start-=1
                    }
                }
                KeyCode::Down => {
                    self.reading_window_start+=1
                }
                _=>{}
            }
        }
        Ok(())
    }

    fn on_key(&mut self, c:char) {
        //unused, will get removed
        match c{
            'q' => {
                self.running = false;
            }
            _ => {}
        }
    }
}

impl<'a> Widget for &App<'a> {
    fn render(self, area: Rect, buf: &mut Buffer){
        //unused, will get removed
        let title = Line::from("Rust ai assistant");
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);
        let counter_text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.counter.unwrap_or(0).to_string().yellow(),
        ])]);
        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
