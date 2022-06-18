# spotify-control
> Control Spotify from the command line!

This tool allows me to (relatively easily) control Spotify from the command line. 

The following commands are available: 
* `play-pause` starts or stops playback or music
* `next` goes to the next song
* `previous` goes to the previous song
* `now-playing` sends a notification of the song currently playing
* `play-song` allows you to play a song using the following options
  * `uri spotify:track:id` will play the track pointed to by `id`, artist and album work as well
  * `search name of song` will search spotify for a song matching the name, and play the first result. By using `search -l|--list name of song` you will get a simple selector where you can pick one of the first 5 results. Using `-c|--count n` in addition to `-l` you can instead display the first `n` songs.

With the flag `-s|--service-name` you can specify a different service to send the request to. Other mediaplayers (like vlc for instance)
migth use a similar api so they can be controlled using this program as well.

## Example
```sh
$ spotify-control play-pause

$ spotify-control play-song uri spotify:track:4cOdK2wGLETKBW3PvgPWqT

$ spotify-control play-song search megalovania
```

## Installing
After cloning the repo, using `cargo install --path .` can be used to install it to `$HOME/.cargo/bin`. So if that folder is
added to your path you can run it from everywhere. 

If you use Arch Linux, you can also install it from the AUR: https://aur.archlinux.org/packages/spotify-control

## Notes
This only works on Linux, since it uses the DBus api.

There's also no guarantee at all that the search actually works, it seems to be a bit finicky at times...
