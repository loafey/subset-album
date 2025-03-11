use crate::song_data::Song;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone)]
pub enum Info {
    PartialSubset(String, String, Vec<String>),
    Subset(String, String),
    Empty,
    MissingTitle(Vec<String>),
}

#[derive(Debug)]
pub enum ClientMessage {
    ArtistLoadingAdd,
    InfoLoadingDone,
    InfoLoadingAdd,
    AddArtistPath(String, PathBuf),
    AddSong(String, String, Song),
    AddInfo(String, String, Info),
}

pub enum InfoMessage {
    Analyze(String, BTreeMap<String, Vec<Song>>),
}

pub enum WorkMessage {
    WorkOnFolder(PathBuf),
}
