use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend, layout::{Constraint, Flex, Layout, Rect}, style::Stylize, text::ToText, widgets::{Block, Paragraph, Wrap}, Frame
};
use tungstenite::{connect, Message};

const URL: &str = "ws://127.0.0.1:3001/ws";
const TEMPLATE_BEGIN: &str = r#"{"General":{"action":""#;
const TEMPLATE_END: &str = r#""}}"#;

fn main() -> Result<(), std::io::Error> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;
    let mut state = State::new();

    let (mut socket, response) = connect(URL).expect("Can't connect");

    loop {
        terminal.draw(|f| ui(&mut state, f, f.area()))?;
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    socket.close(None).expect("bebra");
                    break;
                }
                KeyCode::Char(c) => {
                    state.query.push(c);
                }
                KeyCode::Backspace => {
                    state.query.pop();
                }
                KeyCode::Tab => {
                    state.query = state.cached_query.clone();
                }
                KeyCode::Enter => {
                    let req_text = format!("{}{}{}", TEMPLATE_BEGIN, state.query, TEMPLATE_END);
                    state.request = req_text.clone();
                    state.cached_query = state.query.clone();
                    state.query = "".to_string();

                    socket.send(Message::Text(req_text)).unwrap();
                    loop{
                        let read = socket.read();
                        match read{
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

struct State {
    query: String,
    cached_query: String,
    request: String,
    response: String
}

impl State {
    fn new() -> Self {
        Self {
            query: "".to_string(),
            cached_query: "".to_string(),
            request: "".to_string(),
            response: "".to_string()
        }
    }
}

fn ui(state: &mut State, f: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .flex(Flex::SpaceBetween)
    .split(area);

    let req_res_chunks =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

    // input field
    {
        let block = Block::bordered().yellow().title("Input action");
        let text = Paragraph::new(state.query.as_str()).yellow();
        let input = text.block(block);
        f.render_widget(input, chunks[0]);
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
        let text = Paragraph::new(state.response.as_str()).green().wrap(Wrap::default());
        let response = text.block(block);
        f.render_widget(response, req_res_chunks[1]);
    }

    // footer
    {
        let footer = Paragraph::new("<Q> - quit, <Tab> - cached query, <Enter> - send").dim();
        f.render_widget(footer, chunks[2]);
    }
}
