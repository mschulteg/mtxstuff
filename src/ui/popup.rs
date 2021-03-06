use crate::command::Command;
use crate::command::CommandHandler;
use crate::command::CommandHandlerStatus;

use super::centered_rect_fit_text;
use super::Action;
use super::CommandType;
use super::FocusState;
use super::KeyPressConsumer;
use super::{centered_rect, centered_rect_with_height};
use crossterm::event::KeyCode;
use std::fs::File;
use std::io::prelude::*;
use std::io::Stdout;
use tui::layout::Alignment;
use tui::text::Text;
use tui::widgets::Clear;
use tui::widgets::Gauge;
use tui::widgets::Paragraph;
use tui::widgets::Wrap;
use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders},
    Frame,
};

// TODO: Frame<B: Backend>
pub(crate) trait PopupRender {
    fn render_widget(
        &mut self,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        focus: FocusState,
    );
}

pub(crate) trait Popup: PopupRender + KeyPressConsumer {}
impl<T: PopupRender + KeyPressConsumer> Popup for T {}

//#[derive(Clone)]
pub(crate) struct PopupRenderer {
    pub(crate) popup_stack: Vec<Box<dyn Popup>>,
}

impl PopupRenderer {
    pub(crate) fn active(&self) -> bool {
        !self.popup_stack.is_empty()
    }
}

impl PopupRender for PopupRenderer {
    fn render_widget(
        &mut self,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        focus: FocusState,
    ) {
        let stack_len = self.popup_stack.len();
        for (i, popup) in self.popup_stack.iter_mut().enumerate() {
            let focus = if focus == FocusState::Highlight && i == stack_len - 1 {
                FocusState::Highlight
            } else {
                FocusState::Background
            };
            popup.render_widget(frame, area, focus);
        }
    }
}

impl KeyPressConsumer for PopupRenderer {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        if let Some(active_popup) = self.popup_stack.last_mut() {
            active_popup.process_key(key_code)
        } else {
            Action::Pass
        }
    }
}

pub(crate) struct CommandPopup {
    pub(crate) commands: Vec<Command>,
    pub(crate) command_strings: Vec<String>,
    pub(crate) scroll: u16,
}

impl CommandPopup {
    pub(crate) fn new<B: IntoIterator<Item = Command>>(commands: B) -> Self {
        let commands: Vec<Command> = commands.into_iter().collect();
        let command_strings: Vec<_> = commands
            .iter()
            .map(|cmd| cmd.to_cmd_string())
            .flatten()
            .collect();
        CommandPopup {
            commands,
            command_strings,
            scroll: Default::default(),
        }
    }

    fn render<B: tui::backend::Backend>(
        &mut self,
        frame: &mut Frame<B>,
        area: Rect,
        focus: FocusState,
    ) {
        let border_style = Style::default().fg(focus.border_color());
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title(Span::styled(
                "Commands Preview - Press F2 again to apply the changes - or press Enter to save commands to mtx_commands.sh - Esc to abort",
                Style::default().add_modifier(Modifier::BOLD),
            ))
            .border_type(BorderType::Thick)
            .border_style(border_style);

        let text: Vec<Spans> = self
            .command_strings
            .iter()
            .map(AsRef::as_ref)
            .map(Spans::from)
            .collect();
        // text[0]
        //     .0
        //     .push(Span::styled("HELLO", Style::default().fg(Color::Green)));
        //let paragraph = Paragraph::new(self.commands.join("\n\n"))
        let paragraph = Paragraph::new(text)
            .style(Style::default())
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .scroll((self.scroll, 0));
        let area = centered_rect(80, 80, area);
        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, area);
    }

    fn to_file(&self) -> std::io::Result<()> {
        let mut file = File::create("mtx_commands.sh")?;
        file.write_all(b"#!/bin/sh\n")?;
        for cmd in self.command_strings.iter() {
            file.write_all(cmd.as_bytes())?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }
}

impl PopupRender for CommandPopup {
    fn render_widget(
        &mut self,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        focus: FocusState,
    ) {
        self.render(frame, area, focus);
    }
}

impl KeyPressConsumer for CommandPopup {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        match key_code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll > 0 {
                    self.scroll -= 1
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.scroll < 1000 {
                    self.scroll += 1
                };
            }
            KeyCode::F(2) => {
                return Action::RunCommands((CommandType::AlterFiles, self.commands.clone()));
            }
            KeyCode::Esc => {
                return Action::ClosePopup;
            }
            KeyCode::Enter => match self.to_file() {
                Ok(_) => return Action::ShowMessage("Commands were saved".to_string()),
                Err(err) => {
                    return Action::ShowMessage(format!("Commands could not be saved: {}", err))
                }
            },
            _ => {}
        }
        Action::Pass
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
        focus: FocusState,
    ) {
        let area = centered_rect_with_height(50, 3, area);
        let border_style = Style::default().fg(focus.border_color());
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
        focus: FocusState,
    ) {
        self.render(frame, area, focus);
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
            }
            _ => {}
        }
        Action::Pass
    }
}

#[derive(Clone, Default)]
pub(crate) struct MessagePopup {
    pub(crate) message: String,
}

impl MessagePopup {
    fn render<B: tui::backend::Backend>(
        &mut self,
        frame: &mut Frame<B>,
        area: Rect,
        focus: FocusState,
    ) {
        let margin_y = 2;
        let area = centered_rect_fit_text(self.message.as_ref(), 2, margin_y, area);
        let mut spans = Vec::<Spans>::new();
        for _ in 0..margin_y {
            // add empty lines to vertically center text
            spans.push(Spans::from(vec![Span::raw("")]));
        }
        spans.push(Spans::from(vec![Span::raw(&self.message)]));
        //let input = Paragraph::new(self.message.as_ref())
        let border_style = Style::default().fg(focus.border_color());
        let input = Paragraph::new(spans)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .border_style(border_style),
            )
            .alignment(Alignment::Center);
        frame.render_widget(Clear, area);
        frame.render_widget(input, area);
    }
}

impl PopupRender for MessagePopup {
    fn render_widget(
        &mut self,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        focus: FocusState,
    ) {
        self.render(frame, area, focus);
    }
}

impl KeyPressConsumer for MessagePopup {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        match key_code {
            KeyCode::Esc | KeyCode::Enter => {
                return Action::ClosePopup;
            }
            _ => {}
        }
        Action::Pass
    }
}

pub(crate) struct CommandRunnerPopup<'a> {
    pub(crate) command_handler: Option<CommandHandler>,
    pub(crate) command_type: CommandType,
    pub(crate) title: String,
    pub(crate) scroll: u16,
    pub(crate) results: Option<Vec<Command>>,
    pub(crate) log: Text<'a>,
    pub(crate) error: bool,
}

impl<'a> CommandRunnerPopup<'a> {
    pub(crate) fn new(commands: Vec<Command>, command_type: CommandType, title: String) -> Self {
        CommandRunnerPopup {
            command_handler: Some(CommandHandler::new(commands)),
            command_type,
            title,
            scroll: Default::default(),
            results: Default::default(),
            log: Default::default(),
            error: false,
        }
    }

    fn render<B: tui::backend::Backend>(
        &mut self,
        frame: &mut Frame<B>,
        area: Rect,
        focus: FocusState,
    ) {
        let border_style = Style::default().fg(focus.border_color());
        if let Some(mut command_handler) = self.command_handler.take() {
            match command_handler.check() {
                CommandHandlerStatus::Percent(percent) => {
                    let gauge_box = Gauge::default()
                        .block(
                            Block::default()
                                .title(self.title.clone())
                                .borders(Borders::ALL)
                                .border_type(BorderType::Thick)
                                .border_style(border_style),
                        )
                        .gauge_style(Style::default().fg(focus.sel_color()))
                        .percent(percent);
                    let area = centered_rect(70, 10, area);
                    frame.render_widget(Clear, area);
                    frame.render_widget(gauge_box, area);
                    self.command_handler = Some(command_handler);
                }
                CommandHandlerStatus::Done => {
                    let mut done_commands: Vec<Command> = Vec::new();
                    self.scroll = 0;
                    let results: Vec<std::io::Result<Command>> = command_handler.into_results();
                    for res in results.into_iter() {
                        match res {
                            Ok(command) => {
                                self.log.extend(Text::styled(
                                    format!("Command: {:}\n", command.to_cmd_string().unwrap()),
                                    Style::default().add_modifier(Modifier::BOLD),
                                ));
                                let output = command.output.as_ref().expect("command has executed");
                                if !output.status.success() {
                                    self.error = true;
                                    // TODO - get rid of the clones
                                    self.log.extend(Text::styled(
                                        command.success_string(),
                                        Style::default().fg(Color::Red),
                                    ));
                                    if !output.stdout.is_empty() {
                                        self.log.extend(Text::raw("Command output (stdout) is:"));
                                        self.log.extend(Text::styled(
                                            output.stdout.clone(),
                                            Style::default().fg(Color::DarkGray),
                                        ));
                                    }
                                    if !output.stderr.is_empty() {
                                        self.log.extend(Text::raw("Command output (stderr) is:"));
                                        self.log.extend(Text::styled(
                                            output.stderr.clone(),
                                            Style::default().fg(Color::DarkGray),
                                        ));
                                    }
                                } else {
                                    self.log.extend(Text::styled(
                                        command.success_string(),
                                        Style::default().fg(Color::Green),
                                    ));
                                }
                                self.log.extend(Text::raw("\n"));
                                // put command in results
                                done_commands.push(command);
                            }
                            Err(err) => {
                                self.log.extend(Text::styled(
                                    format!("Failed to execute process: {:}", err),
                                    Style::default().fg(Color::Red),
                                ));
                                self.error = true;
                                break;
                            }
                        }
                    }
                    self.results = Some(done_commands);
                }
            };
        } else if self.error {
            let paragraph = Paragraph::new(self.log.clone())
                .style(Style::default())
                .block(
                    Block::default()
                        .title("Log")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Thick)
                        .border_style(border_style),
                )
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true })
                .scroll((self.scroll, 0));
            let area = centered_rect(80, 80, area);
            frame.render_widget(Clear, area);
            frame.render_widget(paragraph, area);
        } else {
            let message = "Done - Press Enter";
            // TODO: DUPLICATE CODE OF MESSAGE BOX - TIDY UP
            let margin_y = 0;
            let area = centered_rect_fit_text(message, 2, margin_y, area);
            let mut spans = Vec::<Spans>::new();
            for _ in 0..margin_y {
                // add empty lines to vertically center text
                spans.push(Spans::from(vec![Span::raw("")]));
            }
            spans.push(Spans::from(vec![Span::raw(message)]));
            //let input = Paragraph::new(self.message.as_ref())
            let input = Paragraph::new(spans)
                .style(Style::default().fg(Color::White))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Thick)
                        .border_style(border_style),
                )
                .alignment(Alignment::Center);
            frame.render_widget(Clear, area);
            frame.render_widget(input, area);
        }
    }
}

impl<'a> PopupRender for CommandRunnerPopup<'a> {
    fn render_widget(
        &mut self,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        focus: FocusState,
    ) {
        self.render(frame, area, focus);
    }
}

impl<'a> KeyPressConsumer for CommandRunnerPopup<'a> {
    fn process_key(&mut self, key_code: crossterm::event::KeyCode) -> Action {
        match key_code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll > 0 {
                    self.scroll -= 1
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.scroll < 1000 {
                    self.scroll += 1
                };
            }
            KeyCode::Esc => {
                // or self.results.is_some()
                return if self.results.is_some() {
                    Action::CommandsDone((self.command_type, self.results.take().unwrap()))
                } else {
                    Action::Pass
                };
            }
            KeyCode::Enter => {
                return if self.results.is_some() {
                    Action::CommandsDone((self.command_type, self.results.take().unwrap()))
                } else {
                    Action::Pass
                }
            }
            _ => {}
        }
        Action::Pass
    }
}
