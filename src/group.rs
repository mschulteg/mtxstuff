use itertools::Itertools;
use super::table::Table;
use super::file::{File, TrackType};
use super::command::Command;
use crate::track_operations::{TrackOperation, TrackOperations};

pub fn key_sublang_subname(file: &File) -> Vec<GroupKey> {
    file.subtitle_tracks
        .iter()
        .map(|track| {
            GroupKey {
                language: track.language.clone(),
                //track.name.clone().unwrap_or(String::from("")),
                name: track.name.clone(),
                default: track.default,
                forced: track.forced,
                enabled: track.enabled,
            }
        })
        .collect()
}

pub fn key_audlang_audname(file: &File) -> Vec<GroupKey> {
    file.audio_tracks
        .iter()
        .map(|track| {
            GroupKey {
                language: track.language.clone(),
                name: track.name.clone(),
                default: track.default,
                forced: track.forced,
                enabled: track.enabled,
            }
        })
        .collect()
}

#[derive(Clone)]
pub struct Group<'a> {
    pub key: Vec<GroupKey>,
    pub files: Vec<&'a File>,
}

impl <'a>Group<'a>{
    pub fn apply_changes(&self, keys: &[GroupKey], track_type: TrackType) -> Vec<Command>{
        let mut ops = TrackOperations::new(track_type);
        self.key.iter().zip(keys.iter()).enumerate().for_each(|(idx,(cur, changed))| {
            if cur == changed {return};
            if cur.language != changed.language {
                ops.add(idx as i64, TrackOperation::SetLang(changed.language.as_deref()))
            }
            if cur.name != changed.name {
                ops.add(idx as i64, TrackOperation::SetTitle(changed.name.as_deref()))
            }
            if cur.default != changed.default {
                ops.add(idx as i64, TrackOperation::SetDefault(changed.default));
            }
            if cur.forced != changed.forced {
                ops.add(idx as i64, TrackOperation::SetForced(changed.forced));
            }
            if cur.enabled != changed.enabled {
                ops.add(idx as i64, TrackOperation::SetEnabled(changed.enabled));
            }
        });
        let cmds: Vec<_> = self.files.iter().map(|file| ops.generate_command(file)).collect();
        cmds
    }
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, std::fmt::Debug)]
pub struct GroupKey {
    pub language: Option<String>,
    pub name: Option<String>,
    pub default: bool,
    pub forced: bool,
    pub enabled: bool,
}

impl GroupKey {
    pub fn headers(&self) -> Vec<&str> {
        vec!["lang", "name", "def", "fcd", "en"]
    }

    pub fn row(&self) -> Vec<String> {
        let language = self.language.clone().unwrap_or_else(|| String::from(""));
        let name = self.name.clone().unwrap_or_else(|| String::from(""));
        let default = if self.default {
            "[x]".to_owned()
        } else {
            "[ ]".to_owned()
        };
        let forced = if self.forced {
            "[x]".to_owned()
        } else {
            "[ ]".to_owned()
        };
        let enabled = if self.enabled {
            "[x]".to_owned()
        } else {
            "[ ]".to_owned()
        };
        vec![language, name, default, forced, enabled]
    }
}


pub fn print_groupkeys(keys: &[GroupKey]) {
    let data: Vec<Vec<String>> = keys.iter().map(|gk| gk.row()).collect();
    let mut table = Table::new(data.iter().map(AsRef::as_ref), &keys[0].headers());
    let line_numbers: Box<[usize]> = (0..table.lines.len()).collect();
    table.insert_column(0, "#", &line_numbers);
    table.print();
}

pub fn groupby(files: &[File], key_func: fn(&File) -> Vec<GroupKey>) -> Vec<Group> {
    let mut files_temp: Vec<&File> = files.iter().collect();
    files_temp.sort_by_key(|ident| key_func(ident));

    let mut groups = Vec::<Group>::new();
    for (key, group) in &files_temp.into_iter().group_by(|elt| key_func(elt)) {
        groups.push(Group {
            key,
            files: group.collect(),
        })
    }
    groups
}

pub fn print_groups(groups: &[Group]) {
    for (idx, group) in groups.iter().enumerate() {
        //println!("Group with key {:?}", group.key);
        println!("---Group {}---", idx);
        println!("Keys for this group are:");
        if !group.key.is_empty(){
            print_groupkeys(&group.key);
        } else {
            println!("Empty");
        }
        println!("Files in this group are");
        for elem in &group.files {
            println!("    {:?}", elem.file_name);
        }
        println!();
        println!();
    }
}