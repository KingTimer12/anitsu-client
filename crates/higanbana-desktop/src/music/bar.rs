//! Barra do player de músicas, embaixo da janela enquanto um álbum toca.

use std::{cell::Cell, rc::Rc};

use gpui::{
    Bounds, Context, Entity, FontWeight, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit,
    ParentElement, Pixels, Render, SharedString, Styled, Window, canvas, div, img, prelude::*, px, relative,
};
use higanbana_core::{
    library::{base_name, parent_path},
    music::{album::format_duration, queue::Repeat},
};

use crate::{
    state::{AppState, Img, state},
    theme,
    ui::{icon, icon_toggle, truncate},
};

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Seek(f64),
    Volume,
}

pub struct MusicBar {
    state: Entity<AppState>,
    seek_bounds: Rc<Cell<Bounds<Pixels>>>,
    volume_bounds: Rc<Cell<Bounds<Pixels>>>,
    drag: Option<Drag>,
    /// Volume antes de silenciar.
    unmuted: Option<f32>,
}

fn fraction(b: Bounds<Pixels>, x: Pixels) -> f64 {
    ((f32::from(x - b.origin.x)) / f32::from(b.size.width).max(1.)).clamp(0., 1.) as f64
}

impl MusicBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self { state, seek_bounds: Rc::default(), volume_bounds: Rc::default(), drag: None, unmuted: None }
    }

    fn set_volume(&self, v: f32, cx: &mut Context<Self>) {
        self.state.read(cx).core.music().set(Some(v.clamp(0., 1.)), None, None, None);
    }

    fn drag_to(&mut self, x: Pixels, cx: &mut Context<Self>) {
        match self.drag {
            Some(Drag::Seek(_)) => {
                let dur = self.state.read(cx).music.as_ref().and_then(|m| m.duration).unwrap_or(0.);
                self.drag = Some(Drag::Seek(fraction(self.seek_bounds.get(), x) * dur));
            }
            Some(Drag::Volume) => self.set_volume(fraction(self.volume_bounds.get(), x) as f32, cx),
            None => {}
        }
    }

    fn end_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(Drag::Seek(t)) = self.drag.take() {
            self.state.read(cx).core.music().seek(t);
        }
    }
}

impl Render for MusicBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let Some(m) = st.music.clone() else { return div().into_any_element() };
        let Some(album_path) = m.album.clone() else { return div().into_any_element() };
        let core = st.core.clone();
        let meta = m.path.as_ref().and_then(|p| st.music_session.metas.get(p)).cloned();
        let cover = st.cached_image(&format!("album-cover:{album_path}"));
        let file_title = m.path.as_deref().map(|p| higanbana_core::music::album::parse_track(base_name(p)).title).unwrap_or_default();
        let title = m.title.clone().or(meta.as_ref().and_then(|x| x.title.clone())).filter(|t| !t.is_empty()).unwrap_or(file_title);
        let album_name = higanbana_core::music::album::album_title(base_name(&album_path));
        let work = base_name(&parent_path(&album_path)).to_owned();
        let subtitle = m.artist.clone().unwrap_or_else(|| if work.is_empty() { album_name.clone() } else { format!("{work} · {album_name}") });
        let dur = m.duration.unwrap_or(0.);
        let shown = match self.drag {
            Some(Drag::Seek(t)) => t,
            _ => m.position,
        };
        let frac = |t: f64| relative(if dur > 0. { (t / dur).clamp(0., 1.) as f32 } else { 0. });
        let (sb, vb) = (self.seek_bounds.clone(), self.volume_bounds.clone());
        let muted = m.volume <= 0.001;
        let vol_icon = if muted { "volume-x" } else if m.volume < 0.5 { "volume-1" } else { "volume-2" };
        let repeat_icon = if m.repeat == Repeat::One { "repeat-1" } else { "repeat" };
        let cmd = |c: &'static str| {
            let core = core.clone();
            move |_: &gpui::ClickEvent, _: &mut Window, _: &mut gpui::App| {
                let _ = core.music().command(c);
            }
        };

        div()
            .id("music-bar")
            .flex_none()
            .h(px(72.))
            .flex()
            .items_center()
            .gap_4()
            .px_4()
            .border_t_1()
            .border_color(theme::line())
            .bg(theme::bg())
            .on_mouse_move(cx.listener(|v, e: &MouseMoveEvent, _, cx| {
                if v.drag.is_some() {
                    v.drag_to(e.position.x, cx);
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|v, _: &MouseUpEvent, _, cx| {
                v.end_drag(cx);
                cx.notify();
            }))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|v, _: &MouseUpEvent, _, cx| {
                v.end_drag(cx);
                cx.notify();
            }))
            // Faixa: capa, título e obra (abre o álbum)
            .child(
                div()
                    .id("music-open-album")
                    .w(px(280.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_3()
                    .cursor_pointer()
                    .child(div().size_12().flex_none().rounded_md().overflow_hidden().bg(theme::elevated()).children(match cover {
                        Some(Img::Ready(i)) => Some(img(i).size_full().object_fit(ObjectFit::Cover)),
                        _ => None,
                    }))
                    .child(
                        div()
                            .min_w_0()
                            .child(truncate(title).text_sm().font_weight(FontWeight::SEMIBOLD))
                            .child(truncate(subtitle).text_xs().text_color(theme::muted())),
                    )
                    .on_click({
                        let st = self.state.clone();
                        let path = album_path.clone();
                        move |_, _, cx| st.update(cx, |s, cx| s.go(path.clone(), cx))
                    }),
            )
            // Controles e posição
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(icon_toggle("music-shuffle", "shuffle", m.shuffle).size_8().on_click({
                                let core = core.clone();
                                let s = m.shuffle;
                                move |_, _, _| core.music().set(None, Some(!s), None, None)
                            }))
                            .child(crate::ui::icon_button("music-prev", "skip-back").size_8().on_click(cmd("prev")))
                            .child(
                                div()
                                    .id("music-toggle")
                                    .size_9()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_full()
                                    .bg(theme::fg())
                                    .cursor_pointer()
                                    .child(icon(if m.playing { "pause" } else { "play" }).size_4().text_color(theme::bg()))
                                    .on_click(cmd("toggle")),
                            )
                            .child(crate::ui::icon_button("music-next", "skip-forward").size_8().on_click(cmd("next")))
                            .child(icon_toggle("music-repeat", repeat_icon, m.repeat != Repeat::Off).size_8().on_click({
                                let core = core.clone();
                                let next = match m.repeat {
                                    Repeat::Off => Repeat::All,
                                    Repeat::All => Repeat::One,
                                    Repeat::One => Repeat::Off,
                                };
                                move |_, _, _| core.music().set(None, None, Some(next), None)
                            })),
                    )
                    .child(
                        div()
                            .w_full()
                            .max_w(px(560.))
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .text_color(theme::muted())
                            .child(div().w_10().flex().justify_end().child(format_duration(shown)))
                            .child(
                                div()
                                    .id("music-seek")
                                    .relative()
                                    .flex_1()
                                    .h_4()
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |v, e: &MouseDownEvent, _, cx| {
                                        if dur > 0. {
                                            v.drag = Some(Drag::Seek(fraction(v.seek_bounds.get(), e.position.x) * dur));
                                            cx.notify();
                                        }
                                    }))
                                    .child(canvas(move |b, _, _| sb.set(b), |_, _, _, _| {}).absolute().size_full())
                                    .child(
                                        div()
                                            .relative()
                                            .w_full()
                                            .h_1()
                                            .rounded_full()
                                            .bg(theme::white_a(0.15))
                                            .child(div().absolute().top_0().bottom_0().left_0().rounded_full().bg(theme::white_a(0.3)).w(frac(m.buffered)))
                                            .child(div().absolute().top_0().bottom_0().left_0().rounded_full().bg(theme::brand()).w(frac(shown))),
                                    ),
                            )
                            .child(div().w_10().child(if dur > 0. { format_duration(dur) } else { "--:--".into() })),
                    ),
            )
            // Volume e volume igualado
            .child(
                div()
                    .w(px(280.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child(
                        div()
                            .id("music-normalize")
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(if m.normalize { theme::brand() } else { theme::line() })
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if m.normalize { theme::brand_ink() } else { theme::muted() })
                            .cursor_pointer()
                            .child("Volume igualado")
                            .on_click({
                                let core = core.clone();
                                let n = m.normalize;
                                move |_, _, _| core.music().set(None, None, None, Some(!n))
                            }),
                    )
                    .child(crate::ui::icon_button("music-mute", vol_icon).size_8().on_click(cx.listener(move |v, _, _, cx| {
                        if muted {
                            let back = v.unmuted.take().unwrap_or(0.8);
                            v.set_volume(back, cx);
                        } else {
                            v.unmuted = Some(m.volume);
                            v.set_volume(0., cx);
                        }
                    })))
                    .child(
                        div()
                            .id(SharedString::from("music-volume"))
                            .relative()
                            .w(px(96.))
                            .h_4()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(|v, e: &MouseDownEvent, _, cx| {
                                v.drag = Some(Drag::Volume);
                                v.drag_to(e.position.x, cx);
                                cx.notify();
                            }))
                            .child(canvas(move |b, _, _| vb.set(b), |_, _, _, _| {}).absolute().size_full())
                            .child(div().relative().w_full().h_1().rounded_full().bg(theme::white_a(0.15)).child(div().absolute().top_0().bottom_0().left_0().rounded_full().bg(theme::fg()).w(relative(m.volume.clamp(0., 1.))))),
                    ),
            )
            .into_any_element()
    }
}
