use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{sink::SinkExt, stream::StreamExt};
use ratatui::{
    prelude::*,
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap},
};
use shared::{Role, ServerMessage, Message as SharedMessage, ClientMessage};
use std::io;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};

struct App {
    messages: Vec<SharedMessage>,
    current_response: String,
    current_model: String,
    input: String,
    tx: mpsc::Sender<String>,
    // Modal State
    show_model_selector: bool,
    available_models: Vec<String>,
    selected_model_index: usize,
}

impl App {
    fn new(tx: mpsc::Sender<String>) -> Self {
        Self {
            messages: Vec::new(),
            current_response: String::new(),
            current_model: "Unknown".to_string(),
            input: String::new(),
            tx,
            show_model_selector: false,
            available_models: Vec::new(),
            selected_model_index: 0,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup WebSocket
    let (ws_stream, _) = connect_async("ws://127.0.0.1:3001/ws").await?;
    let (mut write, mut read) = ws_stream.split();

    // Channel for sending messages from UI to WS
    let (tx, mut rx) = mpsc::channel::<String>(32);

    // Forward channel messages to WebSocket
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(WsMessage::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Event Channel
    let (tx_event, mut rx_event) = mpsc::channel(100);
    
    // Spawn Event Task
    tokio::task::spawn_blocking(move || {
        loop {
            // Poll with timeout to allow checking for exit signal if we had one
            if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(e) = event::read() {
                    if tx_event.blocking_send(e).is_err() {
                        break;
                    }
                }
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
    let mut tick_rate = tokio::time::interval(std::time::Duration::from_millis(30));

    // Main Loop
    while running {
        terminal.draw(|f| ui(f, &app))?;

        tokio::select! {
             // Tick for smooth UI (optional but good practice)
            _ = tick_rate.tick() => {}

            // Handle Incoming WS Messages
            val = read.next() => {
                if let Some(Ok(msg)) = val {
                    match msg {
                        WsMessage::Text(text) => {
                             if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                match server_msg {
                                    ServerMessage::History(history) => {
                                        app.messages = history.messages;
                                        app.current_model = history.current_model;
                                    }
                                    ServerMessage::Token(token) => {
                                        app.current_response.push_str(&token);
                                    }
                                    ServerMessage::EndOfMessage => {
                                        app.messages.push(SharedMessage {
                                            role: Role::Assistant,
                                            content: app.current_response.clone(),
                                        });
                                        app.current_response.clear();
                                    }
                                    ServerMessage::ModelChanged(new_model) => {
                                         app.current_model = new_model;
                                         app.messages.push(SharedMessage {
                                             role: Role::Assistant,
                                             content: format!("System: Model switched to {}", app.current_model),
                                         });
                                    }
                                    ServerMessage::AvailableModels(models) => {
                                        app.available_models = models;
                                        app.available_models.sort();
                                    }
                                    ServerMessage::Error(err) => {
                                        app.messages.push(SharedMessage {
                                            role: Role::Assistant,
                                            content: format!("System Error: {}", err),
                                        });
                                    }
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
            Some(evt) = rx_event.recv() => {
                if let Event::Key(key) = evt {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                app.show_model_selector = !app.show_model_selector;
                            }
                            // Modal Handling
                            KeyCode::Up if app.show_model_selector => {
                                if app.selected_model_index > 0 {
                                    app.selected_model_index -= 1;
                                }
                            }
                            KeyCode::Down if app.show_model_selector => {
                                if !app.available_models.is_empty() && app.selected_model_index < app.available_models.len() - 1 {
                                    app.selected_model_index += 1;
                                }
                            }
                            KeyCode::Enter if app.show_model_selector => {
                                if let Some(model) = app.available_models.get(app.selected_model_index) {
                                    let client_msg = ClientMessage::SetModel(model.clone());
                                     if let Ok(json) = serde_json::to_string(&client_msg) {
                                        if app.tx.send(json).await.is_err() {
                                            break;
                                        }
                                    }
                                    app.show_model_selector = false;
                                }
                            }
                            KeyCode::Esc if app.show_model_selector => {
                                app.show_model_selector = false;
                            }
                            
                            // Normal Handling
                            KeyCode::Esc => running = false,
                            KeyCode::Char(c) if !app.show_model_selector => app.input.push(c),
                            KeyCode::Backspace if !app.show_model_selector => { app.input.pop(); },
                            KeyCode::Enter if !app.show_model_selector => {
                                let msg = app.input.drain(..).collect::<String>();
                                
                                // Check for slash commands
                                if msg.starts_with("/model ") {
                                    let model_name = msg.trim_start_matches("/model ").to_string();
                                    let client_msg = ClientMessage::SetModel(model_name);
                                    if let Ok(json) = serde_json::to_string(&client_msg) {
                                        if let Err(_) = app.tx.send(json).await {
                                            break;
                                        }
                                    }
                                } else if !msg.is_empty() {
                                    // Normal message
                                     // Optimistic update
                                    app.messages.push(SharedMessage {
                                        role: Role::User,
                                        content: msg.clone(),
                                    });
                                    
                                    let client_msg = ClientMessage::Text(msg);
                                     if let Ok(json) = serde_json::to_string(&client_msg) {
                                        if let Err(_) = app.tx.send(json).await {
                                            break;
                                        }
                                     }
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

    let mut list_items: Vec<ListItem> = app
        .messages
        .iter()
        .map(|m| {
            let prefix = match m.role {
                Role::User => "You: ",
                Role::Assistant => "Assistant: ",
            };
            let content = format!("{}{}", prefix, m.content);
            ListItem::new(Line::from(vec![Span::raw(content)]))
        })
        .collect();
    
    // Add current streaming response if any
    if !app.current_response.is_empty() {
        let content = format!("Assistant: {}", app.current_response);
        list_items.push(ListItem::new(Line::from(vec![Span::raw(content)])));
    }

    let messages_widget = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(format!("Chat - Model: {}", app.current_model)));
    
    f.render_widget(messages_widget, chunks[0]);

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .wrap(Wrap { trim: true });
    
    f.render_widget(input, chunks[1]);

    // Render Modal
    if app.show_model_selector {
        let block = Block::default().title("Select Model").borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area); // Clear background
        f.render_widget(block.clone(), area);

        let items: Vec<ListItem> = app.available_models.iter().enumerate().map(|(i, model)| {
            let style = if i == app.selected_model_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![Span::styled(model, style)]))
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::NONE).padding(Padding::new(1, 1, 1, 1)));
        
        let inner_area = block.inner(area);
        f.render_widget(list, inner_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
