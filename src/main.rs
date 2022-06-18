use std::{
    fmt::Display,
    io::{Read, Write},
    time::Duration,
    vec,
};

use clap::{Parser, Subcommand};
use dbus::{
    arg,
    blocking::{stdintf::org_freedesktop_dbus::Properties, Connection, Proxy},
};

use notify_rust::{Hint, Notification};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
enum Commands {
    /// Play the next song
    Next,
    /// Play the previous song
    Previous,
    /// Play/Pause the current song
    PlayPause,
    /// Show a notification with the current song
    NowPlaying,
    /// Play a song
    PlaySong {
        #[clap(subcommand)]
        mode: PlayMode,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
enum PlayMode {
    Uri {
        /// A uri in the format of spotify:track:<id>
        uri: String,
    },
    Search {
        /// You get the best success with "search title artist"
        query: Vec<String>,

        /// Allows picking from a list of songs instead of starting the first
        #[clap(short, long, action)]
        list: bool,
    },
}

#[derive(Debug, Parser)]
#[clap(author, about, version, long_about = None)]
struct Args {
    /// Changes the service that the DBus commands are sent to
    /// If changed, the play-song commands won't work, and the now-playing might not work
    ///
    #[clap(
        short,
        long,
        value_parser,
        default_value = "org.mpris.MediaPlayer2.spotify"
    )]
    service_name: String,

    #[clap(subcommand)]
    action: Commands,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    tracks: Tracks,
}

#[derive(Serialize, Deserialize, Debug)]
struct Tracks {
    items: Vec<Track>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Track {
    name: String,
    id: String,
    artists: Vec<Artist>,
    album: Album,
}

impl Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let artists = self
            .artists
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>();
        let (last, start) = artists.split_last().unwrap();
        let artists = start.join(", ");
        let artist = if artists.is_empty() {
            last.to_string()
        } else {
            format!("{} and {}", artists, last)
        };
        write!(f, "{} by {} on {}", self.name, artist, self.album.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Artist {
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Album {
    name: String,
}

fn search(query: &str) -> Vec<Track> {
    let url = format!(
        "https://spotify-search-api-test.herokuapp.com/search/tracks?track={}",
        query.replace(' ', "%20")
    );
    let res = ureq::get(&url)
        .call()
        .unwrap()
        .into_json::<Response>()
        .unwrap();
    res.tracks.items
}

fn main() {
    let args = Args::parse();

    let conn = dbus::blocking::Connection::new_session().unwrap();
    let proxy = conn.with_proxy(
        args.service_name,
        "/org/mpris/MediaPlayer2",
        Duration::from_secs(5),
    );
    match args.action {
        Commands::Next => send_command(&proxy, "Next"),
        Commands::Previous => send_command(&proxy, "Previous"),
        Commands::PlayPause => send_command(&proxy, "PlayPause"),
        Commands::NowPlaying => what(proxy),
        Commands::PlaySong { mode } => play_song(&proxy, mode),
    }
}

fn play_song(proxy: &Proxy<&Connection>, mode: PlayMode) {
    match mode {
        PlayMode::Uri { uri } => {
            send_command_with_args(proxy, "OpenUri", uri);
        }
        PlayMode::Search { query, list } => {
            let query = query.join(" ");
            let track = search(&query);
            if list {
                for (i, track) in track.iter().take(5).enumerate() {
                    println!("{} - {}", i, track);
                }
                print!("Enter a number to play: ");
                std::io::stdout().flush().unwrap();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                let input = input.trim().parse::<usize>().unwrap();
                let track = track.get(input).unwrap();
                println!("Playing {}", track);
                let uri = format!("spotify:track:{}", track.id);
                send_command_with_args(proxy, "OpenUri", uri);
            } else if let Some(track) = track.first() {
                println!("Playing {}", track);
                let uri = format!("spotify:track:{}", track.id);
                send_command_with_args(proxy, "OpenUri", uri);
            } else {
                println!("No track found for {}", query);
            }
        }
    }
}

fn send_command(proxy: &Proxy<&Connection>, command: &str) {
    let _: () = proxy
        .method_call("org.mpris.MediaPlayer2.Player", command, ())
        .unwrap();
}

fn send_command_with_args(proxy: &Proxy<&Connection>, command: &str, arg: String) {
    let _: () = proxy
        .method_call("org.mpris.MediaPlayer2.Player", command, (arg,))
        .unwrap();
}

fn get_value<T: 'static + Clone>(map: &arg::PropMap, key: &str, default: T) -> T {
    let res = arg::prop_cast::<T>(map, key);
    if let Some(res) = res {
        res.clone()
    } else {
        default
    }
}

fn what(proxy: Proxy<&Connection>) {
    let metadata: arg::PropMap = proxy
        .get("org.mpris.MediaPlayer2.Player", "Metadata")
        .unwrap();

    let title = get_value(&metadata, "xesam:title", "Unknown".to_string());
    let artist: Vec<String> = get_value(&metadata, "xesam:artist", vec!["Unknown".to_string()]);
    let album = get_value(&metadata, "xesam:album", "Unknown".to_string());
    let cover = get_value(
        &metadata,
        "mpris:artUrl",
        "https://www.scdn.co/i/_global/touch-icon-144.png".to_string(),
    );

    let res = ureq::get(&cover).call().unwrap();
    let mut buffer = vec![];
    res.into_reader().read_to_end(&mut buffer).unwrap();
    let tmp = temp_file::with_contents(&buffer);

    let _not = Notification::new()
        .appname("Spotify Notify")
        .summary(&title)
        .body(&format!("{} - {}", artist.join(", "), album))
        .image_path(tmp.path().to_str().unwrap())
        .hint(Hint::Category("music".to_string()))
        .show()
        .unwrap();
}
