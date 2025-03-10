use anyhow::Result;
use egui::{CollapsingHeader, Color32, RichText, ScrollArea};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

const ROOT: &str = "/home/loafey/BreadBox/Jellyfin/Music/Lidarr";

type Artists = BTreeMap<Artist, Albums>;
type Albums = BTreeMap<String, Album>;
type Artist = String;
type Album = BTreeSet<Song>;
type Song = String;

fn is_song(end: &str) -> bool {
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

fn get_data() -> Result<Artists> {
    let artists = fs::read_dir(ROOT)?;
    let mut top = Artists::new();
    for artist in artists {
        let mut albums_data = Albums::new();
        let artist = artist?;
        let artist_name = artist.file_name().to_string_lossy().to_string();

        let albums = fs::read_dir(artist.path())?;
        for album in albums {
            let mut album_data = Album::new();
            let album = album?;
            if album.path().is_file() {
                continue;
            }
            let album_name = album.file_name().to_string_lossy().to_string();

            let mut songs = fs::read_dir(album.path())?.collect::<Vec<_>>();
            while let Some(song) = songs.pop() {
                let song = song?;
                if song.path().is_dir() {
                    songs.extend(fs::read_dir(song.path())?);
                }
                let song_name = song.file_name().to_string_lossy().to_string();
                if is_song(&song_name) {
                    album_data.insert(song_name);
                }
            }

            albums_data.insert(album_name, album_data);
        }

        top.insert(artist_name, albums_data);
    }
    Ok(top)
}

#[derive(Debug)]
enum Info {
    PartialSubset(String, String, Vec<String>),
    Subset(String, String),
    Empty,
}
type InfoTree = BTreeMap<Artist, BTreeMap<String, Vec<Info>>>;
fn get_info(artists: &Artists) -> InfoTree {
    let mut top = InfoTree::new();

    for (artist, albums) in artists {
        let mut artist_info = BTreeMap::new();

        for (a, (album_a, songs_a)) in albums.iter().enumerate() {
            let mut album_info = Vec::new();

            // Try to find empty albums
            let mut is_empty = false;
            if songs_a.is_empty() {
                album_info.push(Info::Empty);
                is_empty = true;
            }

            // find subsets
            if !is_empty {
                for (b, (album_b, songs_b)) in albums.iter().enumerate() {
                    if a == b {
                        continue;
                    }

                    let mut overlaps = 0;
                    let mut song_overlaps = Vec::new();
                    for song in songs_a {
                        if songs_b.contains(song) {
                            overlaps += 1;
                            song_overlaps.push(song.clone());
                        }
                    }

                    if overlaps == songs_a.len() {
                        album_info.push(Info::Subset(album_a.clone(), album_b.clone()));
                    } else if overlaps > 0 {
                        album_info.push(Info::PartialSubset(
                            album_a.clone(),
                            album_b.clone(),
                            song_overlaps,
                        ));
                    }
                }
            }

            if !album_info.is_empty() {
                artist_info.insert(album_a.clone(), album_info);
            }
        }

        if !artist_info.is_empty() {
            top.insert(artist.clone(), artist_info);
        }
    }

    top
}

fn main() -> Result<()> {
    let artists = get_data()?;
    let info = get_info(&artists);
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "subset-album",
        native_options,
        Box::new(|_| Ok(Box::new(App { artists, info }))),
    )
    .unwrap();
    Ok(())
}

struct App {
    artists: Artists,
    info: InfoTree,
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .id_salt("all-albums")
                    .show(&mut ui[0], |ui| {
                        for (artist, albums) in &self.artists {
                            CollapsingHeader::new(artist)
                                .id_salt(format!("{artist}-info"))
                                .show(ui, |ui| {
                                    for (album, songs) in albums {
                                        ui.collapsing(album, |ui| {
                                            for song in songs {
                                                ui.label(song);
                                            }
                                        });
                                    }
                                });
                        }
                    });
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .id_salt("overlapps")
                    .show(&mut ui[1], |ui| {
                        for (artist, tree) in &self.info {
                            ui.collapsing(artist, |ui| {
                                for (album, fields) in tree {
                                    CollapsingHeader::new(album).default_open(true).show(
                                        ui,
                                        |ui| {
                                            for field in fields {
                                                let (text, color, bread) = match &field {
                                                    Info::PartialSubset(a, b, songs) => (
                                                        "Partial subset",
                                                        Color32::YELLOW,
                                                        format!(
                                                            "{a:?} is a partial subset of {b:?}\n\n{}",songs.iter().map(|s|format!("- {s}")).collect::<Vec<_>>().join(",")
                                                        ),
                                                    ),
                                                    Info::Subset(a, b) => (
                                                        "Subset",
                                                        Color32::GREEN,
                                                        format!("{a:?} is a subset of {b:?}"),
                                                    ),
                                                    Info::Empty => (
                                                        "Empty",
                                                        Color32::RED,
                                                        "this album contains no songs".to_string(),
                                                    ),
                                                };
                                                let label = RichText::new(text).color(color);
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(label);
                                                    ui.label(bread);
                                                });
                                            }
                                        },
                                    );
                                }
                            });
                        }
                    })
            });
        });
    }
}
