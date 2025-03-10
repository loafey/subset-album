# subset-album
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