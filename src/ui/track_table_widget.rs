use super::FocusState;
use crate::group::{Group, GroupKey};
use crate::ui::selectable_state::SelectableState;
use crate::ui::Action;
use crate::ui::ActiveWidget;
use crate::ui::KeyPressConsumer;

use crossterm::event::KeyCode;
use tui::layout::Constraint;
use tui::widgets::Cell;
use tui::widgets::Row;
use tui::widgets::Table;
use tui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, TableState},
    Frame,
};

#[derive(Clone, Default)]
pub(crate) struct TrackTableWidget {
    table_state: TableState,
    pub(crate) selected_col: Option<usize>,
    keys_orig: Vec<GroupKey>,
    keys_copy: Vec<GroupKey>,
}

impl KeyPressConsumer for TrackTableWidget {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        match key_code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(down_res) = self.navigate_down() {
                    if !down_res {
                        return Action::NavigateForward(ActiveWidget::Details);
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(selected_col) = self.selected_col {
                    if selected_col < 4 {
                        self.selected_col = Some(selected_col + 1);
                    }
                } else {
                    self.selected_col = Some(0);
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(selected_col) = self.selected_col {
                    if selected_col > 0 {
                        self.selected_col = Some(selected_col - 1);
                    } else {
                        self.selected_col = None;
                    }
                } else {
                    return Action::NavigateBackward(ActiveWidget::Details);
                }
            }
            KeyCode::Esc => {
                if self.selected_col.is_some() {
                    self.selected_col = None;
                } else {
                    return Action::NavigateBackward(ActiveWidget::Details);
                }
            }
            KeyCode::Enter => {
                if let Some(selected_col) = self.selected_col {
                    let sel_row = self.selected().unwrap();
                    let gkey = self.keys_copy.get_mut(sel_row).unwrap();
                    match selected_col {
                        0 => {
                            if let Some(ref lang) = gkey.language {
                                return Action::EditString(lang.clone());
                            } else {
                                return Action::EditString("".to_owned());
                            }
                        }
                        1 => {
                            if let Some(ref name) = gkey.name {
                                return Action::EditString(name.clone());
                            } else {
                                return Action::EditString("".to_owned());
                            }
                        }
                        2 => {
                            gkey.default = !gkey.default;
                        }
                        3 => {
                            gkey.forced = !gkey.forced;
                        }
                        4 => {
                            gkey.enabled = !gkey.enabled;
                        }
                        _ => {}
                    }
                } else {
                    self.selected_col = Some(0);
                }
            }
            KeyCode::Delete => {
                if let Some(selected_col) = self.selected_col {
                    let sel_row = self.selected().unwrap();
                    let gkey = self.keys_copy.get_mut(sel_row).unwrap();
                    match selected_col {
                        0 => {
                            if gkey.language.is_some() {
                                gkey.language = None
                            }
                        }
                        1 => {
                            if gkey.name.is_some() {
                                gkey.name = None
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Action::Pass
    }
}

impl SelectableState for TrackTableWidget {
    fn select(&mut self, index: Option<usize>) {
        self.table_state.select(index);
    }

    fn selected(&self) -> Option<usize> {
        self.table_state.selected()
    }

    fn length(&self) -> usize {
        self.keys_orig.len()
    }

    fn selectable(&self) -> bool {
        self.selected_col.is_none()
    }

    fn leave(&mut self) {
        self.selected_col = None;
        self.select(None);
    }
}

impl TrackTableWidget {
    pub(crate) fn from_group(group: Option<&Group>) -> Self {
        let keys_orig = if let Some(sel_group) = group {
            sel_group.key.clone()
        } else {
            Vec::<GroupKey>::new()
        };
        let keys_copy = keys_orig.clone();
        Self {
            keys_orig,
            keys_copy,
            ..Self::default()
        }
    }

    pub(crate) fn widget_type() -> ActiveWidget {
        ActiveWidget::Details
    }

    pub(crate) fn get_keys_copy(&self) -> &[GroupKey] {
        &self.keys_copy
    }

    pub(crate) fn get_keys_copy_mut(&mut self) -> &mut [GroupKey] {
        &mut self.keys_copy
    }

    pub(crate) fn render<B: tui::backend::Backend>(
        &mut self,
        frame: &mut Frame<B>,
        area: Rect,
        focus: FocusState,
    ) {
        let highlight_style = Style::default()
            .bg(focus.sel_color())
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD);

        let idx_col_to_string = |key_row: &GroupKey, idx_col| match idx_col {
            //0 => if key_row.language != "und" {Some(key_row.language.clone())} else {None},
            0 => key_row.language.clone(),
            1 => key_row.name.clone(),
            2 => Some(if key_row.default {
                "[X]".to_owned()
            } else {
                "[ ]".to_owned()
            }),
            3 => Some(if key_row.forced {
                "[X]".to_owned()
            } else {
                "[ ]".to_owned()
            }),
            4 => Some(if key_row.enabled {
                "[X]".to_owned()
            } else {
                "[ ]".to_owned()
            }),
            _ => None,
        };

        let create_style = |item: Option<&str>, idx_col: usize, idx_row: usize| {
            let mut style = Style::default();
            if let Some(sel_col) = self.selected_col {
                if sel_col == idx_col && self.selected().unwrap() == idx_row {
                    style = style
                        .bg(focus.sel_color())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD);
                }
            }
            let keyrow = self.keys_orig.get(idx_row).unwrap();
            if idx_col_to_string(keyrow, idx_col).as_deref() != item {
                style = style.add_modifier(Modifier::ITALIC);
            }
            if item.is_none() {
                style = style.fg(Color::DarkGray)
            }
            style
        };

        let group_detail_rows: Vec<Row> = self
            .keys_copy
            .iter()
            .enumerate()
            .map(|(idx_row, keyrow)| {
                Row::new((0..5).into_iter().map(|idx_col| {
                    let item = idx_col_to_string(keyrow, idx_col);
                    let text = if let Some(ref item) = item {
                        item.clone()
                    } else {
                        "unset".to_owned()
                    };
                    let cell = Cell::from(Span::raw(text));
                    cell.style(create_style(item.as_deref(), idx_col, idx_row))
                }))
            })
            .collect();
        let border_style = Style::default().fg(focus.border_color());

        let group_detail = Table::new(group_detail_rows);
        let group_detail = group_detail
            .header(Row::new(vec![
                Cell::from(Span::styled(
                    "lang",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "name",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "def",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "fcd",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "en",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(focus.text_color()))
                    .title("Detail")
                    .border_type(BorderType::Plain)
                    .border_style(border_style),
            )
            .widths(&[
                Constraint::Min(10),
                Constraint::Min(30),
                Constraint::Min(5),
                Constraint::Min(5),
                Constraint::Min(5),
            ])
            .column_spacing(1)
            .highlight_style(highlight_style);

        // disable default highlighting if we want to highlight a single item
        let group_detail = if self.selected_col.is_some() {
            group_detail.highlight_style(Style::default())
        } else {
            group_detail
        };
        frame.render_stateful_widget(group_detail, area, &mut self.table_state);
    }
}
