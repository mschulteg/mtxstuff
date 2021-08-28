use super::selectable_state::SelectableState;
use super::Action;
use super::ActiveWidget;
use super::KeyPressConsumer;
use super::{centered_rect, centered_rect_with_height};
use crossterm::event::KeyCode;
use std::io::Stdout;
use tui::layout::Constraint;
use tui::layout::Direction;
use tui::layout::Layout;
use tui::widgets::Clear;
use tui::widgets::Paragraph;
use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

const SEL_COLOR: Color = Color::LightCyan;

// TODO: Frame<B: Backend>
pub(crate) trait PopupRender {
    fn render_widget(
        &mut self,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        highlight: bool,
    );
}

pub(crate) trait Popup: PopupRender + KeyPressConsumer {}
impl<T: PopupRender + KeyPressConsumer> Popup for T {}

//#[derive(Clone)]
pub(crate) struct PopupRenderer {
    pub(crate) popup_stack: Vec<Box<dyn Popup>>,
}

impl PopupRender for PopupRenderer {
    fn render_widget(
        &mut self,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        highlight: bool,
    ) {
        let stack_len = self.popup_stack.len();
        for (i, popup) in self.popup_stack.iter_mut().enumerate() {
            popup.render_widget(frame, area, highlight && i == stack_len - 1);
        }
    }
}

impl KeyPressConsumer for PopupRenderer {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        if let Some(active_popup) = self.popup_stack.last_mut() {
            log::info!("HERE");
            active_popup.process_key(key_code)
        } else {
            Action::Pass
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct CommandPopup {
    pub(crate) commands: Vec<String>,
    pub(crate) list_state: ListState,
}

impl CommandPopup {
    fn render<B: tui::backend::Backend>(
        &mut self,
        frame: &mut Frame<B>,
        area: Rect,
        highlight: bool,
    ) {
        let border_style = if highlight {
            Style::default().fg(SEL_COLOR)
        } else {
            Style::default()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Saved to mtx_commands.sh")
            .border_type(BorderType::Thick)
            .border_style(border_style);

        let items: Vec<_> = {
            self.commands
                .iter()
                .map(|item| {
                    ListItem::new(Spans::from(vec![Span::styled(
                        item.clone(),
                        Style::default(),
                    )]))
                })
                .collect()
        };
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(SEL_COLOR)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
        let area = centered_rect(80, 80, area);
        frame.render_widget(Clear, area);
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}

impl PopupRender for CommandPopup {
    fn render_widget(
        &mut self,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        highlight: bool,
    ) {
        self.render(frame, area, highlight);
    }
}

impl KeyPressConsumer for CommandPopup {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        match key_code {
            KeyCode::Up => {
                self.navigate_up();
            }
            KeyCode::Down => {
                self.navigate_down();
            }
            KeyCode::Esc => {
                log::info!("We are here: {:?}", self.selected());
                return Action::NavigateBackward(ActiveWidget::Popup);
            }
            _ => {}
        }
        Action::Pass
    }
}

impl SelectableState for CommandPopup {
    fn select(&mut self, index: Option<usize>) {
        self.list_state.select(index);
    }

    fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    fn length(&self) -> usize {
        self.commands.len()
    }
}
#[derive(Clone, Default)]
pub(crate) struct EditPopup {
    pub(crate) input: String,
}

use unicode_width::UnicodeWidthStr;

impl EditPopup {
    fn render<B: tui::backend::Backend>(
        &mut self,
        frame: &mut Frame<B>,
        area: Rect,
        highlight: bool,
    ) {
        let area = centered_rect_with_height(50, 3, area);
        let border_style = if highlight {
            Style::default().fg(SEL_COLOR)
        } else {
            Style::default()
        };
        let input = Paragraph::new(self.input.as_ref())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Edit string")
                    .border_type(BorderType::Thick)
                    .border_style(border_style),
            );
        frame.set_cursor(
            // Put cursor past the end of the input text
            area.x + self.input.width() as u16 + 1,
            // Move one line down, from the border to the input line
            area.y + 1,
        );
        frame.render_widget(Clear, area);
        frame.render_widget(input, area);
    }
}

impl PopupRender for EditPopup {
    fn render_widget(
        &mut self,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        highlight: bool,
    ) {
        self.render(frame, area, highlight);
    }
}

impl KeyPressConsumer for EditPopup {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        match key_code {
            KeyCode::Esc => {
                return Action::EditStringResult(None);
            }
            KeyCode::Enter => {
                return Action::EditStringResult(Some(self.input.clone()));
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(char) => {
                self.input.push(char);
                log::info!("input is now {:?}", self.input);
            }
            _ => {}
        }
        Action::Pass
    }
}
