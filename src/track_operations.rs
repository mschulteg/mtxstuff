use crate::command::Command;
use crate::file::{File, Flag, TrackType};

#[derive(Copy, Clone, PartialEq)]
pub(crate) enum TrackOperation<'a> {
    SetForced(bool),
    SetDefault(bool),
    SetEnabled(bool),
    SetDefaultExclusive(bool),
    SetTitle(Option<&'a str>),
    SetLang(Option<&'a str>),
}

pub(crate) struct TrackOperations<'a> {
    track_type: TrackType,
    cmds: Vec<(i64, TrackOperation<'a>)>,
}

impl<'a> TrackOperations<'a> {
    pub(crate) fn new(track_type: TrackType) -> Self {
        TrackOperations { track_type, cmds: Default::default() }
    }
    
    pub(crate) fn empty(&self) -> bool {
        !self.cmds.is_empty()
    }

    pub(crate) fn add(&mut self, track_no: i64, track_command: TrackOperation<'a>) {
        self.cmds.push((track_no, track_command));
    }

    pub(crate) fn generate_command(&self, file: &File) -> Command {
        let mut command = Command::new("mkvpropedit");
        let mut arguments = self.generate_arguments(file);
        arguments.push(file.file_name.clone());
        command.arguments.extend(arguments);
        command
    }

    fn generate_arguments(&self, file: &File) -> Vec<String> {
        let mut arguments = Vec::<String>::new();
        let tracks = match self.track_type {
            TrackType::Subtitles => &file.subtitle_tracks,
            TrackType::Audio => &file.audio_tracks,
            TrackType::Video => &file.video_tracks,
        };
        let get_track_id = |track_no| tracks.get(track_no as usize).unwrap().id;

        for cmd in &self.cmds {
            let track_no = cmd.0;
            match cmd.1 {
                TrackOperation::SetForced(val) => {
                    TrackOperations::set_flag(
                        &mut arguments,
                        get_track_id(track_no),
                        Flag::Forced,
                        val,
                    );
                }
                TrackOperation::SetDefault(val) => {
                    TrackOperations::set_flag(
                        &mut arguments,
                        get_track_id(track_no),
                        Flag::Default,
                        val,
                    );
                }
                TrackOperation::SetEnabled(val) => {
                    TrackOperations::set_flag(
                        &mut arguments,
                        get_track_id(track_no),
                        Flag::Enabled,
                        val,
                    );
                }
                TrackOperation::SetDefaultExclusive(_) => {
                    // TODO: remove bool completely?
                    for track in tracks.iter() {
                        let value = get_track_id(track_no) == track.id;
                        TrackOperations::set_flag(
                            &mut arguments,
                            track.id,
                            Flag::Default,
                            value,
                        );
                    }
                }
                TrackOperation::SetTitle(val) => {
                    TrackOperations::set_name(
                        &mut arguments,
                        get_track_id(track_no),
                        val,
                    );
                }
                TrackOperation::SetLang(val) => {
                    TrackOperations::set_lang(
                        &mut arguments,
                        get_track_id(track_no),
                        val,
                    );
                }
            }
        }
        arguments
    }

    pub fn set_name(arguments: &mut Vec<String>, track_id: i64, name: Option<&str>) {
        arguments.push("--edit".to_owned());
        arguments.push(format!("track:@{}", track_id + 1));
        if let Some(name) = name {
            arguments.push("--set".to_owned());
            arguments.push(format!("name={}", name));
        } else {
            arguments.push("--delete".to_owned());
            arguments.push("name".to_owned());
        }
    }

    pub fn set_lang(arguments: &mut Vec<String>, track_id: i64, name: Option<&str>) {
        arguments.push("--edit".to_owned());
        arguments.push(format!("track:@{}", track_id + 1));
        if let Some(name) = name {
            arguments.push("--set".to_owned());
            arguments.push(format!("language={}", name));
        } else {
            arguments.push("--set".to_owned());
            arguments.push("language=und".to_string());
            // This just results in language being set to eng
            // see https://gitlab.com/mbunkus/mkvtoolnix/-/issues/1929
            // arguments.push("--delete".to_owned());
            // arguments.push("language".to_owned());
        }
    }

    fn set_flag(arguments: &mut Vec<String>, track_id: i64, flag: Flag, value: bool) {
        arguments.push("--edit".to_owned());
        arguments.push(format!("track:@{}", track_id + 1));
        arguments.push("--set".to_owned());
        let flag_str = match flag {
            Flag::Default => "flag-default",
            Flag::Forced => "flag-forced",
            Flag::Enabled => "flag-enabled",
        };
        let value = if value { "1" } else { "0" };
        arguments.push(format!("{}={}", flag_str, value));
    }
}
