use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{sink::SinkExt, stream::StreamExt};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::io;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

struct App {
    messages: Vec<String>,
    input: String,
    tx: mpsc::Sender<String>,
}

impl App {
    fn new(tx: mpsc::Sender<String>) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            tx,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup WebSocket
    let (ws_stream, _) = connect_async("ws://127.0.0.1:3000/ws").await?;
    let (mut write, mut read) = ws_stream.split();

    // Channel for sending messages from UI to WS
    let (tx, mut rx) = mpsc::channel::<String>(32);

    // Forward channel messages to WebSocket
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App State
    let mut app = App::new(tx);
    let mut running = true;

    // Main Loop
    while running {
        terminal.draw(|f| ui(f, &app))?;

        tokio::select! {
            // Handle Incoming WS Messages
            val = read.next() => {
                if let Some(Ok(msg)) = val {
                    match msg {
                        Message::Text(text) => {
                             if text == "\n" {
                                 app.messages.push(String::new());
                             } else {
                                 if let Some(last) = app.messages.last_mut() {
                                     last.push_str(&text);
                                 } else {
                                     app.messages.push(text);
                                 }
                             }
                        }
                        _ => {}
                    }
                } else {
                    break;
                }
            }
            // Handle User Input
            _ = async {}, if event::poll(std::time::Duration::from_millis(16))? => {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => running = false,
                            KeyCode::Char(c) => app.input.push(c),
                            KeyCode::Backspace => { app.input.pop(); },
                            KeyCode::Enter => {
                                let msg = app.input.drain(..).collect::<String>();
                                app.messages.push(format!("You: {}", msg));
                                // Prepare a new line for the response
                                app.messages.push(String::new()); 
                                if let Err(_) = app.tx.send(msg).await {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // Restore Terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|m| ListItem::new(Line::from(vec![Span::raw(m)])))
        .collect();

    let messages_widget = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Chat"));
    
    f.render_widget(messages_widget, chunks[0]);

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .wrap(Wrap { trim: true });
    
    f.render_widget(input, chunks[1]);
}
