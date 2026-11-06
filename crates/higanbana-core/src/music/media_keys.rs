//! Teclas de mídia do teclado e o controle de mídia do Windows (SMTC), via souvlaki.
//!
//! Os controles vivem numa thread própria; o motor só manda atualizações (título,
//! tocando/pausado) e os eventos das teclas viram comandos do motor.

use std::{ffi::c_void, sync::Mutex, sync::mpsc};

use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use super::engine::{Command, Engine, PlayerState};

#[derive(Debug, Clone, PartialEq)]
struct Update {
    title: Option<String>,
    album: Option<String>,
    artist: Option<String>,
    playing: bool,
    active: bool,
}

pub struct MediaKeys(Mutex<mpsc::Sender<Update>>);

fn last_segment(p: &str) -> String {
    p.rsplit('/').next().unwrap_or(p).to_owned()
}

impl MediaKeys {
/// Liga os controles de mídia à janela (`hwnd` no Windows; nos outros sistemas, `None`).
pub fn start(hwnd: Option<usize>, engine: Engine) -> Self {
    let (tx, rx) = mpsc::channel::<Update>();
    std::thread::Builder::new()
        .name("music-media-keys".into())
        .spawn(move || {
            let config = PlatformConfig { display_name: "higanbana", dbus_name: "higanbana", hwnd: hwnd.map(|h| h as *mut c_void) };
            let Ok(mut controls) = MediaControls::new(config) else { return };
            let attached = controls.attach(move |e| {
                let cmd = match e {
                    MediaControlEvent::Play => Command::Resume,
                    MediaControlEvent::Pause => Command::Pause,
                    MediaControlEvent::Toggle => Command::Toggle,
                    MediaControlEvent::Next => Command::Next,
                    MediaControlEvent::Previous => Command::Prev,
                    MediaControlEvent::Stop => Command::Pause,
                    _ => return,
                };
                engine.send(cmd);
            });
            if attached.is_err() {
                return;
            }
            let mut last: Option<Update> = None;
            for u in rx {
                if last.as_ref() == Some(&u) {
                    continue;
                }
                if last.as_ref().is_none_or(|l| (&l.title, &l.album, &l.artist) != (&u.title, &u.album, &u.artist)) {
                    let _ = controls.set_metadata(MediaMetadata {
                        title: u.title.as_deref(),
                        album: u.album.as_deref(),
                        artist: u.artist.as_deref(),
                        ..Default::default()
                    });
                }
                let _ = controls.set_playback(match (u.active, u.playing) {
                    (false, _) => MediaPlayback::Stopped,
                    (true, true) => MediaPlayback::Playing { progress: None },
                    (true, false) => MediaPlayback::Paused { progress: None },
                });
                last = Some(u);
            }
        })
        .ok();
    MediaKeys(Mutex::new(tx))
}

/// Chamado a cada estado emitido pelo motor (a thread descarta repetidos).
pub fn update(&self, s: &PlayerState) {
    let title = s.title.clone().or_else(|| {
        s.path.as_deref().map(|p| {
            let name = last_segment(p);
            name.rsplit_once('.').map_or(name.clone(), |(b, _)| b.to_owned())
        })
    });
    let u = Update {
        title,
        album: s.album.as_deref().map(last_segment),
        artist: s.artist.clone(),
        playing: s.playing,
        active: s.album.is_some(),
    };
    let _ = self.0.lock().unwrap().send(u);
}
}
