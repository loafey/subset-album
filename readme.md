# subset-album

> [!WARNING]  
> The code is very messy :)

Very quick and dirty GUI tool I made to detect if your music collection contains songs contained by multiple albums.
Easiest way to run it is with `cargo run --release -- <path-to-collection>`.

Music collection must use the following directory structure:
```
Artist 1.
    - Album 
        - EpicSong
    - Album2Disk
        - CD1
            - SONG1
        - CD2
            - SONG2
    - Ep 
        - CoolSong2
    - Ep2
        - CoolSong
Artist 2.
    - CoolAlbum 
        - Song
    - CoolSingle
        - OtherSong 
```
Note that the tool only supports `.mp3`, `.flac` and MPEG-4 files, and that it simply checks the title of songs in the metadata. 
This means that if an artist has released two different songs with the same name, this tool will think they are the same.

## Info this tool can give you about an album
- Empty: this album contains no songs.
- Partial subset: some songs in this album also exist in another album.
- Subset: all songs in this album exist inside another album.
