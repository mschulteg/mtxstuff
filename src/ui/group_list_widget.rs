use super::FocusState;
use crate::ui::selectable_state::SelectableState;
use crate::ui::Action;
use crate::ui::ActiveWidget;
use crate::ui::KeyPressConsumer;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

#[derive(Clone, Default)]
pub(crate) struct GroupListWidget {
    list_state: ListState,
    num_groups: usize,
}

impl KeyPressConsumer for GroupListWidget {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        match key_code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up();
                return Action::LoadGroup;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_down();
                return Action::LoadGroup;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                return Action::NavigateForward(ActiveWidget::Groups);
            }
            _ => {}
        }
        Action::Pass
    }
}

impl SelectableState for GroupListWidget {
    fn select(&mut self, index: Option<usize>) {
        self.list_state.select(index);
    }

    fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    fn length(&self) -> usize {
        self.num_groups
    }
}

impl GroupListWidget {
    pub(crate) fn new(num_groups: usize) -> Self {
        let mut new = Self {
            num_groups,
            ..Self::default()
        };
        if num_groups != 0 {
            new.select(Some(0));
        }
        new
    }

    pub(crate) fn widget_type() -> ActiveWidget {
        ActiveWidget::Groups
    }

    //TODO: remove groups, maybe keep a copy
    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, focus: FocusState) {
        let border_style = Style::default().fg(focus.border_color());
        let groupnames_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(focus.text_color()))
            .title("Groups")
            .border_type(BorderType::Plain)
            .border_style(border_style);

        let groupnames_items: Vec<_> = (0..self.num_groups)
            .map(|idx| {
                ListItem::new(Line::from(vec![Span::styled(
                    format!("Group #{}", idx.to_string()),
                    Style::default(),
                )]))
            })
            .collect();

        let groupnames_list = List::new(groupnames_items)
            .block(groupnames_block)
            .highlight_style(
                Style::default()
                    .bg(focus.sel_color())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(groupnames_list, area, &mut self.list_state);
    }
}
