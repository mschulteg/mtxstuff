use std;
use std::path::Path;
use std::process;

use serde_json::Value;

#[derive(Clone, Copy, Debug)]
pub enum TrackType {
    Video,
    Audio,
    Subtitles,
}

#[derive(Clone, Copy)]
pub enum Flag {
    Default,
    Forced,
    Enabled,
}

#[derive(Debug)]
pub struct Track {
    pub name: Option<String>,
    pub language: Option<String>,
    pub ttype: TrackType,
    pub id: i64,
    pub default: bool,
    pub forced: bool,
    pub enabled: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct File {
    pub video_tracks: Vec<Track>,
    pub audio_tracks: Vec<Track>,
    pub subtitle_tracks: Vec<Track>,
    pub file_name: String,
    pub json: Value,
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.file_name == other.file_name
    }
}

#[derive(Debug, Clone)]
struct IdentifyStructureError;
impl std::fmt::Display for IdentifyStructureError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Missing or wrong shaped keys/values in mkvmerge identify output"
        )
    }
}

use anyhow::{Context, Result};
impl std::error::Error for IdentifyStructureError {}

impl File {
    pub fn from_path(path: &Path) -> Result<Self> {
        let json_bytes = process::Command::new("mkvmerge")
            .arg("--identification-format")
            .arg("json")
            .arg("--identify")
            .arg(path)
            .output()
            .context("Calling mkvmerge failed")?
            .stdout;
        let json_str = std::str::from_utf8(&json_bytes[..])?;
        let json_val = serde_json::from_str(json_str)?;
        if let Some(file) = Self::from_json(json_val) {
            Ok(file)
        } else {
            Err(IdentifyStructureError.into())
        }
    }

    pub fn from_json_str(json_str: &str) -> Option<Self> {
        let json_val = serde_json::from_str(json_str).unwrap();
        Self::from_json(json_val)
    }

    pub fn from_json(json: Value) -> Option<Self> {
        let mut video_tracks = Vec::<Track>::new();
        let mut audio_tracks = Vec::<Track>::new();
        let mut subtitle_tracks = Vec::<Track>::new();
        for value in json.get("tracks")?.as_array()? {
            let track = Track::from_json(value)?;
            match track.ttype {
                TrackType::Video => video_tracks.push(track),
                TrackType::Audio => audio_tracks.push(track),
                TrackType::Subtitles => subtitle_tracks.push(track),
            }
        }
        let file_name = String::from(json.get("file_name")?.as_str()?);
        Some(File {
            video_tracks,
            audio_tracks,
            subtitle_tracks,
            file_name,
            json,
        })
    }
}

impl Track {
    pub fn from_json(json: &Value) -> Option<Self> {
        let properties = json.get("properties")?;
        let name = properties
            .get("track_name")
            .and_then(|t| t.as_str())
            .map(String::from);
        let language = properties.get("language")?.as_str()?.to_string();
        let language = if language == "und" {
            None
        } else {
            Some(language)
        };
        let default = properties.get("default_track")?.as_bool()?;
        let forced = properties.get("forced_track")?.as_bool()?;
        let enabled = properties.get("enabled_track")?.as_bool()?;
        let id = json.get("id")?.as_i64()?;
        let ttype = json.get("type")?.as_str()?;
        let ttype = match ttype {
            "audio" => TrackType::Audio,
            "video" => TrackType::Video,
            "subtitles" => TrackType::Subtitles,
            _ => return None,
        };
        Some(Track {
            name,
            language,
            ttype,
            id,
            default,
            forced,
            enabled,
        })
    }
}
