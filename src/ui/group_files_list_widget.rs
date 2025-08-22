use super::FocusState;
use crate::group::Group;
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
pub(crate) struct GroupFilesListWidget {
    list_state: ListState,
    file_names: Vec<String>,
}

impl KeyPressConsumer for GroupFilesListWidget {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        match key_code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(down_res) = self.navigate_up() {
                    if !down_res {
                        return Action::NavigateBackward(ActiveWidget::Files);
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_down();
            }
            KeyCode::Esc => {
                return Action::NavigateBackward(ActiveWidget::Files);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                return Action::NavigateBackward(ActiveWidget::Files);
            }
            _ => {}
        }
        Action::Pass
    }
}

impl SelectableState for GroupFilesListWidget {
    fn select(&mut self, index: Option<usize>) {
        self.list_state.select(index);
    }

    fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    fn length(&self) -> usize {
        self.file_names.len()
    }
}

impl GroupFilesListWidget {
    // pub(crate) fn new<I, T>(file_names: Option<I>) -> Self where
    // I: IntoIterator<Item = T>,
    // T: ToString,
    // {
    //     let file_names: Vec<String> = file_names.into_iter().map(|s| s.to_string()).collect();
    //     Self {file_names, ..Self::default()}
    // }
    pub(crate) fn widget_type() -> ActiveWidget {
        ActiveWidget::Files
    }

    pub(crate) fn from_group(group: Option<&Group>) -> Self {
        let mut new = Self::default();
        new.set_filenames(group);
        new
    }

    fn set_filenames(&mut self, group: Option<&Group>) {
        self.file_names.clear();
        self.list_state = ListState::default();
        if let Some(group) = group {
            self.file_names
                .extend(group.files.iter().map(|file| file.file_name.clone()));
        }
    }

    pub(crate) fn render(&mut self, frame: &mut Frame, area: Rect, focus: FocusState) {
        // Group files
        let group_files_items: Vec<_> = self
            .file_names
            .iter()
            .map(|file_name| {
                ListItem::new(Line::from(vec![Span::styled(
                    file_name.clone(),
                    Style::default(),
                )]))
            })
            .collect();

        let border_style = Style::default().fg(focus.border_color());

        let group_files = List::new(group_files_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(focus.text_color()))
                    .title("Files")
                    .border_type(BorderType::Plain)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .bg(focus.sel_color())
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(group_files, area, &mut self.list_state);
    }
}
