use anyhow::Result;
use audiotags::Tag;
use egui::{CollapsingHeader, Color32, FontId, RichText, ScrollArea, TopBottomPanel, Ui};
use rayon::prelude::*;
use std::{
    collections::BTreeMap,
    env::args,
    fs,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::Duration,
};

mod song_data;
use song_data::*;
mod messages;
use messages::*;

fn get_data(
    recv: Receiver<WorkMessage>,
    sender: &mut Sender<ClientMessage>,
    info_sender: &mut Sender<InfoMessage>,
) -> Result<()> {
    loop {
        let mut top = Artists::new();
        loop {
            let WorkMessage::WorkOnFolder(artist) = match recv.recv_timeout(Duration::from_secs(1))
            {
                Ok(o) => o,
                Err(e) => match e {
                    std::sync::mpsc::RecvTimeoutError::Timeout => break,
                    std::sync::mpsc::RecvTimeoutError::Disconnected => return Ok(()),
                },
            };
            let mut albums_data = Albums::new();
            let artist_name = artist.file_name().unwrap().to_string_lossy().to_string();

            let albums = fs::read_dir(artist)?;
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
                        continue;
                    }
                    let song_name = song.file_name().to_string_lossy().to_string();
                    if is_song(&song_name) {
                        album_data.push(Song {
                            name: MISSING.to_string(),
                            path: song.path(),
                            unique: false,
                        });
                        sender.send(ClientMessage::ArtistLoadingAdd).unwrap();
                    }
                }

                albums_data.insert(album_name, album_data);
            }

            top.insert(artist_name, albums_data);
        }

        for (artist, mut albums) in top {
            albums.iter_mut().par_bridge().for_each(|(album, songs)| {
                let mut new_songs = Vec::new();
                for Song { path, .. } in songs.clone() {
                    let tag = Tag::new().read_from_path(&path).ok();
                    let name = tag
                        .as_ref()
                        .and_then(|v| v.title().map(|x| x.to_string()))
                        .unwrap_or(MISSING.to_string());
                    let unique = tag
                        .as_ref()
                        .and_then(|v| v.comment().map(|x| x == "unique"))
                        .unwrap_or_default();
                    let data = Song {
                        name: name.clone(),
                        path: path.clone(),
                        unique,
                    };
                    sender
                        .send(ClientMessage::AddSong(
                            artist.clone(),
                            album.clone(),
                            data.clone(),
                        ))
                        .unwrap();
                    new_songs.push(data);
                }

                *songs = new_songs;
            });
            sender.send(ClientMessage::InfoLoadingAdd).unwrap();
            info_sender
                .send(InfoMessage::Analyze(artist.clone(), albums.clone()))
                .unwrap();
        }
    }
}

type InfoTree = BTreeMap<Artist, BTreeMap<String, Vec<Info>>>;
fn get_info(
    sender: &mut Sender<ClientMessage>,
    artist: String,
    albums: BTreeMap<String, Vec<Song>>,
) {
    for (a, (album_a, songs_a)) in albums.iter().enumerate() {
        // Try to find empty albums
        let mut is_empty = false;
        if songs_a.is_empty() {
            sender
                .send(ClientMessage::AddInfo(
                    artist.clone(),
                    album_a.clone(),
                    Info::Empty,
                ))
                .unwrap();
            is_empty = true;
        }

        // Find missing names
        let mut missing = Vec::new();
        for Song { name, path, .. } in songs_a {
            if name == MISSING {
                missing.push(path.to_string_lossy().to_string());
            }
        }
        if !missing.is_empty() {
            sender
                .send(ClientMessage::AddInfo(
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
                        .send(ClientMessage::AddInfo(
                            artist.clone(),
                            album_a.clone(),
                            Info::Subset(album_a.clone(), album_b.clone()),
                        ))
                        .unwrap();
                } else if overlaps > 0 {
                    sender
                        .send(ClientMessage::AddInfo(
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
    }
    sender.send(ClientMessage::InfoLoadingDone).unwrap();
}

fn main() -> Result<()> {
    let (mut sender, reciever) = channel();
    let (work_sender, work_reciever) = channel();
    let (info_sender, info_reciever) = channel();
    thread::spawn({
        let work_sender = work_sender.clone();
        let sender = sender.clone();
        move || {
            let artists = fs::read_dir(args().nth(1).unwrap()).unwrap();
            for artist in artists {
                let artist = artist.unwrap();
                sender
                    .send(ClientMessage::AddArtistPath(
                        artist.file_name().to_str().unwrap().to_string(),
                        artist.path(),
                    ))
                    .unwrap();
                work_sender
                    .send(WorkMessage::WorkOnFolder(artist.path()))
                    .unwrap();
            }
        }
    });
    thread::spawn({
        let mut sender = sender.clone();
        let mut info_sender = info_sender.clone();
        move || get_data(work_reciever, &mut sender, &mut info_sender).unwrap()
    });
    thread::spawn(move || {
        while let Ok(m) = info_reciever.recv() {
            match m {
                InfoMessage::Analyze(art, m) => get_info(&mut sender, art, m),
            }
        }
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
                artist_loading_status: (0, 0),
                info_loading_status: (0, 0),
                work_sender,
                artist_paths: Default::default(),
            }))
        }),
    )
    .unwrap();
    Ok(())
}

struct App {
    artist_loading_status: (usize, usize),
    info_loading_status: (usize, usize),
    reciever: Receiver<ClientMessage>,
    work_sender: Sender<WorkMessage>,
    artists: Artists,
    info: InfoTree,
    artist_paths: BTreeMap<String, PathBuf>,
}
impl App {
    fn draw_data(&mut self, ui: &mut Ui) {
        ui.columns(2, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .id_salt("all-albums")
                .show(&mut ui[0], |ui| {
                    ui.heading("All artists:");
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
                    ui.heading("Potential problems:");
                    for (artist, tree) in self.info.clone() {
                        ui.collapsing(&artist, |ui| {
                            if ui.button("Reload").clicked() {
                                let v = self.artists.remove(&artist);
                                let _ = self.info.remove(&artist);
                                let songs = v
                                    .map(|v| v.into_values().map(|v| v.len()).sum::<usize>())
                                    .unwrap_or_default();
                                self.artist_loading_status.0 -= songs;
                                self.artist_loading_status.1 -= songs;
                                self.info_loading_status.0 -= 1;
                                self.info_loading_status.1 -= 1;
                                self.work_sender
                                    .send(WorkMessage::WorkOnFolder(
                                        self.artist_paths.get(&artist).unwrap().clone(),
                                    ))
                                    .unwrap();
                            }
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
                                                    Color32::RED,
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

    fn progress_bar(&self, ui: &mut Ui, title: &str, (cur, max): (usize, usize)) {
        let progress_bar_len = 20;
        let p = if max != 0 && cur != 0 {
            let progress = cur as f32 / max as f32;
            (0..progress_bar_len)
                .map(|i| {
                    let percent = i as f32 / progress_bar_len as f32;
                    if percent < progress {
                        '█'
                    } else {
                        '░'
                    }
                })
                .collect::<String>()
        } else {
            "░".repeat(progress_bar_len)
        };
        let text =
            RichText::new(format!("{title}: ▟{p}▛ {cur}/{max}")).font(FontId::monospace(16.0));
        ui.heading(text);
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mut i = 0;
        while let Ok(m) = self.reciever.try_recv() {
            match m {
                ClientMessage::ArtistLoadingAdd => self.artist_loading_status.1 += 1,
                ClientMessage::InfoLoadingAdd => self.info_loading_status.1 += 1,
                ClientMessage::InfoLoadingDone => self.info_loading_status.0 += 1,
                ClientMessage::AddArtistPath(artist, path) => {
                    self.artist_paths.insert(artist, path);
                }
                ClientMessage::AddInfo(artist, album, info) => {
                    self.info
                        .entry(artist)
                        .or_default()
                        .entry(album)
                        .or_default()
                        .push(info);
                }
                ClientMessage::AddSong(artist, album, song) => {
                    self.artist_loading_status.0 += 1;
                    self.artists
                        .entry(artist)
                        .or_default()
                        .entry(album)
                        .or_default()
                        .push(song);
                }
            }

            i += 1;
            if i > 60 {
                break;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.artists.is_empty() {
                ui.centered_and_justified(|ui| {
                    self.progress_bar(ui, "Mapping songs", self.artist_loading_status);
                });
            } else {
                TopBottomPanel::top("top-panel").show(ctx, |ui| {
                    self.progress_bar(ui, "Mapping artists", self.artist_loading_status);
                    self.progress_bar(ui, " Finding faults", self.info_loading_status);
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.draw_data(ui);
                });
            }
        });
        ctx.request_repaint_after(Duration::from_secs_f64(0.066));
    }
}
