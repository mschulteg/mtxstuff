use std::io::Stdout;
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
pub(crate) trait Popup {
    fn render_widget(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect);
}

//#[derive(Clone)]
pub(crate) struct PopupRenderer {
    pub(crate) popup_stack: Vec<Box<dyn Popup>>,
}

impl PopupRenderer {
    pub(crate) fn render_stuff(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        for popup in self.popup_stack.iter_mut() {
            popup.render_widget(frame, area);
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct CommandPopup {
    pub(crate) commands: Vec<String>,
    pub(crate) list_state: ListState,
}

impl CommandPopup {
    pub(crate) fn render(&mut self) -> List {
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Saved to mtx_commands.sh")
            .border_type(BorderType::Thick);

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
        list
    }
}

impl Popup for CommandPopup {
    fn render_widget(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        frame.render_widget(self.render(), area);
    }
}
