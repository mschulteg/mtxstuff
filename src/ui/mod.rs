mod group_files_list_widget;
mod group_list_widget;
mod popup;
mod selectable_state;
mod track_table_widget;
use crate::ui::popup::PopupRender;

use self::popup::EditPopup;

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
use tui::Frame;
use std::io::{self, Stdout};
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

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum MenuItem {
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

#[derive(Copy, Clone, PartialEq, Debug)]
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
    EditString(String),
    EditStringResult(Option<String>),
    LoadGroup,
    SwitchTab(MenuItem),
    Quit,
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

fn centered_rect_with_height(percent_x: u16, height_y: u16, r: Rect) -> Rect {
    let height_rest = if r.height >= height_y {
        r.height - height_y
    } else {0};
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(height_rest / 2),
                Constraint::Length(height_y),
                Constraint::Length(height_rest / 2),
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
        log::info!(
            "active_widget is:{:?}, keycode is {:?}",
            self.active_widget,
            key_code
        );
        let res_action = match self.active_widget {
            ActiveWidget::Groups => self.group_list.process_key(key_code),
            ActiveWidget::Details => self.track_table.process_key(key_code),
            ActiveWidget::Popup => self.popup_data.process_key(key_code),
            ActiveWidget::Files => self.group_files_list.process_key(key_code),
        };
        // TODO: Match key_codes for GrouTabData itself (Tab switching, Quitting) if not in edit mode
        // For now, only do it if the active widget returned Action::Pass
        if res_action == Action::Pass && self.active_widget != ActiveWidget::Popup {
            match key_code {
                KeyCode::Char('h') => return Action::SwitchTab(MenuItem::Home),
                KeyCode::Char('s') => return Action::SwitchTab(MenuItem::Subs),
                KeyCode::Char('a') => return Action::SwitchTab(MenuItem::Audio),
                KeyCode::Char('q') => return Action::Quit,
                KeyCode::F(2) => {
                    self.generate_commands();
                    return Action::Pass;
                }
                _ => {}
            }
        }
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
                ActiveWidget::Popup => {
                    self.popup_data.popup_stack.pop();
                    self.group_files_list.leave();
                    self.track_table.leave();
                    self.active_widget = ActiveWidget::Groups;
                }
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
            Action::EditString(string) => {
                let new_popup = EditPopup { input: string };
                self.popup_data.popup_stack.push(Box::new(new_popup));
                self.active_widget = ActiveWidget::Popup;
            }
            Action::EditStringResult(res) => {
                if let Some(string) = res {
                    let row = self
                        .track_table
                        .selected()
                        .expect("Currently edited item must be selected");
                    self.track_table
                        .get_keys_copy_mut()
                        .get_mut(row)
                        .expect("Currently edit item must exist")
                        .name = Some(string);
                }
                self.popup_data.popup_stack.pop();
                // This is the same as above in Action::NavigateBackward(Popup)
                // Find a better solution
                //self.group_files_list.leave();
                //self.track_table.leave();
                self.active_widget = ActiveWidget::Details;
            }
            Action::LoadGroup => self.load_selected_group(),
            switch_tab @ Action::SwitchTab(_) => return switch_tab,
            Action::Quit => return Action::Quit,
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
        let strings: Vec<_> = commands
            .iter()
            .map(|cmd| cmd.to_cmd_string())
            .flatten()
            .collect();
        file.write_all(b"#!/bin/sh\n").unwrap();
        for cmd in strings.iter() {
            file.write_all(cmd.as_bytes()).unwrap();
            file.write_all(b"\n").unwrap();
        }
        self.active_widget = ActiveWidget::Popup;
        let mut command_popup = CommandPopup::default();
        command_popup.commands.extend(strings);
        command_popup.try_enter(); // Last chance we get to call Trait method
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
    
    fn render(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        let horiz_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
            .split(area);
        let vert_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(horiz_split[1]);

        self.group_files_list.render(frame, vert_split[1], self.active_widget);
        self.track_table.render(frame, vert_split[0], self.active_widget);
        self.group_list.render(frame, horiz_split[0], self.active_widget);

        if self.active_widget == ActiveWidget::Popup {
            self.popup_data.render_widget(frame, area, self.active_widget == ActiveWidget::Popup);
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

            match active_menu_item {
                MenuItem::Home => rect.render_widget(render_home(), chunks[1]),
                MenuItem::Subs => {
                    sub_tab_data.render(rect, chunks[1]);
                }
                MenuItem::Audio => {
                    audio_tab_data.render(rect, chunks[1]);
                }
            }
            rect.render_widget(progressbar, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => {
                let action = match active_menu_item {
                    MenuItem::Subs => sub_tab_data.process_key(event.code),
                    MenuItem::Audio => audio_tab_data.process_key(event.code),
                    _ => match event.code {
                        KeyCode::Char('h') => Action::SwitchTab(MenuItem::Home),
                        KeyCode::Char('s') => Action::SwitchTab(MenuItem::Subs),
                        KeyCode::Char('a') => Action::SwitchTab(MenuItem::Audio),
                        KeyCode::Char('q') => Action::Quit,
                        _ => Action::Pass,
                    },
                };
                match action {
                    Action::Quit => {
                        disable_raw_mode()?;
                        terminal.show_cursor()?;
                        break;
                    }
                    Action::SwitchTab(MenuItem::Home) => active_menu_item = MenuItem::Home,
                    Action::SwitchTab(MenuItem::Subs) => active_menu_item = MenuItem::Subs,
                    Action::SwitchTab(MenuItem::Audio) => active_menu_item = MenuItem::Audio,
                    _ => {}
                }
            }
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
            "mtxstuff alpha",
            Style::default().fg(Color::LightBlue),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Press 'S' to access Subtitle view, 'A' to access audio track view.")]),
        Spans::from(vec![Span::raw("Files are scanned and put into groups that share the same track metadata (name, lang, flags).")]),
        Spans::from(vec![Span::raw("This makes it easy to change metadata on multiple files that share the same general track list shape.")]),
        Spans::from(vec![Span::raw("Changes are applied to all files in a group!")]),
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
