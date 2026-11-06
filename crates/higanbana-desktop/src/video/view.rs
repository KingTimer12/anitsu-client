//! Tela do player ("Imersivo"): o mpv desenha na janela filha embaixo e esta
//! tela é só a interface por cima.

use std::{
    cell::Cell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::StreamExt;
use gpui::{
    Bounds, ClickEvent, Context, CursorStyle, Div, EventEmitter, FocusHandle, Focusable, FontWeight, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render,
    SharedString, Stateful, Styled, Task, Window, canvas, div, linear_color_stop, linear_gradient, prelude::*, px,
    relative,
};
use higanbana_core::{
    Core,
    library::{join_path, parse_episode},
    nuvem::FileEntry,
    player::{
        aniskip::SkipTimes,
        logic::{
            AdvanceState, Chapter, SubPick, Track, TrackKind, can_auto_advance, classify_chapters, episode_label,
            episode_number, estimate_ending, format_time, is_watched, next_card_start, next_episode, pick_tracks,
            retry_delay, video_files,
        },
        store::{PlayerData, Progress, Settings, TrackPref},
    },
};
use serde_json::Value;

use super::{
    mpv::{EndReason, Mpv, MpvEvent, Observe},
    surface::Surface,
};
use crate::{
    theme,
    ui::{button, icon, icon_button, icon_toggle, spinner, truncate},
};

const HIDE_AFTER: Duration = Duration::from_millis(2500);
/// Progresso gravado em disco no máximo a cada tanto (e sempre ao pausar/sair/trocar).
const SAVE_EVERY: Duration = Duration::from_secs(5);

/// O que está tocando. `dir` é a pasta do release (chave das faixas lembradas).
#[derive(Debug, Clone)]
pub struct PlayTarget {
    pub dir: String,
    pub file: FileEntry,
    pub files: Vec<FileEntry>,
    pub mal_id: Option<u32>,
    /// Seção `Animes/` (consulta o AniSkip); doramas e desenhos não.
    pub is_anime: bool,
    pub anime_title: String,
}

pub enum PlayerEvent {
    Close,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Menu {
    Tracks,
    Settings,
    Episodes,
}

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Seek(f64),
    Volume,
}

/// Opções fixas: nada da interface do próprio mpv, cache só em RAM.
fn base_options(wid: usize, cache_mib: u32) -> Vec<(&'static str, String)> {
    vec![
        ("wid", wid.to_string()),
        ("vo", "gpu-next".into()),
        ("hwdec", "auto-safe".into()),
        ("keep-open", "yes".into()),
        ("force-window", "yes".into()),
        ("osc", "no".into()),
        ("osd-level", "0".into()),
        ("input-default-bindings", "no".into()),
        ("input-vo-keyboard", "no".into()),
        ("input-cursor", "no".into()),
        ("cursor-autohide", "no".into()),
        ("cache", "yes".into()),
        ("cache-on-disk", "no".into()),
        ("demuxer-max-bytes", format!("{cache_mib}MiB")),
        ("demuxer-max-back-bytes", "50MiB".into()),
        ("sub-auto", "no".into()),
    ]
}

const OBSERVED: [(&str, Observe); 10] = [
    ("time-pos", Observe::Double),
    ("duration", Observe::Double),
    ("pause", Observe::Flag),
    ("volume", Observe::Double),
    ("mute", Observe::Flag),
    ("paused-for-cache", Observe::Flag),
    ("demuxer-cache-time", Observe::Double),
    ("chapter-list", Observe::Node),
    ("track-list", Observe::Node),
    ("eof-reached", Observe::Flag),
];

pub struct PlayerView {
    core: Arc<Core>,
    target: PlayTarget,
    surface: Rc<Surface>,
    mpv: Option<Mpv>,
    focus: FocusHandle,
    data: PlayerData,

    pos: f64,
    dur: f64,
    paused: bool,
    volume: f64,
    muted: bool,
    buffering: bool,
    cache_ahead: f64,
    chapters: Vec<Chapter>,
    tracks: Vec<Track>,
    eof: bool,
    menu: Option<Menu>,
    idle: bool,
    net_error: bool,
    fatal: bool,
    /// Falha ao iniciar o mpv (DLL ausente etc.).
    broken: Option<SharedString>,
    skip: SkipTimes,
    next_cancelled: bool,
    /// Arquivo atual aberto pelo mpv (`file-loaded`). Antes disso, eventos podem ser do anterior.
    loaded: bool,
    expected_chapters: usize,
    skip_resolved_for: String,
    load_seq: u64,
    tracks_applied_for: u64,
    /// Arquivo de onde já saímos para o próximo: evita avançar duas vezes.
    advanced_from: Option<String>,
    playing_path: String,
    retry_attempt: usize,
    retry_task: Option<Task<()>>,
    hide_task: Option<Task<()>>,
    /// Segundos restantes da contagem do próximo episódio.
    countdown: Option<f64>,
    countdown_task: Option<Task<()>>,
    last_saved: Instant,
    drag: Option<Drag>,
    seek_bounds: Rc<Cell<Bounds<Pixels>>>,
    volume_bounds: Rc<Cell<Bounds<Pixels>>>,
    hover: Option<(Pixels, f64)>,
    /// Última posição do mouse (o GPUI repete o evento a cada quadro).
    last_mouse: Option<gpui::Point<Pixels>>,
    _events: Option<Task<()>>,
}

impl EventEmitter<PlayerEvent> for PlayerView {}

impl Focusable for PlayerView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl PlayerView {
    pub fn new(core: Arc<Core>, target: PlayTarget, surface: Rc<Surface>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let data = core.player().data();
        let focus = cx.focus_handle();
        window.focus(&focus);
        let mut this = Self {
            core,
            target,
            surface,
            mpv: None,
            focus,
            data,
            pos: 0.,
            dur: 0.,
            paused: false,
            volume: 100.,
            muted: false,
            buffering: true,
            cache_ahead: 0.,
            chapters: Vec::new(),
            tracks: Vec::new(),
            eof: false,
            menu: None,
            idle: false,
            net_error: false,
            fatal: false,
            broken: None,
            skip: SkipTimes::default(),
            next_cancelled: false,
            loaded: false,
            expected_chapters: 0,
            skip_resolved_for: String::new(),
            load_seq: 0,
            tracks_applied_for: u64::MAX,
            advanced_from: None,
            playing_path: String::new(),
            retry_attempt: 0,
            retry_task: None,
            hide_task: None,
            countdown: None,
            countdown_task: None,
            last_saved: Instant::now(),
            drag: None,
            seek_bounds: Rc::default(),
            volume_bounds: Rc::default(),
            hover: None,
            last_mouse: None,
            _events: None,
        };
        match Mpv::new(&base_options(this.surface.wid(), this.data.settings.cache_mib), &OBSERVED) {
            Ok((mpv, mut rx)) => {
                this.mpv = Some(mpv);
                this._events = Some(cx.spawn(async move |this, cx| {
                    while let Some(e) = rx.next().await {
                        if this.update(cx, |v, cx| v.on_mpv(e, cx)).is_err() {
                            break;
                        }
                    }
                }));
                this.load_current(None, cx);
            }
            Err(e) => this.broken = Some(e.into()),
        }
        this.poke(cx);
        this
    }

    fn file(&self) -> &FileEntry {
        &self.target.file
    }

    fn remote_path(&self) -> String {
        join_path(&self.target.dir, &self.file().name)
    }

    fn next(&self) -> Option<FileEntry> {
        next_episode(&self.target.files, &self.file().name)
    }

    fn settings(&self) -> &Settings {
        &self.data.settings
    }

    // --- Progresso ---

    fn record_progress(&mut self, force: bool) {
        if self.dur <= 0. || self.playing_path.is_empty() {
            return;
        }
        let p = Progress {
            position: self.pos,
            duration: self.dur,
            watched: is_watched(self.pos, self.dur, self.skip.ed),
            updated_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64),
        };
        self.data.progress.insert(self.playing_path.clone(), p.clone());
        if force || self.last_saved.elapsed() >= SAVE_EVERY {
            self.last_saved = Instant::now();
            self.core.player().save_progress(self.playing_path.clone(), p);
        }
    }

    // --- mpv ---

    fn on_mpv(&mut self, e: MpvEvent, cx: &mut Context<Self>) {
        match e {
            MpvEvent::Prop(name, v) => self.on_prop(&name, v),
            MpvEvent::FileLoaded => {
                self.expected_chapters = 0;
                self.loaded = true;
                self.net_error = false;
                self.fatal = false;
                self.retry_attempt = 0;
            }
            MpvEvent::EndFile(EndReason::Error) => self.on_error(cx),
            MpvEvent::EndFile(_) => {}
        }
        self.effects(cx);
        cx.notify();
    }

    fn on_prop(&mut self, name: &str, v: Value) {
        let num = v.as_f64();
        let flag = v.as_bool().unwrap_or(false);
        match name {
            "time-pos" => {
                if let (Some(t), true) = (num, self.loaded) {
                    self.pos = t;
                    self.record_progress(false);
                }
            }
            "duration" => self.dur = num.unwrap_or(0.),
            "pause" => {
                self.paused = flag;
                if flag {
                    self.record_progress(true);
                }
            }
            "volume" => self.volume = num.unwrap_or(100.).round(),
            "mute" => self.muted = flag,
            "paused-for-cache" => self.buffering = flag,
            "demuxer-cache-time" => self.cache_ahead = num.unwrap_or(0.),
            "chapter-list" => {
                self.chapters = serde_json::from_value(v).unwrap_or_default();
                self.expected_chapters = self.expected_chapters.max(self.chapters.len());
            }
            "track-list" => self.tracks = serde_json::from_value(v).unwrap_or_default(),
            "eof-reached"
                if self.loaded => {
                    self.eof = flag;
                    if flag {
                        self.idle = false;
                    }
                }
            _ => {}
        }
    }

    /// Falhou: tenta de novo na mesma posição, esperando cada vez mais, e desiste depois de 5.
    fn on_error(&mut self, cx: &mut Context<Self>) {
        let Some(wait) = retry_delay(self.retry_attempt) else {
            self.net_error = false;
            self.buffering = false;
            self.fatal = true;
            return;
        };
        self.retry_attempt += 1;
        self.net_error = true;
        self.retry_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            let _ = this.update(cx, |v, cx| {
                let at = v.pos;
                v.load_current(Some(at), cx);
            });
        }));
    }

    fn load_current(&mut self, start_at: Option<f64>, cx: &mut Context<Self>) {
        let Some(mpv) = &self.mpv else { return };
        self.playing_path = self.remote_path();
        let saved = self.data.progress.get(&self.playing_path).filter(|p| !p.watched).map(|p| p.position);
        let start = start_at.or(saved).unwrap_or(0.);
        self.load_seq += 1;
        self.loaded = false;
        self.expected_chapters = 0;
        self.buffering = true;
        self.eof = false;
        self.chapters.clear();
        self.tracks.clear();
        if start_at.is_none() {
            // Arquivo novo (não uma nova tentativa do mesmo).
            self.skip = SkipTimes::default();
            self.next_cancelled = false;
            self.fatal = false;
            self.skip_resolved_for.clear();
            self.retry_attempt = 0;
            self.pos = start;
            self.dur = 0.;
            self.countdown = None;
            self.countdown_task = None;
        }
        let url = self.core.player().stream_url(&self.playing_path, self.file().size);
        let mut args = vec!["loadfile".to_owned(), url, "replace".into(), "-1".into()];
        if start > 1. {
            args.push(format!("start={start:.1}"));
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        if mpv.command(&args).is_err() {
            self.on_error(cx);
        }
        cx.notify();
    }

    /// Reações ao estado: abertura/encerramento, faixas, próximo episódio.
    fn effects(&mut self, cx: &mut Context<Self>) {
        self.resolve_skip(cx);
        self.apply_tracks();
        self.update_countdown(cx);
        let go = can_auto_advance(&AdvanceState {
            loaded: self.loaded,
            eof: self.eof,
            auto_next: self.settings().auto_next,
            cancelled: self.next_cancelled,
            has_next: self.next().is_some(),
            advanced_from: self.advanced_from.as_deref(),
            current: &self.target.file.name,
        });
        if go {
            self.go_next(cx);
        }
    }

    /// Capítulos do arquivo, senão AniSkip (só animes). Uma vez por arquivo.
    fn resolve_skip(&mut self, cx: &mut Context<Self>) {
        let name = self.file().name.clone();
        if !self.loaded || self.dur <= 0. || self.skip_resolved_for == name || self.chapters.len() < self.expected_chapters {
            return;
        }
        self.skip_resolved_for = name.clone();
        let from_chapters = classify_chapters(&self.chapters, self.dur);
        if from_chapters.op.is_some() || from_chapters.ed.is_some() {
            self.skip = from_chapters;
            return;
        }
        let (Some(mal), true, Some(ep)) = (self.target.mal_id, self.target.is_anime, episode_number(&name)) else { return };
        let core = self.core.clone();
        let dur = self.dur;
        let task = self.core.spawn(async move { core.player().aniskip(mal, ep, dur).await });
        cx.spawn(async move |this, cx| {
            if let Ok(times) = task.await {
                let _ = this.update(cx, |v, cx| {
                    if v.file().name == name {
                        v.skip = times;
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    /// Faixas lembradas do release ou idiomas preferidos, a cada carga.
    fn apply_tracks(&mut self) {
        if !self.loaded || !self.tracks.iter().any(|t| t.kind != TrackKind::Video) || self.tracks_applied_for == self.load_seq {
            return;
        }
        self.tracks_applied_for = self.load_seq;
        let s = &self.data.settings;
        let (sid, aid) = pick_tracks(&self.tracks, self.data.tracks.get(&self.target.dir), &s.sub_lang, &s.audio_lang);
        if let Some(sid) = sid {
            self.set_track("sid", sid);
        }
        if let Some(aid) = aid {
            self.set_track("aid", SubPick::Id(aid));
        }
    }

    fn set_track(&self, prop: &str, pick: SubPick) {
        let value = match pick {
            SubPick::Id(id) => id.to_string(),
            SubPick::Off => "no".into(),
        };
        if let Some(m) = &self.mpv {
            let _ = m.set(prop, &value);
        }
    }

    // --- Ações ---

    fn toggle_pause(&mut self) {
        if let Some(m) = &self.mpv {
            let _ = m.set_flag("pause", !self.paused);
        }
    }

    fn seek(&mut self, t: f64) {
        let t = t.min(self.dur - 0.5).max(0.);
        if let Some(m) = &self.mpv {
            let _ = m.command(&["seek", &format!("{t:.3}"), "absolute+exact"]);
        }
        self.pos = t;
    }

    fn set_volume(&mut self, v: f64) {
        let v = v.clamp(0., 100.);
        if let Some(m) = &self.mpv {
            let _ = m.set_double("volume", v);
            if self.muted && v > 0. {
                let _ = m.set_flag("mute", false);
            }
        }
        self.volume = v;
    }

    fn toggle_mute(&mut self) {
        if let Some(m) = &self.mpv {
            let _ = m.set_flag("mute", !self.muted);
        }
    }

    fn go_next(&mut self, cx: &mut Context<Self>) {
        let Some(n) = self.next() else { return };
        let from = self.file().name.clone();
        if self.advanced_from.as_deref() == Some(from.as_str()) {
            return;
        }
        self.advanced_from = Some(from);
        self.switch_to(n, cx);
    }

    fn switch_to(&mut self, file: FileEntry, cx: &mut Context<Self>) {
        self.record_progress(true);
        self.menu = None;
        self.target.file = file;
        self.load_current(None, cx);
    }

    fn show_skip(&self) -> bool {
        self.settings().show_skip_intro && self.skip.op.is_some_and(|op| self.pos >= op.start && self.pos < op.end - 1.)
    }

    fn skip_intro(&mut self) {
        if let (Some(op), true) = (self.skip.op, self.show_skip()) {
            self.seek(op.end);
        }
    }

    /// Início dos créditos: capítulos/AniSkip, senão estimativa (anime de TV), senão fim do arquivo.
    fn show_next(&self) -> bool {
        let ending = self.skip.ed.or_else(|| estimate_ending(self.dur, self.target.is_anime));
        self.next().is_some()
            && self.dur > 0.
            && !self.next_cancelled
            && self.pos >= next_card_start(self.dur, ending, self.settings().countdown_sec)
    }

    /// Liga/desliga a contagem do cartão de próximo episódio.
    fn update_countdown(&mut self, cx: &mut Context<Self>) {
        let want = self.show_next() && self.settings().auto_next;
        if !want {
            self.countdown = None;
            self.countdown_task = None;
            return;
        }
        if self.countdown.is_some() {
            return;
        }
        self.countdown = Some(self.settings().countdown_sec as f64);
        self.countdown_task = Some(cx.spawn(async move |this, cx| {
            let mut last = Instant::now();
            loop {
                cx.background_executor().timer(Duration::from_millis(100)).await;
                let done = this.update(cx, |v, cx| {
                    let now = Instant::now();
                    let step = if v.paused { 0. } else { (now - last).as_secs_f64() };
                    last = now;
                    let left = (v.countdown.unwrap_or(0.) - step).max(0.);
                    v.countdown = Some(left);
                    cx.notify();
                    if left <= 0. {
                        v.go_next(cx);
                        true
                    } else {
                        false
                    }
                });
                if done.unwrap_or(true) {
                    break;
                }
            }
        }));
    }

    fn choose_sub(&mut self, t: Option<Track>) {
        self.set_track("sid", t.as_ref().map_or(SubPick::Off, |t| SubPick::Id(t.id)));
        let prev = self.data.tracks.get(&self.target.dir).cloned().unwrap_or_default();
        let pref = TrackPref { sub_off: t.is_none(), sub_lang: t.as_ref().and_then(|t| t.lang.clone()), sub_title: t.and_then(|t| t.title), ..prev };
        self.save_tracks(pref);
    }

    fn choose_audio(&mut self, t: Track) {
        self.set_track("aid", SubPick::Id(t.id));
        let prev = self.data.tracks.get(&self.target.dir).cloned().unwrap_or_default();
        let pref = TrackPref { audio_lang: t.lang, audio_title: t.title, ..prev };
        self.save_tracks(pref);
    }

    fn save_tracks(&mut self, pref: TrackPref) {
        self.data.tracks.insert(self.target.dir.clone(), pref.clone());
        self.core.player().save_tracks(self.target.dir.clone(), pref);
    }

    fn change_settings(&mut self, f: impl FnOnce(&mut Settings)) {
        let mut s = self.data.settings.clone();
        f(&mut s);
        if s.cache_mib != self.data.settings.cache_mib
            && let Some(m) = &self.mpv
        {
            let _ = m.set("demuxer-max-bytes", &format!("{}MiB", s.cache_mib));
        }
        self.data.settings = s.clone();
        self.core.player().save_settings(s);
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.is_fullscreen() {
            window.toggle_fullscreen();
        }
        cx.emit(PlayerEvent::Close);
    }

    /// Mostra os controles e esconde de novo com o mouse parado.
    fn poke(&mut self, cx: &mut Context<Self>) {
        if self.idle {
            self.idle = false;
            cx.notify();
        }
        self.hide_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(HIDE_AFTER).await;
            let _ = this.update(cx, |v, cx| {
                v.idle = true;
                cx.notify();
            });
        }));
    }

    fn controls_hidden(&self) -> bool {
        self.idle && !self.paused && self.menu.is_none() && !self.buffering && self.drag.is_none()
    }

    fn toggle_menu(&mut self, m: Menu) {
        self.menu = if self.menu == Some(m) { None } else { Some(m) };
    }

    fn on_key(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let m = &e.keystroke.modifiers;
        if m.control || m.alt || m.platform {
            return;
        }
        match e.keystroke.key.as_str() {
            "space" => self.toggle_pause(),
            "left" => self.seek(self.pos - 5.),
            "right" => self.seek(self.pos + 5.),
            "up" => self.set_volume(self.volume + 5.),
            "down" => self.set_volume(self.volume - 5.),
            "m" => self.toggle_mute(),
            "f" => window.toggle_fullscreen(),
            "n" => self.go_next(cx),
            "s" => self.skip_intro(),
            "escape" => {
                if self.menu.is_some() {
                    self.menu = None;
                } else if window.is_fullscreen() {
                    window.toggle_fullscreen();
                } else {
                    self.close(window, cx);
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        self.poke(cx);
        cx.notify();
    }

    // --- Arrastar barras ---

    fn fraction(bounds: Bounds<Pixels>, x: Pixels) -> f64 {
        let w = f32::from(bounds.size.width).max(1.);
        ((f32::from(x - bounds.origin.x)) / w).clamp(0., 1.) as f64
    }

    fn drag_to(&mut self, x: Pixels) {
        match self.drag {
            Some(Drag::Seek(_)) => self.drag = Some(Drag::Seek(Self::fraction(self.seek_bounds.get(), x) * self.dur)),
            Some(Drag::Volume) => self.set_volume(Self::fraction(self.volume_bounds.get(), x) * 100.),
            None => {}
        }
    }

    fn end_drag(&mut self) {
        if let Some(Drag::Seek(t)) = self.drag.take() {
            self.seek(t);
        }
    }
}

impl Drop for PlayerView {
    fn drop(&mut self) {
        self.record_progress(true);
        // O mpv recebe `quit` ao ser descartado; a janela filha some já.
        self.mpv = None;
        self.surface.hide();
    }
}

// --- Desenho ---

fn gradient_down(alpha: f32) -> gpui::Background {
    linear_gradient(180., linear_color_stop(theme::black_a(alpha), 0.), linear_color_stop(theme::black_a(0.), 1.))
}

fn gradient_up(alpha: f32) -> gpui::Background {
    linear_gradient(0., linear_color_stop(theme::black_a(alpha), 0.), linear_color_stop(theme::black_a(0.), 1.))
}

fn panel(id: &'static str, width: f32) -> Stateful<Div> {
    div()
        .id(id)
        .absolute()
        .right_4()
        .bottom(px(112.))
        .w(px(width))
        .max_h(px(520.))
        .overflow_y_scroll()
        .p_2()
        .rounded_2xl()
        .border_1()
        .border_color(theme::line_strong())
        .bg(gpui::Rgba { a: 0.94, ..theme::surface() })
        .shadow_lg()
        .occlude()
}

fn heading(text: &'static str) -> Div {
    div().px_3().pt_2().pb_1().text_xs().font_weight(FontWeight::MEDIUM).text_color(theme::faint()).child(text)
}

fn option(id: impl Into<gpui::ElementId>, selected: bool, label: impl Into<SharedString>, hint: Option<String>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_3()
        .w_full()
        .px_3()
        .py_2()
        .rounded_lg()
        .text_sm()
        .cursor_pointer()
        .hover(|s| s.bg(theme::white_a(0.08)))
        .when(selected, |d| d.text_color(theme::accent_ink()))
        .child(icon("check").size_4().text_color(theme::accent_ink()).when(!selected, |i| i.invisible()))
        .child(truncate(label).flex_1())
        .children(hint.map(|h| div().text_xs().text_color(theme::faint()).child(h.to_uppercase())))
}

fn toggle_row(id: &'static str, label: &'static str, on: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_3()
        .py_2p5()
        .rounded_lg()
        .text_sm()
        .cursor_pointer()
        .hover(|s| s.bg(theme::white_a(0.06)))
        .child(label)
        .child(
            div()
                .relative()
                .flex_none()
                .w(px(36.))
                .h(px(20.))
                .rounded_full()
                .bg(if on { theme::accent() } else { theme::white_a(0.15) })
                .child(div().absolute().top(px(2.)).left(px(if on { 18. } else { 2. })).size(px(16.)).rounded_full().bg(theme::white())),
        )
}

fn choice_chip(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>, selected: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex_1()
        .flex()
        .justify_center()
        .px_2()
        .py_1p5()
        .rounded_lg()
        .border_1()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .map(|d| {
            if selected {
                d.border_color(theme::accent()).bg(theme::accent_soft()).text_color(theme::accent_ink())
            } else {
                d.border_color(theme::line_strong()).text_color(theme::muted()).hover(|s| s.text_color(theme::fg()))
            }
        })
        .child(label.into())
}

const LANGS: [(&str, &str); 5] = [("por", "Português"), ("eng", "Inglês"), ("jpn", "Japonês"), ("kor", "Coreano"), ("spa", "Espanhol")];

impl PlayerView {
    fn render_tracks(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let subs: Vec<Track> = self.tracks.iter().filter(|t| t.kind == TrackKind::Sub).cloned().collect();
        let audios: Vec<Track> = self.tracks.iter().filter(|t| t.kind == TrackKind::Audio).cloned().collect();
        let none_selected = !subs.iter().any(|t| t.selected);
        panel("tracks-menu", 288.)
            .child(heading("Legendas"))
            .child(option("sub-off", none_selected, "Desligada", None).on_click(cx.listener(|v, _, _, cx| {
                v.choose_sub(None);
                cx.notify();
            })))
            .children(subs.into_iter().map(|t| {
                let hint = t.title.as_ref().and(t.lang.clone());
                option(SharedString::from(format!("sub-{}", t.id)), t.selected, t.name(), hint).on_click(cx.listener(
                    move |v, _, _, cx| {
                        v.choose_sub(Some(t.clone()));
                        cx.notify();
                    },
                ))
            }))
            .when(audios.len() > 1, |p| {
                p.child(
                    div()
                        .mt_1()
                        .pt_1()
                        .border_t_1()
                        .border_color(theme::line())
                        .child(heading("Áudio"))
                        .children(audios.into_iter().map(|t| {
                            let hint = t.title.as_ref().and(t.lang.clone());
                            option(SharedString::from(format!("audio-{}", t.id)), t.selected, t.name(), hint).on_click(
                                cx.listener(move |v, _, _, cx| {
                                    v.choose_audio(t.clone());
                                    cx.notify();
                                }),
                            )
                        })),
                )
            })
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let s = self.settings().clone();
        let lang_row = |id: &'static str, label: &'static str, value: String, sub: bool| {
            div()
                .px_3()
                .py_2p5()
                .text_sm()
                .child(div().mb_2().child(label))
                .child(div().flex().flex_wrap().gap_1p5().children(LANGS.iter().map(|(code, name)| {
                    let code = *code;
                    choice_chip(SharedString::from(format!("{id}-{code}")), *name, value == code).on_click(cx.listener(
                        move |v, _, _, cx| {
                            v.change_settings(|s| {
                                if sub {
                                    s.sub_lang = code.into();
                                } else {
                                    s.audio_lang = code.into();
                                }
                            });
                            cx.notify();
                        },
                    ))
                })))
        };
        panel("settings-menu", 320.)
            .child(toggle_row("auto-next", "Ir para o próximo episódio sozinho", s.auto_next).on_click(cx.listener(
                |v, _, _, cx| {
                    v.change_settings(|s| s.auto_next = !s.auto_next);
                    cx.notify();
                },
            )))
            .when(s.auto_next, |p| {
                p.child(
                    div().px_3().py_2p5().text_sm().child(div().mb_2().child("Tempo da contagem")).child(
                        div().flex().gap_1p5().children([5u8, 10, 15].map(|n| {
                            choice_chip(SharedString::from(format!("cd-{n}")), format!("{n} s"), s.countdown_sec == n).on_click(
                                cx.listener(move |v, _, _, cx| {
                                    v.change_settings(|s| s.countdown_sec = n);
                                    cx.notify();
                                }),
                            )
                        })),
                    ),
                )
            })
            .child(toggle_row("skip-intro", "Mostrar \"Pular abertura\"", s.show_skip_intro).on_click(cx.listener(
                |v, _, _, cx| {
                    v.change_settings(|s| s.show_skip_intro = !s.show_skip_intro);
                    cx.notify();
                },
            )))
            .child(div().my_1().border_t_1().border_color(theme::line()))
            .child(lang_row("sub-lang", "Legenda preferida", s.sub_lang.clone(), true))
            .child(lang_row("audio-lang", "Áudio preferido", s.audio_lang.clone(), false))
            .child(div().my_1().border_t_1().border_color(theme::line()))
            .child(
                div().px_3().py_2p5().text_sm().child(div().mb_2().child("Memória à frente")).child(
                    div().flex().gap_1p5().children([100u32, 150, 300, 500].map(|n| {
                        choice_chip(SharedString::from(format!("mem-{n}")), format!("{n} MB"), s.cache_mib == n).on_click(
                            cx.listener(move |v, _, _, cx| {
                                v.change_settings(|s| s.cache_mib = n);
                                cx.notify();
                            }),
                        )
                    })),
                ),
            )
    }

    fn render_episodes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.file().name.clone();
        panel("episodes-menu", 384.).child(heading("Episódios")).children(video_files(&self.target.files).into_iter().map(|f| {
            let n = episode_number(&f.name);
            let p = self.data.progress.get(&join_path(&self.target.dir, &f.name)).cloned();
            let is_current = f.name == current;
            let label = n.map_or_else(|| parse_episode(&f.name).label, |n| format!("Episódio {n}"));
            let id = SharedString::from(format!("ep-{}", f.name));
            div()
                .id(id)
                .flex()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .rounded_lg()
                .cursor_pointer()
                .hover(|s| s.bg(theme::white_a(0.08)))
                .when(is_current, |d| d.bg(theme::accent_soft()))
                .child(
                    div()
                        .w_8()
                        .flex_none()
                        .flex()
                        .justify_center()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if is_current { theme::accent_ink() } else { theme::muted() })
                        .child(n.map_or("·".to_string(), |n| n.to_string())),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(truncate(label).text_sm())
                        .children(p.as_ref().map(|p| {
                            let frac = if p.watched { 1. } else { (p.position / p.duration.max(1.)).min(1.) };
                            div()
                                .mt_1()
                                .h(px(2.))
                                .rounded_full()
                                .bg(theme::white_a(0.12))
                                .child(div().h_full().w(relative(frac as f32)).bg(theme::accent()))
                        })),
                )
                .when(p.as_ref().is_some_and(|p| p.watched), |d| d.child(icon("check").size_4().text_color(theme::ok())))
                .on_click(cx.listener(move |v, _, _, cx| v.switch_to(f.clone(), cx)))
        }))
    }

    fn render_seekbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dur = self.dur.max(0.001);
        let shown = match self.drag {
            Some(Drag::Seek(t)) => t,
            _ => self.pos,
        };
        let frac = |t: f64| relative((t / dur).clamp(0., 1.) as f32);
        let bounds = self.seek_bounds.clone();
        let hover = self.hover;
        div()
            .id("seekbar")
            .relative()
            .flex()
            .items_center()
            .h_5()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|v, e: &MouseDownEvent, _, cx| {
                    if v.dur > 0. {
                        v.drag = Some(Drag::Seek(Self::fraction(v.seek_bounds.get(), e.position.x) * v.dur));
                        cx.stop_propagation();
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(|v, e: &MouseMoveEvent, _, cx| {
                let b = v.seek_bounds.get();
                v.hover = Some((e.position.x - b.origin.x, Self::fraction(b, e.position.x) * v.dur));
                cx.notify();
            }))
            .on_hover(cx.listener(|v, h: &bool, _, cx| {
                if !h {
                    v.hover = None;
                    cx.notify();
                }
            }))
            .child(canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {}).absolute().size_full())
            .child(
                div()
                    .relative()
                    .w_full()
                    .h_1()
                    .rounded_full()
                    .bg(theme::white_a(0.18))
                    .child(div().absolute().top_0().bottom_0().left_0().rounded_full().bg(theme::white_a(0.32)).w(frac(self.pos + self.cache_ahead)))
                    .child(div().absolute().top_0().bottom_0().left_0().rounded_full().bg(theme::accent()).w(frac(shown)))
                    .children(
                        self.chapters
                            .iter()
                            .filter(|c| c.time > 0.)
                            .map(|c| div().absolute().top(px(-1.)).bottom(px(-1.)).w(px(2.)).bg(theme::black_a(0.8)).left(frac(c.time))),
                    ),
            )
            .when(self.dur > 0., |d| {
                d.child(div().absolute().size(px(14.)).ml(px(-7.)).rounded_full().bg(theme::white()).shadow_md().left(frac(shown)))
            })
            .children(hover.map(|(x, t)| {
                div()
                    .absolute()
                    .bottom(px(24.))
                    .left(x - px(24.))
                    .px_2()
                    .py_0p5()
                    .rounded_md()
                    .bg(theme::black_a(0.85))
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .child(format_time(t))
            }))
    }

    fn render_controls(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vol_icon = if self.muted || self.volume == 0. {
            "volume-x"
        } else if self.volume < 50. {
            "volume-1"
        } else {
            "volume-2"
        };
        let vol = if self.muted { 0. } else { self.volume };
        let vb = self.volume_bounds.clone();
        let menu = self.menu;
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(icon_button("play", if self.paused { "play" } else { "pause" }).on_click(cx.listener(|v, _, _, cx| {
                v.toggle_pause();
                cx.notify();
            })))
            .child(
                icon_button("next", "skip-forward")
                    .when(self.next().is_none(), |b| b.opacity(0.35))
                    .on_click(cx.listener(|v, _, _, cx| v.go_next(cx))),
            )
            .child(icon_button("mute", vol_icon).on_click(cx.listener(|v, _, _, cx| {
                v.toggle_mute();
                cx.notify();
            })))
            .child(
                div()
                    .id("volume")
                    .relative()
                    .w(px(96.))
                    .h_5()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|v, e: &MouseDownEvent, _, cx| {
                            v.drag = Some(Drag::Volume);
                            v.drag_to(e.position.x);
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .child(canvas(move |b, _, _| vb.set(b), |_, _, _, _| {}).absolute().size_full())
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h_1()
                            .rounded_full()
                            .bg(theme::white_a(0.18))
                            .child(div().absolute().top_0().bottom_0().left_0().rounded_full().bg(theme::white()).w(relative((vol / 100.) as f32))),
                    ),
            )
            .child(
                div()
                    .ml_2()
                    .flex_none()
                    .text_sm()
                    .text_color(theme::muted())
                    .child(format!("{} / {}", format_time(self.pos), format_time(self.dur))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .px_4()
                    .flex()
                    .justify_center()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(truncate(episode_label(&self.file().name))),
            )
            .child(icon_toggle("tracks", "captions", menu == Some(Menu::Tracks)).on_click(cx.listener(|v, _, _, cx| {
                v.toggle_menu(Menu::Tracks);
                cx.notify();
            })))
            .child(icon_toggle("settings", "settings-2", menu == Some(Menu::Settings)).on_click(cx.listener(|v, _, _, cx| {
                v.toggle_menu(Menu::Settings);
                cx.notify();
            })))
            .child(icon_toggle("episodes", "list-video", menu == Some(Menu::Episodes)).on_click(cx.listener(|v, _, _, cx| {
                v.toggle_menu(Menu::Episodes);
                cx.notify();
            })))
            .child(
                icon_button("fullscreen", if window.is_fullscreen() { "minimize" } else { "maximize" })
                    .on_click(|_, window, _| window.toggle_fullscreen()),
            )
    }

    fn render_next_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let label = self.next().map_or_else(String::new, |n| {
            episode_number(&n.name).map_or_else(|| n.name.clone(), |num| format!("Episódio {num}"))
        });
        let total = self.settings().countdown_sec as f64;
        let fill = self.countdown.map_or(0., |left| 1. - left / total.max(1.));
        div()
            .id("next-card")
            .occlude()
            .w(px(320.))
            .p_4()
            .rounded_2xl()
            .border_1()
            .border_color(theme::line_strong())
            .bg(gpui::Rgba { a: 0.92, ..theme::surface() })
            .shadow_lg()
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(div().text_xs().text_color(theme::muted()).child(match self.countdown {
                Some(left) => format!("Próximo episódio em {}", left.ceil() as u32),
                None => "Próximo episódio".into(),
            }))
            .child(truncate(label).mt_0p5().mb_3().font_weight(FontWeight::SEMIBOLD))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("watch-next")
                            .relative()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap_1p5()
                            .overflow_hidden()
                            .px_3()
                            .py_2()
                            .rounded_lg()
                            .bg(theme::accent())
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .child(div().absolute().top_0().bottom_0().left_0().bg(theme::white_a(0.3)).w(relative(fill as f32)))
                            .child(icon("play").size_4())
                            .child("Assistir agora")
                            .on_click(cx.listener(|v, _, _, cx| v.go_next(cx))),
                    )
                    .child(button("cancel-next", "Cancelar", false).on_click(cx.listener(|v, _, _, cx| {
                        v.next_cancelled = true;
                        v.countdown = None;
                        v.countdown_task = None;
                        cx.notify();
                    }))),
            )
    }
}

impl Render for PlayerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let hidden = self.controls_hidden();
        let surface = self.surface.clone();
        let title = self.target.anime_title.clone();
        let ep = episode_label(&self.file().name);
        let show_next = self.show_next();
        let show_skip = self.show_skip() && !show_next;

        div()
            .id("player")
            .track_focus(&self.focus)
            .key_context("Player")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .text_color(theme::fg())
            .when(hidden, |d| d.cursor(CursorStyle::None))
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_move(cx.listener(|v, e: &MouseMoveEvent, _, cx| {
                if v.drag.is_some() {
                    v.drag_to(e.position.x);
                    cx.notify();
                }
                if v.last_mouse != Some(e.position) {
                    v.last_mouse = Some(e.position);
                    v.poke(cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|v, _: &MouseUpEvent, _, cx| {
                    v.end_drag();
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|v, _: &MouseUpEvent, _, cx| {
                    v.end_drag();
                    cx.notify();
                }),
            )
            // Janela do mpv acompanha a área do player (pixels físicos).
            .child(
                canvas(
                    move |b, window, _| {
                        let s = window.scale_factor();
                        surface.place(
                            (f32::from(b.origin.x) * s).round() as i32,
                            (f32::from(b.origin.y) * s).round() as i32,
                            (f32::from(b.size.width) * s).round() as i32,
                            (f32::from(b.size.height) * s).round() as i32,
                        );
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            // Barra de título sobreposta (some com o mouse parado)
            .when(!hidden, |d| d.child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .h_12()
                    .pl_2()
                    .pr_4()
                    .bg(gradient_down(0.75))
                    .child(icon_button("back", "arrow-left").on_click(cx.listener(|v, _, window, cx| v.close(window, cx))))
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .flex_1()
                            .text_sm()
                            .child(truncate(title).flex_none().max_w(relative(0.7)).font_weight(FontWeight::SEMIBOLD))
                            .child(div().flex_none().text_color(theme::muted()).child(format!(" · {ep}"))),
                    ),
            ))
            // Área do vídeo: clique pausa, duplo clique alterna tela cheia
            .child(
                div()
                    .id("video-area")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .on_click(cx.listener(|v, e: &ClickEvent, window, cx| {
                        if v.menu.is_some() {
                            v.menu = None;
                        } else {
                            v.toggle_pause();
                            if e.click_count() == 2 {
                                window.toggle_fullscreen();
                            }
                        }
                        cx.notify();
                    }))
                    .when(self.buffering && !self.net_error && !self.fatal && self.broken.is_none(), |d| {
                        d.child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(div().size_10().text_color(theme::white_a(0.8)).child(spinner("buffering"))),
                        )
                    })
                    .when_some(self.broken.clone(), |d, msg| {
                        d.child(
                            div().absolute().inset_0().flex().items_center().justify_center().child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_2()
                                    .px_6()
                                    .py_5()
                                    .rounded_2xl()
                                    .bg(theme::black_a(0.8))
                                    .text_sm()
                                    .child(icon("circle-alert").text_color(theme::danger()))
                                    .child("O motor de vídeo (mpv) não abriu.")
                                    .child(div().text_xs().text_color(theme::muted()).child(msg)),
                            ),
                        )
                    })
                    .when(self.fatal, |d| {
                        d.child(
                            div().absolute().inset_0().flex().items_center().justify_center().child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_3()
                                    .px_6()
                                    .py_5()
                                    .rounded_2xl()
                                    .border_1()
                                    .border_color(theme::line_strong())
                                    .bg(theme::black_a(0.8))
                                    .text_sm()
                                    .child(icon("circle-alert").size_6().text_color(theme::danger()))
                                    .child("Não foi possível tocar este arquivo.")
                                    .child(
                                        div()
                                            .max_w(px(288.))
                                            .text_xs()
                                            .text_color(theme::muted())
                                            .child("Pode ser a conexão, a sessão expirada ou um arquivo indisponível na nuvem."),
                                    )
                                    .child(button("retry", "Tentar de novo", true).mt_1().on_click(cx.listener(|v, _, _, cx| {
                                        cx.stop_propagation();
                                        v.retry_attempt = 0;
                                        v.fatal = false;
                                        let at = v.pos;
                                        v.load_current(Some(at), cx);
                                    }))),
                            ),
                        )
                    })
                    .when(self.net_error, |d| {
                        d.child(
                            div().absolute().inset_0().flex().items_center().justify_center().child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .px_5()
                                    .py_3()
                                    .rounded_xl()
                                    .border_1()
                                    .border_color(theme::line_strong())
                                    .bg(theme::black_a(0.75))
                                    .text_sm()
                                    .child(icon("wifi-off").text_color(theme::danger()))
                                    .child("Sem conexão com a nuvem. Tentando de novo…"),
                            ),
                        )
                    }),
            )
            // Pular abertura / próximo episódio: acima dos controles, mesmo com eles escondidos
            .child(
                div()
                    .absolute()
                    .right_5()
                    .bottom(px(if hidden { 24. } else { 112. }))
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap_3()
                    .when(show_skip, |d| {
                        d.child(
                            div()
                                .id("skip-intro")
                                .occlude()
                                .flex()
                                .items_center()
                                .gap_2()
                                .px_4()
                                .py_2p5()
                                .rounded_xl()
                                .border_1()
                                .border_color(theme::line_strong())
                                .bg(theme::black_a(0.7))
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .cursor_pointer()
                                .hover(|s| s.border_color(theme::accent()).bg(theme::black_a(0.85)))
                                .child("Pular abertura")
                                .child(icon("chevrons-right").text_color(theme::accent_ink()))
                                .on_click(cx.listener(|v, _, _, cx| {
                                    cx.stop_propagation();
                                    v.skip_intro();
                                    cx.notify();
                                })),
                        )
                    })
                    .when(show_next, |d| d.child(self.render_next_card(cx))),
            )
            .when(self.menu == Some(Menu::Tracks), |d| d.child(self.render_tracks(cx)))
            .when(self.menu == Some(Menu::Settings), |d| d.child(self.render_settings(cx)))
            .when(self.menu == Some(Menu::Episodes), |d| d.child(self.render_episodes(cx)))
            // Controles inferiores
            .when(!hidden, |d| d.child(
                div()
                    .id("controls")
                    .flex_none()
                    .px_5()
                    .pt_10()
                    .pb_3()
                    .bg(gradient_up(0.85))
                    .child(self.render_seekbar(cx))
                    .child(self.render_controls(window, cx)),
            ))
    }
}
