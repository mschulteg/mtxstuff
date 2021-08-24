mod group_files_list_widget;
mod group_list_widget;
mod popup;
mod selectable_state;
mod track_table_widget;
use super::file::TrackType;
use super::group::Group;
use super::ui::group_files_list_widget::GroupFilesListWidget;
use super::ui::group_list_widget::GroupListWidget;
use super::ui::popup::{CommandPopup, PopupRenderer};
use super::ui::selectable_state::SelectableState;
use super::ui::track_table_widget::TrackTableWidget;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Tabs},
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
pub(crate) enum ActiveWidget {
    Groups,
    Details,
    Files,
    Popup,
}

#[derive(PartialEq)]
pub(crate) enum Action {
    NavigateForward(ActiveWidget),
    NavigateBackward(ActiveWidget),
    LoadGroup,
    Pass,
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

pub(crate) trait KeyPressConsumer {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action;
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

impl<'a> KeyPressConsumer for GroupTabData<'a> {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        let res_action = match self.active_widget {
            ActiveWidget::Groups => self.group_list.process_key(key_code),
            ActiveWidget::Details => self.track_table.process_key(key_code),
            ActiveWidget::Popup => self.track_table.process_key(key_code),
            ActiveWidget::Files => self.group_files_list.process_key(key_code),
        };
        match res_action {
            Action::NavigateForward(src_widget) => match src_widget {
                ActiveWidget::Details => {
                    if self.group_files_list.try_enter() {
                        self.track_table.leave();
                        self.active_widget = ActiveWidget::Files;
                    }
                }
                ActiveWidget::Groups => {
                    if self.track_table.try_enter() {
                        self.active_widget = ActiveWidget::Details;
                    } else if self.group_files_list.try_enter() {
                        self.active_widget = ActiveWidget::Files;
                    }
                }
                _ => {}
            },
            Action::NavigateBackward(src_widget) => match src_widget {
                ActiveWidget::Files => {
                    if self.track_table.try_enter() {
                        self.group_files_list.leave();
                        self.active_widget = ActiveWidget::Details;
                    } else {
                        self.group_files_list.leave();
                        self.active_widget = ActiveWidget::Groups;
                    }
                }
                ActiveWidget::Details => {
                    self.active_widget = ActiveWidget::Details;
                    self.track_table.leave();
                    self.active_widget = ActiveWidget::Groups;
                }
                _ => {}
            },
            Action::LoadGroup => self.load_selected_group(),
            Action::Pass => {}
        }
        Action::Pass
    }
}

use std::fs::File;
use std::io::prelude::*;
impl<'a> GroupTabData<'a> {
    fn new(groups: &'a [Group<'a>], track_type: TrackType) -> Self {
        GroupTabData {
            group_list: GroupListWidget::new(groups.len()),
            track_table: TrackTableWidget::default(),
            group_files_list: GroupFilesListWidget::default(),
            groups,
            active_widget: ActiveWidget::Groups,
            popup_data: PopupRenderer {
                popup_stack: Vec::new(),
            },
            track_type,
        }
    }

    fn generate_commands(&mut self) {
        let sel_group = self.selected_group().unwrap();
        let commands = sel_group.apply_changes(&self.track_table.get_keys_copy(), self.track_type);
        let mut file = File::create("mtx_commands.sh").unwrap();
        let strings: Vec<_> = commands.iter().map(|cmd| cmd.to_cmd_string()).collect();
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

    fn load_selected_group(&mut self) {
        self.track_table = TrackTableWidget::from_group(self.selected_group());
        self.group_files_list = GroupFilesListWidget::from_group(self.selected_group());
    }

    fn selected_group(&self) -> Option<&Group> {
        self.group_list
            .selected()
            .and_then(|selected| self.groups.get(selected))
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

            if last_tick.elapsed() >= tick_rate && tx.send(Event::Tick).is_ok() {
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
    audio_tab_data.load_selected_group();
    sub_tab_data.load_selected_group();

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
                tab_data.group_list.render(rect, horiz_split[0]);

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
                        tab_data.process_key(KeyCode::Right);
                    };
                }
                KeyCode::Left => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.process_key(KeyCode::Left);
                    };
                }
                KeyCode::Down => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.process_key(KeyCode::Down);
                    };
                }
                KeyCode::Up => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.process_key(KeyCode::Up);
                    };
                }
                KeyCode::Enter => {
                    if let Some(tab_data) = match active_menu_item {
                        MenuItem::Subs => Some(&mut sub_tab_data),
                        MenuItem::Audio => Some(&mut audio_tab_data),
                        _ => None,
                    } {
                        tab_data.process_key(KeyCode::Enter);
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
                        tab_data.process_key(KeyCode::Esc);
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
