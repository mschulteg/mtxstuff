mod command;
mod file;
mod group;
mod table;
mod ui;
mod track_operations;

use crate::ui::main_loop;
use crate::command::Command;
use crate::file::{File, Flag, Track, TrackType};
use crate::group::{groupby, key_audlang_audname, key_sublang_subname, print_groups};
use crate::track_operations::{TrackOperation, TrackOperations};

use std::path::{Path, PathBuf};
use walkdir::WalkDir;
fn get_files_recursively(path: &Path) -> Vec<PathBuf> {
    //let walker = WalkDir::new("/mnt/k/Incoming/tmp/mtxstuff_test").into_iter();
    let walker = WalkDir::new(path).into_iter();
    let files: Vec<PathBuf> = walker
        .filter(|e| e.as_ref().unwrap().metadata().unwrap().is_file())
        .map(|e| e.unwrap().path().to_path_buf())
        .filter(|e| e.extension().map(|e| e == "mkv").unwrap_or(false))
        .collect();
    files
}

use std::process;

fn test_subprocess(paths: Vec<PathBuf>) -> Vec<String> {
    // -> Vec<Identify> {
    //let json_strings =
    let json_strings: Vec<String> = paths
        .iter()
        .map(|path| {
            process::Command::new("mkvmerge")
                .arg("--identification-format")
                .arg("json")
                .arg("--identify")
                .arg(path)
                .output()
                .unwrap()
                .stdout
        })
        .map(|slice| std::str::from_utf8(&slice).unwrap().to_owned())
        .collect();
    //let stdout = capture_data.stdout_str();
    json_strings
}

fn test_identify(json_strings: Vec<String>) -> Vec<File> {
    json_strings
        .iter()
        .map(|json_str| serde_json::from_str(json_str).unwrap())
        .map(|json_val| File::from_json(json_val).unwrap())
        //.filter_map(File::from_json)
        .collect()
}

use clap::{App, AppSettings, Arg};

use log::info;
#[cfg(debug_assertions)]
use log4rs;

fn main() {
    #[cfg(debug_assertions)]
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    info!("test");

    let arg_directory = Arg::with_name("directory")
        .help("Path to directory")
        .required(true);
    let arg_group = Arg::with_name("group")
        .help("Path to directory")
        .required(false)
        .takes_value(true)
        .long("group");
    let arg_track = Arg::with_name("track")
        .help("Track number of the selected group")
        .required(false)
        .takes_value(true)
        .long("track");
    let arg_default = Arg::with_name("set-default")
        .help("Set the track with the specified number as default")
        .required(false)
        .takes_value(true)
        .long("set-default");
    let arg_default_ex = Arg::with_name("set-default-ex")
        .help("Set the track with the specified number as exclusive default")
        .required(false)
        .takes_value(true)
        .long("set-default-ex");
    let arg_forced = Arg::with_name("set-forced")
        .help("Set the track with the specified number as forced")
        .required(false)
        .takes_value(true)
        .long("set-forced");
    let arg_enabled = Arg::with_name("set-enabled")
        .help("Set the track with the specified number as enabled")
        .required(false)
        .takes_value(true)
        .long("set-enabled");
    let matches = App::new("mtxtesto")
        .setting(AppSettings::GlobalVersion)
        .version("1.0")
        .author("Moritz Schulte")
        .about("mtxtesto")
        .subcommand(
            App::new("subs")
                .about("controls testing features")
                .arg(&arg_directory)
                .arg(&arg_group)
                .arg(&arg_track)
                .arg(&arg_forced)
                .arg(&arg_enabled)
                .arg(&arg_default_ex)
                .arg(&arg_default),
        )
        .subcommand(
            App::new("audio")
                .about("controls testing features")
                .arg(&arg_directory)
                .arg(&arg_group)
                .arg(&arg_track)
                .arg(&arg_forced)
                .arg(&arg_enabled)
                .arg(&arg_default_ex)
                .arg(&arg_default),
        )
        .subcommand(
            App::new("tui")
                .about("controls testing features")
                .arg(&arg_directory),
        )
        .get_matches();

    let (sub_name, sub_matches) = match matches.subcommand() {
        (name, Some(sub_m)) => (name, sub_m),
        _ => {
            println!("No subcommand provided, exiting.");
            return;
        }
    };

    let path = sub_matches.value_of("directory");
    let path = PathBuf::from(path.unwrap());
    let paths = get_files_recursively(&path);
    let json_strings = test_subprocess(paths);
    let files = test_identify(json_strings);

    match sub_name {
        "subs" => cli_mode(files, sub_name, sub_matches),
        "audio" => cli_mode(files, sub_name, sub_matches),
        "video" => cli_mode(files, sub_name, sub_matches),
        "tui" => tui_mode(files), //, sub_name, sub_matches),
        _ => panic!(),
    }
}


fn cli_mode(files: Vec<File>, sub_name: &str, sub_matches: &clap::ArgMatches) {
    let group_no = sub_matches
        .value_of("group")
        .and_then(|o| o.parse::<usize>().ok());
    let track_no = sub_matches
        .value_of("track")
        .and_then(|o| o.parse::<i64>().ok());
    let set_default_value = sub_matches
        .value_of("set-default")
        .and_then(|o| o.parse::<i64>().ok())
        .map(|o| o != 0);
    let set_default_ex_value = sub_matches
        .value_of("set-default-ex")
        .and_then(|o| o.parse::<i64>().ok())
        .map(|o| o != 0);
    let set_forced_value = sub_matches
        .value_of("set-forced")
        .and_then(|o| o.parse::<i64>().ok())
        .map(|o| o != 0);
    let set_enabled_value = sub_matches
        .value_of("set-enabled")
        .and_then(|o| o.parse::<i64>().ok())
        .map(|o| o != 0);

    let track_type: TrackType = match sub_name {
        "subs" => TrackType::Subtitles,
        "audio" => TrackType::Audio,
        "video" => TrackType::Video,
        _ => panic!(),
    };

    let (sel_group, _groups) = match track_type {
        TrackType::Subtitles => {
            let groups = groupby(&files, key_sublang_subname);
            println!("SUBS");
            if let Some(group_no) = group_no {
                let group = groups.get(group_no).unwrap().clone();
                print_groups(&[group.clone()]);
                (Some(group), groups)
            } else {
                let mut files: Vec<&File> = Vec::new();
                groups.iter().for_each(|group| files.extend(&group.files));
                print_groups(&groups);
                (None, groups)
            }
        }
        TrackType::Audio => {
            let groups = groupby(&files, key_audlang_audname);
            println!("AUDIO");
            if let Some(group_no) = group_no {
                let group = groups.get(group_no).unwrap().clone();
                print_groups(&[group.clone()]);
                (Some(group), groups)
            } else {
                let mut files: Vec<&File> = Vec::new();
                groups.iter().for_each(|group| files.extend(&group.files));
                print_groups(&groups);
                (None, groups)
            }
        }
        TrackType::Video => return,
    };

    let sel_group = match sel_group {
        Some(group) => group,
        None => return,
    };

    if let Some(track_no) = track_no {
        let mut track_ops = TrackOperations::new(track_type);
        if let Some(set_default_value) = set_default_value {
            track_ops.add(track_no, TrackOperation::SetDefault(set_default_value))
        };
        if let Some(set_default_ex_value) = set_default_ex_value {
            if set_default_value.is_none() {
                track_ops.add(track_no, TrackOperation::SetDefaultExclusive(set_default_ex_value))
            } else {
                println!("Cannot use set-default-ex and set-default at the same time, exiting.");
                return;
            }
        };
        if let Some(set_forced_value) = set_forced_value {
            track_ops.add(track_no, TrackOperation::SetForced(set_forced_value))
        };
        if let Some(set_enabled_value) = set_enabled_value {
            track_ops.add(track_no, TrackOperation::SetEnabled(set_enabled_value))
        };
        // Generate and run commands
        let commands: Vec<Command> = sel_group
            .files
            .iter()
            .map(|file| track_ops.generate_command(file))
            .collect();
        commands.iter().for_each(|cmd| cmd.run());
    }
}

fn tui_mode(files: Vec<File>) {
    //, sub_name: &str, sub_matches: &clap::ArgMatches) {
    let groups_subs = groupby(&files, key_sublang_subname);
    let groups_audio = groupby(&files, key_audlang_audname);
    main_loop(&groups_subs, &groups_audio).unwrap();
}
