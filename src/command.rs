use super::file::{File, Flag, Track, TrackType};
#[derive(Debug)]
pub struct Command<'a> {
    pub executable: String,
    pub file: &'a File,
    arguments: Vec<String>,
}

impl<'a> Command<'a> {
    pub fn new(file: &'a File) -> Self {
        Command {
            executable: "mkvpropedit".into(),
            file,
            arguments: Vec::new(),
        }
    }

    pub fn to_cmd_string(&self) -> Option<String> {
        if self.arguments.len() == 0 {
            return None;
        }
        let mut string = self.executable.clone();
        string.push(' ');
        string.push_str(&self.arguments.join(" "));
        string.push(' ');
        string.push('"');
        string.push_str(&self.file.file_name);
        string.push('"');
        Some(string)
    }

    pub fn run(&self) {
        let mut string = self.executable.clone();
        for arg in self.arguments.iter() {
            string.push(' ');
            string.push_str(arg);
        }

        println!("Running command {:?}", string);
        // let result = Exec::cmd(&self.executable)
        //         .args(&self.arguments)
        //         .capture()
        //         .unwrap()
        //         .stdout_str();
    }

    pub fn track_set(
        &mut self,
        ttype: TrackType,
        flag: Flag,
        track_no: i64,
        value: bool,
        unset_others: bool,
    ) {
        let tracks = match ttype {
            TrackType::Subtitles => &self.file.subtitle_tracks,
            TrackType::Audio => &self.file.audio_tracks,
            TrackType::Video => &self.file.video_tracks,
        };
        let sel_track = tracks.get(track_no as usize).unwrap();

        if unset_others {
            for track in tracks.iter() {
                let value = sel_track.id == track.id;
                self.arguments
                    .extend(Command::track_set_flag(track, flag, value));
            }
        } else {
            self.arguments
                .extend(Command::track_set_flag(sel_track, flag, value));
        }
    }

    fn track_set_flag(track: &Track, flag: Flag, value: bool) -> Vec<String> {
        let mut arguments = Vec::new();
        arguments.push("--edit".to_owned());
        arguments.push(format!("track:@{}", track.id + 1));
        arguments.push("--set".to_owned());
        let flag_str = match flag {
            Flag::Default => "flag-default",
            Flag::Forced => "flag-forced",
            Flag::Enabled => "flag-enabled",
        };
        arguments.push(flag_str.to_owned());
        let value = if value { "1" } else { "0" };
        arguments.push(value.to_owned());
        arguments
    }
}
