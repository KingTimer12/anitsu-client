//! Encarte do álbum ("encarte de CD japonês"): capa com obi, faixas por disco, arquivos.

use base64::Engine as _;
use gpui::{
    Context, Entity, EventEmitter, FontWeight, IntoElement, ObjectFit, ParentElement, Render, SharedString, Styled, Window, div, img,
    prelude::*, px, relative,
};
use higanbana_core::{
    music::album::{Album, format_duration, format_label},
    nuvem::FileEntry,
};

pub use higanbana_core::music::album::looks_like_album;

use crate::{
    components::{content_width, file_list, skeleton},
    state::{AppState, Img, state},
    theme,
    ui::{icon, truncate},
};

pub enum AlbumEvent {
    /// Não era álbum (nada tocável): quem abriu mostra a lista de arquivos.
    NotAlbum,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Tracks,
    Files,
}

pub struct AlbumView {
    state: Entity<AppState>,
    work: Option<String>,
    album: Option<Album>,
    tab: Tab,
    cover_key: Option<String>,
}

impl EventEmitter<AlbumEvent> for AlbumView {}

fn pad(n: Option<u32>, i: usize) -> String {
    format!("{:02}", n.unwrap_or(i as u32 + 1))
}

impl AlbumView {
    pub fn new(path: String, files: Vec<FileEntry>, work: Option<String>, cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let core = state.read(cx).core.clone();
        let task = core.clone().spawn(async move { core.cloud().load_album(&path, &files).await });
        cx.spawn(async move |this, cx| {
            let album = task.await.ok().flatten();
            let _ = this.update(cx, |v, cx| match album {
                None => cx.emit(AlbumEvent::NotAlbum),
                Some(a) => {
                    v.state.update(cx, |s, cx| s.music_show(&a, cx));
                    v.load_cover(&a, cx);
                    v.album = Some(a);
                    cx.notify();
                }
            });
        })
        .detach();
        Self { state, work, album: None, tab: Tab::Tracks, cover_key: None }
    }

    /// Capa da pasta; sem ela, a capa embutida na primeira faixa.
    fn load_cover(&mut self, a: &Album, cx: &mut Context<Self>) {
        let key = format!("album-cover:{}", a.path);
        self.cover_key = Some(key.clone());
        if self.state.read(cx).cached_image(&key).is_some() {
            return;
        }
        self.state.update(cx, |s, _| s.mark_image_loading(&key));
        let core = self.state.read(cx).core.clone();
        let cover = a.cover_path.clone().zip(a.cover_size);
        let first = a.tracks.first().cloned();
        let task = core.clone().spawn(async move {
            if let Some((path, size)) = cover
                && let Ok(b) = core.cloud().image(&path, size).await
            {
                return Some(b);
            }
            let t = first?;
            let meta = core.music().meta(&t.path, t.size, &t.ext, true).await.ok()?;
            let data = meta.cover?;
            base64::engine::general_purpose::STANDARD.decode(data.split_once(',')?.1).ok()
        });
        let state = self.state.clone();
        cx.spawn(async move |_, cx| {
            let bytes = task.await.ok().flatten();
            let _ = state.update(cx, |s, cx| match bytes {
                Some(b) => s.put_image(key, b, cx),
                None => s.fail_image(&key, cx),
            });
        })
        .detach();
    }
}

impl Render for AlbumView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(a) = self.album.clone() else { return skeleton().h(px(384.)).rounded_2xl().into_any_element() };
        let width = content_width(window, 1280.);
        let st = self.state.read(cx);
        let current = st.is_current_album(&a);
        let live = if current { st.music.clone() } else { None };
        let metas = st.music_session.metas.clone();
        let progress = st.core.music().store.data().progress.get(&a.path).copied();
        let cover = self.cover_key.as_ref().and_then(|k| st.cached_image(k));

        let obi = a.catalog.clone().or(a.year.clone());
        let format = live.as_ref().and_then(|l| l.format.clone()).map(|f| format_label(&f.codec, f.bits, f.sample_rate, f.lossless, f.resampled_to)).or_else(|| {
            a.tracks.iter().find_map(|t| metas.get(&t.path)).map(|m| format_label(&m.codec, m.bits, m.sample_rate, m.lossless, None))
        });
        let durations: Vec<Option<f64>> = a.tracks.iter().map(|t| metas.get(&t.path).and_then(|m| m.duration)).collect();
        let total_secs = durations.iter().all(Option::is_some).then(|| durations.iter().flatten().sum::<f64>());
        let resume = progress.filter(|p| (p.index > 0 || p.position > 5.) && p.index < a.tracks.len());
        let playing = live.as_ref().is_some_and(|l| l.playing);
        let ended = live.as_ref().is_some_and(|l| l.ended);
        let current_index = live.as_ref().and_then(|l| l.index);

        let cover_el = match cover {
            Some(Img::Ready(image)) => img(image).size_full().object_fit(ObjectFit::Cover).into_any_element(),
            _ => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p_4()
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .text_color(theme::muted())
                .child(a.title.clone())
                .into_any_element(),
        };

        let st_entity = self.state.clone();
        let main_label = if current && playing {
            "Pausar"
        } else if current && !ended {
            "Continuar"
        } else {
            "Tocar"
        };
        let album_for_play = a.clone();
        let main_button = div()
            .id("album-play")
            .flex()
            .items_center()
            .gap_2()
            .h_10()
            .px_4()
            .rounded_lg()
            .bg(theme::brand())
            .cursor_pointer()
            .font_weight(FontWeight::MEDIUM)
            .hover(|d| d.opacity(0.9))
            .child(icon(if current && playing { "pause" } else { "play" }).size_4().text_color(theme::white()))
            .child(main_label)
            .on_click({
                let st = st_entity.clone();
                move |_, _, cx| {
                    st.update(cx, |s, cx| {
                        if current && playing {
                            s.music_command("pause");
                        } else if current && !ended {
                            s.music_command("resume");
                        } else {
                            s.music_open(album_for_play.clone(), 0, 0., cx);
                        }
                    })
                }
            });

        let aside = div()
            .w(px(288.))
            .flex_none()
            .p_5()
            .rounded_xl()
            .bg(theme::surface())
            .child(
                div()
                    .relative()
                    .mr_5()
                    .child(div().w(px(228.)).h(px(228.)).overflow_hidden().rounded_md().bg(theme::elevated()).shadow_lg().child(cover_el))
                    .children(obi.map(|o| {
                        div()
                            .absolute()
                            .top_0()
                            .right(px(-20.))
                            .h_full()
                            .w_9()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .rounded_r_md()
                            .bg(theme::brand())
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .children(o.chars().map(|c| div().child(c.to_string())))
                    })),
            )
            .child(div().mt_5().text_2xl().font_weight(FontWeight::BOLD).line_height(relative(1.3)).child(a.title.clone()))
            .child(
                div()
                    .mt_1()
                    .text_sm()
                    .text_color(theme::muted())
                    .child([self.work.clone(), a.year.clone()].into_iter().flatten().collect::<Vec<_>>().join(" · ")),
            )
            .child(
                div().mt_4().flex().flex_wrap().gap_2().child(main_button).children(resume.filter(|_| !current).map(|p| {
                    let st = st_entity.clone();
                    let al = a.clone();
                    div()
                        .id("album-resume")
                        .flex()
                        .items_center()
                        .h_10()
                        .px_3()
                        .rounded_lg()
                        .border_1()
                        .border_color(theme::line_strong())
                        .cursor_pointer()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .hover(|d| d.border_color(theme::brand_ink()))
                        .child(format!("Continuar faixa {} ({})", p.index + 1, format_duration(p.position)))
                        .on_click(move |_, _, cx| st.update(cx, |s, cx| s.music_open(al.clone(), p.index, p.position, cx)))
                })),
            )
            .children(format.map(|f| div().mt_4().text_xs().text_color(theme::muted()).child(f)))
            .child(div().mt_1().text_xs().text_color(theme::faint()).child(format!(
                "{} faixas{}",
                a.tracks.len(),
                total_secs.map(|s| format!(" · {}", format_duration(s))).unwrap_or_default()
            )));

        let tab_button = |id: Tab, label: String, active: bool| {
            div()
                .id(SharedString::from(label.clone()))
                .pb_2()
                .border_b_2()
                .border_color(if active { theme::brand() } else { gpui::transparent_black().into() })
                .cursor_pointer()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(if active { theme::fg() } else { theme::muted() })
                .child(label)
                .on_click(cx.listener(move |v, _, _, cx| {
                    v.tab = id;
                    cx.notify();
                }))
        };
        let tabs = div()
            .mb_3()
            .flex()
            .gap_5()
            .border_b_1()
            .border_color(theme::line())
            .child(tab_button(Tab::Tracks, "Faixas".into(), self.tab == Tab::Tracks))
            .child(tab_button(Tab::Files, format!("Arquivos ({})", a.others.len()), self.tab == Tab::Files));

        let body = match self.tab {
            Tab::Files => {
                let others = a.others.clone();
                div()
                    .child(div().mb_3().text_sm().text_color(theme::muted()).child("Só as faixas nesta pasta."))
                    .child(file_list(&a.path, &others, None, None, cx))
                    .into_any_element()
            }
            Tab::Tracks => div()
                .flex()
                .flex_col()
                .gap_4()
                .children(a.discs.iter().map(|&d| {
                    let rows: Vec<(usize, higanbana_core::music::album::Track)> = a.tracks.iter().cloned().enumerate().filter(|(_, t)| t.disc == d).collect();
                    div()
                        .when(a.discs.len() > 1, |x| {
                            x.child(div().mb_1().px_3().flex().items_center().gap_2().text_xs().text_color(theme::faint()).child(icon("disc-3").size_3p5().text_color(theme::faint())).child(format!("Disco {d}")))
                        })
                        .children(rows.into_iter().map(|(i, t)| {
                            let meta = metas.get(&t.path);
                            let on = current_index == Some(i) && current;
                            let st = st_entity.clone();
                            let al = a.clone();
                            div()
                                .id(SharedString::from(format!("track-{i}")))
                                .flex()
                                .items_center()
                                .gap_3()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .cursor_pointer()
                                .map(|x| if on { x.bg(gpui::Rgba { a: 0.1, ..theme::brand() }) } else { x.hover(|s| s.bg(theme::white_a(0.04))) })
                                .child(
                                    div()
                                        .w_12()
                                        .flex_none()
                                        .text_xl()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(if on { theme::brand_ink() } else { theme::faint() })
                                        .child(if on && playing { icon("audio-lines").text_color(theme::brand_ink()).into_any_element() } else { div().child(pad(t.number, i)).into_any_element() }),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(truncate(meta.and_then(|m| m.title.clone()).filter(|s| !s.is_empty()).unwrap_or(t.title.clone())).when(on, |x| x.text_color(theme::brand_ink())))
                                        .children(meta.and_then(|m| m.artist.clone()).map(|ar| truncate(ar).text_xs().text_color(theme::muted()))),
                                )
                                .child(div().text_sm().text_color(theme::muted()).child(meta.and_then(|m| m.duration).map(format_duration).unwrap_or_default()))
                                .on_click(move |_, _, cx| {
                                    st.update(cx, |s, cx| if on { s.music_command("toggle") } else { s.music_open(al.clone(), i, 0., cx) })
                                })
                        }))
                }))
                .into_any_element(),
        };

        div()
            .w(px(width))
            .flex()
            .items_start()
            .gap_10()
            .child(aside)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(tabs)
                    .child(body),
            )
            .into_any_element()
    }
}
