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
use serde_json::{json, Value};
use std::io::Read;
use std::time::Duration;
use std::{io::{self, Write}, vec};
use async_openai::{Client, config::OpenAIConfig};

use std::sync::{Arc};
use tokio::sync::Mutex;

use crate::config::send_message;

pub struct App<'a> {
    pub counter: Option<u32>,
    pub title: &'a str,
    pub running: bool,
    pub messages: Arc<Mutex<Vec<Value>>>,
    pub model: Option<String>,
    pub input: String,
    pub client: Option<Client<OpenAIConfig>>,
    pub readable_messages: Arc<Mutex<Vec<String>>>,
    pub reading_window_start: u16, //offset so scrolling can be implemented
    pub token: CancellationToken,
}

impl<'a> App<'a>{
    pub fn new(title: &'a str, token: CancellationToken) -> Self{
        App { counter: None, title, running: true, model: None, messages: Arc::new(Mutex::new(vec![])), 
            input: String::new(), client: None, readable_messages: Arc::new(Mutex::new(vec![])), reading_window_start: 0, token }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal, token: CancellationToken) -> io::Result<()> {
        while self.running {
            terminal.draw(|frame| {
                self.draw(frame);
            }).unwrap();
            //eprintln!("draw");
            if event::poll(Duration::from_millis(100)).unwrap(){
                self.handle_events().await?;
                //eprintln!("handle_event");
            }
        }
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        terminal.show_cursor()?;
        token.cancel();
        Ok(())
    }

    fn draw(&mut self, frame: &'_ mut Frame<'_>) {
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

        let readable = tokio::task::block_in_place(|| {self.readable_messages.blocking_lock() });
        self.reading_window_start = self.reading_window_start.min(readable.len() as u16);
        let messages_parsed = readable[(self.reading_window_start as usize)..]
            .iter().map(|mess| {
                Line::styled(mess, Style::default())
        }).collect::<Vec<Line>>();

        let readable_messages = Paragraph::new(messages_parsed)
            .block(Block::default().borders(Borders::ALL));

        let input_field = Paragraph::new(Line::from(vec![
            Span::from(&self.input)
        ])).block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, chunks[0]);
        frame.render_widget(readable_messages, chunks[1]);
        frame.render_widget(input_field, chunks[2]);
    }

    //async fn message_send(&self, readable: Vec<String>){
    //    let reply = send_message(self).await.unwrap();
    //    for line in reply.split("\n"){
    //        self.readable_messages.lock().await.push(line.to_owned());
    //    }
    //}

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
                    let mut readable_asdfasdf = tokio::task::block_in_place(||self.readable_messages.blocking_lock());
                    readable_asdfasdf.push(self.input.clone());
                    let mut raw_asdfasdf = tokio::task::block_in_place(||self.messages.blocking_lock());
                    let a =json!({"role": "user", "content": self.input.trim()});
                    //eprintln!("{:?}", &a);
                    raw_asdfasdf.push(a);
                    let client = self.client.as_ref().unwrap().clone();

                    let model = self.model.clone();
                    let model = model.unwrap();
                    let messages = Arc::clone(&self.messages);

                    let readable = Arc::clone(&self.readable_messages);
                    self.input = String::new();

                    tokio::task::spawn(async move {
                        let mut messages_guard = messages.lock().await;
                        let mut readable_guard = readable.lock().await;
                        let (mut raw, mut readable) = send_message(
                            &client.clone(), &mut messages_guard, &model.clone())
                        .await.unwrap();
                        messages_guard.append(&mut raw);
                        for asdf in &readable{
                            for line in asdf.split("\\n"){
                                let a = line.to_owned();
                                readable_guard.push(a);
                            }
                        }
                    });
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
