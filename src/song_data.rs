use std::{cmp::Ordering, collections::BTreeMap, path::PathBuf};

pub type Artists = BTreeMap<Artist, Albums>;
pub type Albums = BTreeMap<String, Album>;
pub type Artist = String;
pub type Album = Vec<Song>;

#[derive(Clone, Debug)]
pub struct Song {
    pub name: String,
    pub path: PathBuf,
    pub unique: bool,
}
impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        !self.unique && !other.unique && self.name == other.name
    }
}
impl Eq for Song {}
impl PartialOrd for Song {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.name.cmp(&other.name))
    }
}
impl Ord for Song {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.unique || other.unique {
            true => Ordering::Equal,
            false => other.partial_cmp(other).unwrap(),
        }
    }
}

pub const MISSING: &str = "-- MISSING TITLE --";

pub fn is_song(end: &str) -> bool {
    end.ends_with(".3gp")
        || end.ends_with(".aa")
        || end.ends_with(".aac")
        || end.ends_with(".aax")
        || end.ends_with(".act")
        || end.ends_with(".aiff")
        || end.ends_with(".alac")
        || end.ends_with(".amr")
        || end.ends_with(".ape")
        || end.ends_with(".au")
        || end.ends_with(".awb")
        || end.ends_with(".dss")
        || end.ends_with(".dvf")
        || end.ends_with(".flac")
        || end.ends_with(".gsm")
        || end.ends_with(".iklax")
        || end.ends_with(".ivs")
        || end.ends_with(".m4a")
        || end.ends_with(".m4b")
        || end.ends_with(".m4p")
        || end.ends_with(".mmf")
        || end.ends_with(".movpkg")
        || end.ends_with(".mp3")
        || end.ends_with(".mpc")
        || end.ends_with(".msv")
        || end.ends_with(".nmf")
        || end.ends_with(".ogg")
        || end.ends_with(".opus")
        || end.ends_with(".ra")
        || end.ends_with(".rm")
        || end.ends_with(".raw")
        || end.ends_with(".rf64")
        || end.ends_with(".sln")
        || end.ends_with(".tta")
        || end.ends_with(".voc")
        || end.ends_with(".vox")
        || end.ends_with(".wav")
        || end.ends_with(".wma")
        || end.ends_with(".wv")
        || end.ends_with(".webm")
        || end.ends_with(".8svx")
        || end.ends_with(".cda")
}
