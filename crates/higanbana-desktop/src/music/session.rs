//! Estado do player de músicas (a barra e o encarte leem daqui).

use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};

use gpui_kit::Context;
use higanbana_core::music::{
    TrackMeta,
    album::{Album, Track},
    engine::PlayerState,
    queue::QueueTrack,
    store::MusicPrefs,
};

use crate::state::AppState;

/// Salvar onde parou a cada tanto enquanto toca.
const SAVE_EVERY: Duration = Duration::from_secs(5);

#[derive(Default)]
pub struct MusicSession {
    pub album: Option<Album>,
    pub metas: HashMap<String, TrackMeta>,
    queue: VecDeque<Track>,
    queued: HashSet<String>,
    running: bool,
    last_save: Option<Instant>,
}

impl AppState {
    pub fn music_open(&mut self, album: Album, index: usize, at: f64, cx: &mut Context<Self>) {
        let tracks = album.tracks.iter().map(|t| QueueTrack { path: t.path.clone(), size: t.size, ext: t.ext.clone() }).collect();
        self.core.music().play(album.path.clone(), tracks, index, at);
        self.music_session.album = Some(album);
        cx.notify();
    }

    pub fn music_command(&mut self, cmd: &str) {
        let _ = self.core.music().command(cmd);
    }

    pub fn is_current_album(&self, album: &Album) -> bool {
        self.music.as_ref().and_then(|m| m.album.as_deref()) == Some(album.path.as_str())
    }

    /// Encarte aberto: retoma o álbum tocando e busca as tags das faixas, uma por vez.
    pub fn music_show(&mut self, album: &Album, cx: &mut Context<Self>) {
        if self.is_current_album(album) && self.music_session.album.as_ref().is_none_or(|a| a.path != album.path) {
            self.music_session.album = Some(album.clone());
        }
        for t in &album.tracks {
            if !self.music_session.metas.contains_key(&t.path) && self.music_session.queued.insert(t.path.clone()) {
                self.music_session.queue.push_back(t.clone());
            }
        }
        self.pump_meta(cx);
    }

    fn pump_meta(&mut self, cx: &mut Context<Self>) {
        if self.music_session.running {
            return;
        }
        let Some(t) = self.music_session.queue.pop_front() else { return };
        self.music_session.running = true;
        let core = self.core.clone();
        let task = self.core.spawn(async move { (t.path.clone(), core.music().meta(&t.path, t.size, &t.ext, false).await) });
        cx.spawn(async move |this, cx| {
            let r = task.await.ok();
            let _ = this.update(cx, |s, cx| {
                s.music_session.running = false;
                if let Some((path, Ok(m))) = r {
                    s.music_session.metas.insert(path, m);
                    cx.notify();
                }
                s.pump_meta(cx);
            });
        })
        .detach();
    }

    pub fn on_music_state(&mut self, m: PlayerState, cx: &mut Context<Self>) {
        let prev = self.music.replace(m.clone());
        let changed = prev.as_ref().is_none_or(|p| p.index != m.index || p.playing != m.playing || p.ended != m.ended);
        let due = self.music_session.last_save.is_none_or(|t| t.elapsed() >= SAVE_EVERY);
        if (changed || (m.playing && due))
            && let (Some(album), Some(index)) = (self.music_session.album.as_ref(), m.index)
            && m.album.as_deref() == Some(album.path.as_str())
        {
            self.music_session.last_save = Some(Instant::now());
            // Álbum acabou: a próxima vez começa do início.
            let (i, at) = if m.ended { (0, 0.) } else { (index, m.position) };
            self.core.music().store.set_progress(&album.path, i, at);
        }
        if let Some(p) = prev
            && (p.volume != m.volume || p.shuffle != m.shuffle || p.repeat != m.repeat || p.normalize != m.normalize)
        {
            self.core.music().store.set_prefs(MusicPrefs { volume: m.volume, shuffle: m.shuffle, repeat: m.repeat, normalize: m.normalize });
        }
        cx.notify();
    }
}
