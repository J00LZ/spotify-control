use std::{collections::HashMap, fmt::Display, io::Write, vec};

use clap::{Parser, Subcommand};

use notify_rust::{Hint, Notification};
use serde::{Deserialize, Serialize};
use zbus::{
    dbus_proxy,
    zvariant::{OwnedValue, Value},
};

#[dbus_proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2",
    default_service = "org.mpris.MediaPlayer2.spotify"
)]
trait Player {
    fn play_pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;
    fn previous(&self) -> zbus::Result<()>;
    fn open_uri(&self, uri: &str) -> zbus::Result<()>;
    #[dbus_proxy(property)]
    fn metadata(&self) -> zbus::Result<Metadata>;
}

#[derive(Debug)]
pub enum Error {
    ZbusError(zbus::Error),
    MetadataError(MetadataError),
}

#[derive(Debug, Clone)]
pub enum MetadataError {
    MissingKey(String),
    InvalidValueType(String),
}

#[derive(Debug)]
pub struct Metadata {
    r#title: String,
    artists: Vec<String>,
    album: String,
    artwork: String,
}

impl TryInto<OwnedValue> for Metadata {
    type Error = zbus::Error;
    fn try_into(self) -> zbus::Result<OwnedValue> {
        let mut map = HashMap::new();
        map.insert("xesam:title".to_string(), Value::new(self.title));
        map.insert("xesam:artist".to_string(), Value::new(self.artists));
        map.insert("xesam:album".to_string(), Value::new(self.album));
        map.insert("mpris:artUrl".to_string(), Value::new(self.artwork));
        Ok(Value::Dict(map.into()).into())
    }
}

impl Into<Metadata> for OwnedValue {
    fn into(self) -> Metadata {
        let mut map: HashMap<String, Value<'_>> = HashMap::new();
        if let Value::Dict(dict) = self.into() {
            map = dict.try_into().unwrap();
        }
        let title = map.get("xesam:title").cloned().unwrap().downcast().unwrap();
        let artists = map
            .get("xesam:artist")
            .cloned()
            .unwrap()
            .downcast()
            .unwrap();
        let album = map.get("xesam:album").cloned().unwrap().downcast().unwrap();
        let artwork = map
            .get("mpris:artUrl")
            .cloned()
            .unwrap()
            .downcast()
            .unwrap();

        Metadata {
            title,
            artists,
            album,
            artwork,
        }
    }
}

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

        #[clap(short, long, default_value = "5")]
        count: usize,
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

async fn search(query: &str) -> Vec<Track> {
    let url = format!(
        "https://spotify-search-api-test.herokuapp.com/search/tracks?track={}",
        query.replace(' ', "%20")
    );
    let res: Response = reqwest::get(&url).await.unwrap().json().await.unwrap();
    res.tracks.items
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let conn = zbus::Connection::session().await.unwrap();

    let proxy = PlayerProxy::builder(&conn)
        .destination(args.service_name)
        .unwrap()
        .build()
        .await
        .unwrap();

    match args.action {
        Commands::Next => proxy.next().await.unwrap(),
        Commands::Previous => proxy.previous().await.unwrap(),
        Commands::PlayPause => proxy.play_pause().await.unwrap(),
        Commands::NowPlaying => what(proxy.metadata().await.unwrap().try_into().unwrap()).await,
        Commands::PlaySong { mode } => play_song(&proxy, mode).await,
    }
}

async fn play_song<'proxy>(proxy: &PlayerProxy<'proxy>, mode: PlayMode) {
    match mode {
        PlayMode::Uri { uri } => proxy.open_uri(&uri).await.unwrap(),
        PlayMode::Search { query, list, count } => {
            let query = query.join(" ");
            let track = search(&query).await;
            if list {
                for (i, track) in track.iter().take(count).enumerate() {
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
                proxy.open_uri(&uri).await.unwrap()
            } else if let Some(track) = track.first() {
                println!("Playing {}", track);
                let uri = format!("spotify:track:{}", track.id);
                proxy.open_uri(&uri).await.unwrap()
            } else {
                println!("No track found for {}", query);
            }
        }
    }
}

async fn what(metadata: Metadata) {
    let res = reqwest::get(&metadata.artwork).await.unwrap();
    let bytes = res.bytes().await.unwrap();
    let tmp = temp_file::with_contents(&bytes);

    let _not = Notification::new()
        .appname("Spotify Notify")
        .summary(&metadata.title)
        .body(&format!(
            "{} - {}",
            metadata.artists.join(", "),
            metadata.album
        ))
        .image_path(tmp.path().to_str().unwrap())
        .hint(Hint::Category("music".to_string()))
        .show()
        .unwrap();
}
