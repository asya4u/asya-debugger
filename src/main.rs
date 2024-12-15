use std::thread;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize,
    widgets::{Block, Paragraph, Wrap},
    Frame,
};
use tungstenite::{connect, Message};

const TEMPLATE_BEGIN: &str = r#"{"General":{"action":""#;
const TEMPLATE_END: &str = r#""}}"#;

fn main() -> Result<(), std::io::Error> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;
    let mut state = State::new();

    let url = state.url.clone();
    let (mut socket, response) = connect(url).expect("Can't connect");
    let mut http_response = String::new();
    http_response.push_str(response.status().to_string().as_str());
    http_response.push_str("\n\n");
    for (key, value) in response.headers() {
        http_response.push_str(key.as_str());
        http_response.push_str(": ");
        http_response.push_str(value.to_str().unwrap());
        http_response.push('\n');
    }

    state.response = http_response;

    loop {
        terminal.draw(|f| ui(&mut state, f, f.area()))?;
        if let Event::Key(key) = event::read()? {
            if key.modifiers.contains(KeyModifiers::CONTROL)
            {
                match key.code {
                    KeyCode::Char('c') => {
                        socket.close(None).expect("bebra");
                        break;
                    }
                    KeyCode::Char('w') => {
                        state.query = state.cached_query.clone();
                        continue;
                    }
                    _ => {}
                }
            }
            match key.code {
                KeyCode::Char(c) => {
                    state.query.push(c);
                }
                KeyCode::Backspace => {
                    state.query.pop();
                }
                KeyCode::Tab => {
                    match state.mode {
                        InputMode::URL => {
                            state.mode = InputMode::Query;
                        }
                        InputMode::Query => {
                            state.mode = InputMode::URL;
                        }
                    }
                }
                KeyCode::Enter => {
                    if state.mode == InputMode::URL {
                        continue;
                    }
                    let req_text = format!("{}{}{}", TEMPLATE_BEGIN, state.query, TEMPLATE_END);
                    state.request = req_text.clone();
                    state.cached_query = state.query.clone();
                    state.query = "".to_string();

                    socket.send(Message::Text(req_text)).unwrap();
                    loop {
                        if !socket.can_read() {
                            break;
                        }
                        let read = socket.read();
                        match read {
                            Ok(Message::Text(text)) => {
                                state.response = text;
                                break;
                            }
                            Err(err) => {
                                state.response = err.to_string();
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum InputMode{
    URL,
    Query
}

struct State {
    url: String,
    query: String,
    cached_query: String,
    request: String,
    response: String,
    mode: InputMode
}

impl State {
    fn new() -> Self {
        Self {
            url: "ws://127.0.0.1:3001/ws".to_string(),
            query: "".to_string(),
            cached_query: "".to_string(),
            request: "".to_string(),
            response: "".to_string(),
            mode: InputMode::Query
        }
    }
}

fn ui(state: &mut State, f: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(6),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .flex(Flex::SpaceBetween)
    .split(area);
    let input_chunks =
        Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).split(chunks[0]);
    let req_res_chunks =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

    // url field (currently does nothing)
    {
        let mut block = Block::bordered().title("URL");
        let mut text = Paragraph::new(state.url.as_str());
        match state.mode {
            InputMode::URL => {
                block = block.yellow();
                text = text.yellow();
            }
            InputMode::Query => {
                block = block.white();
                text = text.white();
            }
        }
        let input = text.block(block);
        f.render_widget(input, input_chunks[0]);
    }
    // query field
    {
        let mut block = Block::bordered().title("Input action");
        let mut text = Paragraph::new(state.query.as_str());
        match state.mode {
            InputMode::URL => {
                block = block.white();
                text = text.white();
            }
            InputMode::Query => {
                block = block.yellow();
                text = text.yellow();
            }
        }
        let input = text.block(block);
        f.render_widget(input, input_chunks[1]);
    }

    // request block
    {
        let block = Block::bordered().green().title("Request");
        let text = Paragraph::new(state.request.as_str())
            .green()
            .wrap(Wrap::default());
        let request = text.block(block);
        f.render_widget(request, req_res_chunks[0]);
    }

    // response block
    {
        let block = Block::bordered().green().title("Response");
        let text = Paragraph::new(state.response.as_str())
            .green()
            .wrap(Wrap::default());
        let response = text.block(block);
        f.render_widget(response, req_res_chunks[1]);
    }

    // footer
    {
        let text = match state.mode {
            InputMode::URL => "<Ctrl + C> - quit, <Tab> - switch to query, <Ctrl + W> - cached input, <Enter> - send",
            InputMode::Query => "<Ctrl + C> - quit, <Tab> - switch to URL, <Ctrl + W> - cached input, <Enter> - send",
        };
        let footer = Paragraph::new(text).dim();
        f.render_widget(footer, chunks[2]);
    }
}
