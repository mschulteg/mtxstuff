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
    pub language: String,
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

impl File {
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
        let language = properties.get("language")?.as_str()?;
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
            language: String::from(language),
            ttype,
            id,
            default,
            forced,
            enabled,
        })
    }
}