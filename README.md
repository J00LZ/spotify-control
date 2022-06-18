# spotify-control
> Control Spotify from the command line!

This tool allows me to (relatively easily) control Spotify from the command line. 

The following 4 commands are available: 
* `play-pause` starts or stops playback or music
* `next` goes to the next song
* `previous` goes to the previous song
* `now-playing` sends a notification of the song currently playing

With the flag `-s|--service-name` you can specify a different service to send the request to. Other mediaplayers (like vlc for instance)
migth use a simmilar api so they can be controlled using this program as well.

## Example
```sh
$ spotify-control play-pause
```

## Installing
After cloning the repo, using `cargo install --path .` can be used to install it to `$HOME/.cargo/bin`. So if that folder is
added to your path you can run it from everywhere. 

## Notes
This only works on Linux, since it uses the DBus api.