use std::{io::Read, time::Duration, vec};

use clap::{Parser, ValueEnum};
use dbus::{
    arg,
    blocking::{stdintf::org_freedesktop_dbus::Properties, Connection, Proxy},
};

use notify_rust::{Hint, Notification};

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum)]
enum Action {
    Next,
    Previous,
    PlayPause,
    NowPlaying,
}

#[derive(Debug, Parser)]
#[clap(author, about, version, long_about = None)]
struct Args {
    #[clap(
        short,
        long,
        value_parser,
        default_value = "org.mpris.MediaPlayer2.spotify"
    )]
    service_name: String,

    #[clap(value_parser)]
    action: Action,
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
        Action::Next => send_command(&proxy, "Next"),
        Action::Previous => send_command(&proxy, "Previous"),
        Action::PlayPause => send_command(&proxy, "PlayPause"),
        Action::NowPlaying => what(proxy),
    }
}

fn send_command(proxy: &Proxy<&Connection>, command: &str) {
    let _: () = proxy
        .method_call("org.mpris.MediaPlayer2.Player", command, ())
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
