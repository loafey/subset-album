use crate::song_data::Song;
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum Info {
    PartialSubset(String, String, Vec<String>),
    Subset(String, String),
    Empty,
    MissingTitle(Vec<String>),
}

pub enum ClientMessage {
    ArtistLoading(usize, usize),
    InfoLoadingDone,
    InfoLoadingAdd,
    AddSong(String, String, Song),
    AddInfo(String, String, Info),
}

#[derive(Debug)]
pub enum InfoMessage {
    Analyze(String, BTreeMap<String, Vec<Song>>),
}
