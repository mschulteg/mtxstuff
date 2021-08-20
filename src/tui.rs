use super::group::{Group, GroupKey};
use super::file::{TrackType};
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use tui::Frame;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table,
        TableState, Tabs,
    },
    Terminal,
};

const SEL_COLOR: Color = Color::LightCyan;

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
    Subs,
    Audio,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Subs => 1,
            MenuItem::Audio => 2,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum ActiveWidget {
    Groups,
    Details,
    DetailsItems,
    Popup,
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

// trait SelectableState{
//     fn select(&mut self, index: Option<usize>);
//     fn selected(&self) -> Option<usize>;

//     fn navigate_down(&mut self, list_size: usize) {
//         if let Some(selected) = self.selected() {
//             if selected >= list_size - 1 {
//                 self.select(Some(0));
//             } else {
//                 self.select(Some(selected + 1));
//             }
//         }
//     }

//     fn navigate_up(&mut self, list_size: usize) {
//         if let Some(selected) = self.selected() {
//             if selected > 0 {
//                 self.select(Some(selected - 1));
//             } else {
//                 self.select(Some(list_size - 1));
//             }
//         }
//     }
// }

// impl SelectableState for ListState {
//     fn select(&mut self, index: Option<usize>) {
//         self.select(index)
//     }

//     fn selected(&self) -> Option<usize> {
//         self.selected()
//     }
// }

// impl SelectableState for TableState {
//     fn select(&mut self, index: Option<usize>) {
//         self.select(index)
//     }

//     fn selected(&self) -> Option<usize> {
//         self.selected()
//     }
// }

use std::io::Stdout;
// TODO: Frame<B: Backend>
trait Popup {
    fn render_widget(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect);

    fn selected(&self) -> Option<usize> {
        None
    }

    fn select(&mut self, index: Option<usize>) {
    }

    fn length(&self) -> usize {
        0
    }
}

//#[derive(Clone)]
struct PopupRenderer {
    popup_stack: Vec<Box<dyn Popup>>
}

impl PopupRenderer {
    fn render_stuff(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        for popup in self.popup_stack.iter_mut() {
            popup.render_widget(frame, area);
        }
    }
}

#[derive(Clone, Default)]
struct CommandPopup {
    commands: Vec<String>,
    list_state: ListState,
}


impl CommandPopup {
    fn render(&mut self) -> List{
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

    fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }
}




#[derive(Clone, Default)]
struct GroupFilesListWidget {
    list_state: ListState,
    file_names: Vec<String>,
}

impl GroupFilesListWidget {
    fn set_filenames(&mut self, group: &Group) {
        self.file_names.clear();
        self.file_names.extend(group.files.iter().map(|file|file.file_name.clone()));
    }

    fn render<B: tui::backend::Backend> (&mut self, frame: &mut Frame<B>, area: Rect) {
        // Group files
        let group_files_items: Vec<_> = self.file_names.iter().map(|file_name| ListItem::new(Spans::from(vec![Span::styled(file_name.clone(), Style::default(),)]))).collect();

        let group_files = List::new(group_files_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White))
                    .title("Files")
                    .border_type(BorderType::Plain),
            )
            .highlight_style(
                Style::default()
                    .bg(SEL_COLOR)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(group_files, area, &mut self.list_state);
    }
}



#[derive(Clone, Default)]
struct GroupListWidget {
    list_state: ListState,
}

impl GroupListWidget {
    //TODO: remove groups, maybe keep a copy
    fn render<B: tui::backend::Backend> (&mut self, frame: &mut Frame<B>, area: Rect, num_groups: usize) {
        let groupnames_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Groups")
            .border_type(BorderType::Plain);

        let groupnames_items: Vec<_> = (0..num_groups)
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


#[derive(Clone, Default)]
struct TrackTableWidget {
    table_state: TableState,
    selected_col: Option<usize>,
    keys_orig: Vec<GroupKey>,
    keys_copy: Vec<GroupKey>,
}

impl TrackTableWidget {
    fn select(&mut self, index: Option<usize>) {
        self.table_state.select(index);
    }

    fn selected(&self) -> Option<usize> {
        self.table_state.selected()
    }

    fn render<B: tui::backend::Backend> (&mut self, frame: &mut Frame<B>, area: Rect) {
        let highlight_style = Style::default()
            .bg(SEL_COLOR)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD);
        
        let create_style = |item: &String, idx_col: usize, idx_row: usize| {
            let mut style  = Style::default();
            if let Some(sel_col) = self.selected_col {
                if sel_col == idx_col && self.selected().unwrap() == idx_row {
                    style = style.bg(SEL_COLOR)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
                }
            }
            if let Some(group_item) = self.keys_orig.get(idx_row)
                .and_then(|r| r.row().get(idx_col).cloned()){
                if group_item != *item {
                    style = style.add_modifier(Modifier::ITALIC);
                }
            }
            style
        };

        let group_detail_rows: Vec<Row> = 
            self.keys_copy.iter().enumerate().map(|(idx_row, keyrow)| {
                Row::new(keyrow.row().iter().enumerate().map(|(idx_col, item)| {
                    let cell = Cell::from(Span::raw(item.clone()));
                    cell.style(create_style(item, idx_col, idx_row))
                }))
            }).collect();

        let group_detail= Table::new(group_detail_rows);
        let group_detail = group_detail
            .header(Row::new(vec![
                Cell::from(Span::styled(
                    "lange",
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
                    .style(Style::default().fg(Color::White))
                    .title("Detail")
                    .border_type(BorderType::Plain),
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
        frame.render_stateful_widget(
            group_detail,
            area,
            &mut self.table_state,
        );
    }
}

//#[derive(Clone)]
struct GroupTabData<'a> {
    group_list: GroupListWidget,
    track_table: TrackTableWidget,
    group_files_list: GroupFilesListWidget,
    groups: &'a [Group<'a>],
    active_widget: ActiveWidget,
    popup_data: PopupRenderer,
    track_type: TrackType,
}

use std::fs::File;
use std::io::prelude::*;
impl<'a> GroupTabData<'a>{
    fn new(groups: &'a [Group<'a>], track_type: TrackType) -> Self {
        let mut group_list = GroupListWidget::default();
        group_list.list_state.select(if !groups.is_empty() { Some(0) } else { None });
        GroupTabData {
            group_list,
            track_table: TrackTableWidget::default(),
            group_files_list: GroupFilesListWidget::default(),
            groups,
            active_widget: ActiveWidget::Groups,
            popup_data: PopupRenderer{popup_stack: Vec::new()},
            track_type,
        }
    }

    fn generate_commands(&mut self) {
        let sel_group = self.selected_group().unwrap();
        let commands = sel_group.apply_changes(&self.track_table.keys_copy, self.track_type);
        let mut file = File::create("mtx_commands.sh").unwrap();
        let strings: Vec<_> = commands.iter().map(|cmd|cmd.to_cmd_string()).collect();
        file.write_all(b"#!/bin/sh\n").unwrap();
        for cmd in strings.iter() {
            file.write_all(cmd.as_bytes()).unwrap();
            file.write_all(b"\n").unwrap();
        }
        self.active_widget = ActiveWidget::Popup;
        let mut command_popup = CommandPopup::default();
        command_popup.commands.extend(strings);
        self.popup_data.popup_stack.push(Box::new(command_popup));
    }

    fn refresh_keys(&mut self){
        self.track_table.keys_copy = if let Some(sel_group) = self.selected_group() {
            sel_group.key.clone()
        } else {
            Vec::<GroupKey>::new()
        };
        self.track_table.keys_orig = self.track_table.keys_copy.clone();
    }

    fn select(&mut self, index: Option<usize>) {
        match self.active_widget {
            ActiveWidget::Groups => {
                self.group_list.list_state.select(index);
                self.refresh_keys();
            },
            ActiveWidget::Details => self.track_table.select(index),
            _ => {}
        }
    }

    fn selected(&self) -> Option<usize> {
        match self.active_widget {
            ActiveWidget::Groups => self.group_list.list_state.selected(),
            ActiveWidget::Details => self.track_table.selected(),
            //TODO: this does not work for popups, maybe move from this approach, having a universal selected() select() navigate_down() ...
            _ => None,
        }
    }


    fn selected_group(&self) -> Option<&Group> {
        self
            .group_list
            .list_state
            .selected()
            .and_then(|selected| self.groups.get(selected))
    }

    fn length(&self) -> usize {
        match self.active_widget {
            ActiveWidget::Groups => self.groups.len(),
            ActiveWidget::Details => self
                .group_list
                .list_state
                .selected()
                .and_then(|selected| self.groups.get(selected))
                .map(|g| g.key.len())
                .unwrap_or(0),
            _ => 0,
        }
    }

    fn navigate_down(&mut self) {
        if let Some(selected) = self.selected() {
            if selected >= self.length() - 1 {
                self.select(Some(0));
            } else {
                self.select(Some(selected + 1));
            }
        }
    }

    fn navigate_up(&mut self) {
        if let Some(selected) = self.selected() {
            if selected > 0 {
                self.select(Some(selected - 1));
            } else {
                self.select(Some(self.length() - 1));
            }
        }
    }

    fn navigate_right(&mut self) {
        if self.active_widget == ActiveWidget::Groups {
            self.active_widget = ActiveWidget::Details;
            if self.length() == 0 {
                self.active_widget = ActiveWidget::Groups;
            } else {
                self.select(Some(0))
            }
        }
        if self.active_widget == ActiveWidget::DetailsItems {
            if let Some(selected_col) = self.track_table.selected_col {
                if selected_col < 4 {// TODO: do not hard code
                    self.track_table.selected_col = Some(selected_col + 1);
                }
            }
        }
    }

    fn navigate_left(&mut self) {
        if self.active_widget == ActiveWidget::Details {
            self.select(None);
            self.active_widget = ActiveWidget::Groups;
        }
        if self.active_widget == ActiveWidget::DetailsItems {
            if let Some(selected_col) = self.track_table.selected_col {
                if selected_col > 0 {
                    self.track_table.selected_col = Some(selected_col - 1);
                }
            }
        }
    }

    fn navigate_enter(&mut self) {
        if self.active_widget == ActiveWidget::Details {
            self.active_widget = ActiveWidget::DetailsItems;
            self.track_table.selected_col = Some(0);
        }
        else if self.active_widget == ActiveWidget::DetailsItems {
            if let Some(sel_row) = self.track_table.selected(){
                let gkey = self.track_table.keys_copy.get_mut(sel_row).unwrap();
                match self.track_table.selected_col.unwrap() {
                    2 => {gkey.default = !gkey.default;}
                    3 => {gkey.forced = !gkey.forced;}
                    4 => {gkey.enabled = !gkey.enabled;}
                    _ => {}
                }
            }
        }
    }

    fn navigate_back(&mut self) {
        if self.active_widget == ActiveWidget::Popup {
            self.active_widget = ActiveWidget::DetailsItems;
        }
        if self.active_widget == ActiveWidget::DetailsItems {
            self.active_widget = ActiveWidget::Details;
            self.track_table.selected_col = None;
        }
    }
}

pub fn main_loop(
    groups_subs: &[Group],
    groups_audio: &[Group],
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate && tx.send(Event::Tick).is_ok(){
                last_tick = Instant::now();
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_titles = vec!["Home", "Subs", "Audio", "Quit"];
    let mut active_menu_item = MenuItem::Home;

    let mut audio_tab_data = GroupTabData::new(groups_audio, TrackType::Audio);
    let mut sub_tab_data = GroupTabData::new(groups_subs, TrackType::Subtitles);
    // Refresh keys which means that keys are copied to the editable area.
    audio_tab_data.refresh_keys();
    sub_tab_data.refresh_keys();

    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(size);

            let progressbar = Paragraph::new("Bla")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .title("Progress")
                        .border_type(BorderType::Plain),
                );

            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(SEL_COLOR)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(SEL_COLOR))
                .divider(Span::raw("|"));

            rect.render_widget(tabs, chunks[0]);

            let mut render_maintab = |tab_data: &mut GroupTabData| {
                let horiz_split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                    .split(chunks[1]);
                let vert_split = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(horiz_split[1]);

                tab_data.group_files_list.render(rect, vert_split[1]);

                tab_data.track_table.render(rect, vert_split[0]);
                tab_data.group_list.render(rect, horiz_split[0], tab_data.groups.len());
                if tab_data.active_widget == ActiveWidget::Popup {
                    //let block = Block::default().title("Popup").borders(Borders::ALL);
                    let popup_area = centered_rect(80, 80, chunks[1]);
                    rect.render_widget(Clear, popup_area);
                    tab_data.popup_data.render_stuff(rect, popup_area);
                }
            };

            match active_menu_item {
                MenuItem::Home => rect.render_widget(render_home(), chunks[1]),
                MenuItem::Subs => {
                    render_maintab(&mut sub_tab_data);
                }
                MenuItem::Audio => {
                    render_maintab(&mut audio_tab_data);
                }
            }
            rect.render_widget(progressbar, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('h') => active_menu_item = MenuItem::Home,
                KeyCode::Char('s') => active_menu_item = MenuItem::Subs,
                KeyCode::Char('a') => active_menu_item = MenuItem::Audio,
                KeyCode::Char('d') => {}
                KeyCode::Right => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.navigate_right();
                    };
                }
                KeyCode::Left => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.navigate_left();
                    };
                }
                KeyCode::Down => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.navigate_down();
                    };
                }
                KeyCode::Up => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.navigate_up();
                    };
                }
                KeyCode::Enter => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.navigate_enter();
                    };
                }
                KeyCode::F(2) => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.generate_commands();
                    };
                }
                KeyCode::Esc => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.navigate_back();
                    };
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }

    Ok(())
}

fn render_home<'a>() -> Paragraph<'a> {
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "pet-CLI",
            Style::default().fg(Color::LightBlue),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Press 'p' to access pets, 'a' to add random new pets and 'd' to delete the currently selected pet.")]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Home")
            .border_type(BorderType::Plain),
    );
    home
}
