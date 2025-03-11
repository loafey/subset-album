use anyhow::Result;
use audiotags::Tag;
use egui::{CollapsingHeader, Color32, RichText, ScrollArea, TopBottomPanel, Ui};
use rayon::prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    env::args,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering::Relaxed},
        mpsc::{channel, Receiver, Sender},
    },
    thread,
    time::Duration,
};

type Artists = BTreeMap<Artist, Albums>;
type Albums = BTreeMap<String, Album>;
type Artist = String;
type Album = Vec<Song>;

#[derive(Clone)]
struct Song {
    name: String,
    path: PathBuf,
}
impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for Song {}
impl PartialOrd for Song {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.name.cmp(&other.name))
    }
}
impl Ord for Song {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.partial_cmp(other).unwrap()
    }
}

const MISSING: &str = "-- MISSING TITLE --";

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

fn get_data(sender: &mut Sender<Message>) -> Result<Artists> {
    let artists = fs::read_dir(args().nth(1).unwrap())?;
    let mut top = Artists::new();
    let mut total_songs = 0;
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
                    album_data.push(Song {
                        name: MISSING.to_string(),
                        path: song.path(),
                    });
                    total_songs += 1;
                }
            }

            albums_data.insert(album_name, album_data);
        }

        sender.send(Message::Loading(0, total_songs)).unwrap();

        top.insert(artist_name, albums_data);
    }

    let fixed = AtomicUsize::new(0);
    for (artist, albums) in top.iter_mut() {
        albums.iter_mut().par_bridge().for_each(|(album, songs)| {
            let mut new_songs = Vec::new();
            for Song { path, .. } in songs.clone() {
                let name = Tag::new()
                    .read_from_path(&path)
                    .ok()
                    .and_then(|v| v.title().map(|x| x.to_string()))
                    .unwrap_or(MISSING.to_string());
                sender
                    .send(Message::AddSong(
                        artist.clone(),
                        album.clone(),
                        Song {
                            name: name.clone(),
                            path: path.clone(),
                        },
                    ))
                    .unwrap();
                new_songs.push(Song { name, path });
                let s = fixed.fetch_add(1, Relaxed);
                sender.send(Message::Loading(s, total_songs)).unwrap();
            }

            *songs = new_songs;
        });
    }

    Ok(top)
}

#[derive(Debug)]
enum Info {
    PartialSubset(String, String, Vec<String>),
    Subset(String, String),
    Empty,
    MissingTitle(Vec<String>),
}
type InfoTree = BTreeMap<Artist, BTreeMap<String, Vec<Info>>>;
fn get_info(sender: &mut Sender<Message>, artists: &Artists) {
    let mut total_albums = 0;
    for albums in artists.values() {
        total_albums += albums.len();
        sender.send(Message::Loading(0, total_albums)).unwrap();
    }

    let mut current_album = 0;
    for (artist, albums) in artists {
        for (a, (album_a, songs_a)) in albums.iter().enumerate() {
            // Try to find empty albums
            let mut is_empty = false;
            if songs_a.is_empty() {
                sender
                    .send(Message::AddInfo(
                        artist.clone(),
                        album_a.clone(),
                        Info::Empty,
                    ))
                    .unwrap();
                is_empty = true;
            }

            // Find missing names
            let mut missing = Vec::new();
            for Song { name, path } in songs_a {
                if name == MISSING {
                    missing.push(path.to_string_lossy().to_string());
                }
            }
            if !missing.is_empty() {
                sender
                    .send(Message::AddInfo(
                        artist.clone(),
                        album_a.clone(),
                        Info::MissingTitle(missing),
                    ))
                    .unwrap();
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
                        sender
                            .send(Message::AddInfo(
                                artist.clone(),
                                album_a.clone(),
                                Info::Subset(album_a.clone(), album_b.clone()),
                            ))
                            .unwrap();
                    } else if overlaps > 0 {
                        sender
                            .send(Message::AddInfo(
                                artist.clone(),
                                album_a.clone(),
                                Info::PartialSubset(
                                    album_a.clone(),
                                    album_b.clone(),
                                    song_overlaps.into_iter().map(|s| s.name).collect(),
                                ),
                            ))
                            .unwrap();
                    }
                }
            }

            current_album += 1;
            sender
                .send(Message::Loading(current_album, total_albums))
                .unwrap();
        }
    }
}

fn main() -> Result<()> {
    let (mut sender, reciever) = channel();
    thread::spawn(move || {
        let artists = get_data(&mut sender).unwrap();
        get_info(&mut sender, &artists);
    });
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "subset-album",
        native_options,
        Box::new(|_| {
            Ok(Box::new(App {
                artists: Default::default(),
                info: Default::default(),
                reciever,
                loading_status: (0, usize::MAX),
            }))
        }),
    )
    .unwrap();
    Ok(())
}

enum Message {
    Loading(usize, usize),
    AddSong(String, String, Song),
    AddInfo(String, String, Info),
}

struct App {
    loading_status: (usize, usize),
    reciever: Receiver<Message>,
    artists: Artists,
    info: InfoTree,
}
impl App {
    fn draw_data(&self, ui: &mut Ui) {
        ui.columns(2, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .id_salt("all-albums")
                .show(&mut ui[0], |ui| {
                    ui.heading("All albums:");
                    for (artist, albums) in &self.artists {
                        CollapsingHeader::new(artist)
                            .id_salt(format!("{artist}-info"))
                            .show(ui, |ui| {
                                for (album, songs) in albums {
                                    ui.collapsing(album, |ui| {
                                        for song in songs {
                                            ui.label(&song.name);
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
                    ui.heading("Clean up work:");
                    for (artist, tree) in &self.info {
                        ui.collapsing(artist, |ui| {
                            for (album, fields) in tree {
                                CollapsingHeader::new(album)
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        for field in fields {
                                            let (text, color, bread) = match &field {
                                                Info::PartialSubset(a, b, songs) => (
                                                    "Partial subset",
                                                    Color32::YELLOW,
                                                    format!(
                                                        "{a:?} is a partial subset of {b:?}\n\n{}",
                                                        songs
                                                            .iter()
                                                            .map(|s| format!("- {s}"))
                                                            .collect::<Vec<_>>()
                                                            .join(",")
                                                    ),
                                                ),
                                                Info::MissingTitle(titles) => (
                                                    "Missing titles",
                                                    Color32::BLUE,
                                                    format!("\n{}", titles.join("\n\n")),
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
                                    });
                            }
                        });
                    }
                });
        });
    }

    fn progress_bar(&self, ui: &mut Ui) {
        let (cur, max) = self.loading_status;
        let p = if max != 0 && cur != 0 {
            let progress = cur as f32 / max as f32;
            let progress_bar_len = 20;
            (0..progress_bar_len)
                .map(|i| {
                    let percent = i as f32 / progress_bar_len as f32;
                    if percent < progress {
                        '◼'
                    } else {
                        '◻'
                    }
                })
                .collect::<String>()
        } else {
            String::new()
        };
        ui.heading(format!("{p} {cur}/{max}"));
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Ok(m) = self.reciever.try_recv() {
            match m {
                Message::Loading(a, b) => self.loading_status = (a, b),
                Message::AddInfo(artist, album, info) => {
                    self.info
                        .entry(artist)
                        .or_default()
                        .entry(album)
                        .or_default()
                        .push(info);
                }
                Message::AddSong(artist, album, song) => {
                    self.artists
                        .entry(artist)
                        .or_default()
                        .entry(album)
                        .or_default()
                        .push(song);
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.artists.is_empty() {
                ui.centered_and_justified(|ui| self.progress_bar(ui));
            } else {
                TopBottomPanel::top("top-panel").show(ctx, |ui| self.progress_bar(ui));
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.draw_data(ui);
                });
            }
        });
        ctx.request_repaint_after(Duration::from_secs_f64(0.066));
    }
}
