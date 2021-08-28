use crate::ui::selectable_state::SelectableState;
use crate::ui::Action;
use crate::ui::ActiveWidget;
use crate::ui::KeyPressConsumer;

use crossterm::event::KeyCode;
use tui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

const SEL_COLOR: Color = Color::LightCyan;

#[derive(Clone, Default)]
pub(crate) struct GroupListWidget {
    list_state: ListState,
    num_groups: usize,
}

impl KeyPressConsumer for GroupListWidget {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        match key_code {
            KeyCode::Up => {
                self.navigate_up();
                return Action::LoadGroup;
            }
            KeyCode::Down => {
                self.navigate_down();
                return Action::LoadGroup;
            }
            KeyCode::Right => {
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
    pub(crate) fn render<B: tui::backend::Backend>(
        &mut self,
        frame: &mut Frame<B>,
        area: Rect,
        active_widget: ActiveWidget,
    ) {
        let border_style = if Self::widget_type() == active_widget {
            Style::default().fg(SEL_COLOR)
        } else {
            Style::default()
        };
        let groupnames_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Groups")
            .border_type(BorderType::Plain)
            .border_style(border_style);

        let groupnames_items: Vec<_> = (0..self.num_groups)
            .map(|idx| {
                ListItem::new(Spans::from(vec![Span::styled(
                    format!("Group #{}", idx.to_string()),
                    Style::default(),
                )]))
            })
            .collect();

        let groupnames_list = List::new(groupnames_items)
            .block(groupnames_block)
            .highlight_style(
                Style::default()
                    .bg(SEL_COLOR)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(groupnames_list, area, &mut self.list_state);
    }
}
