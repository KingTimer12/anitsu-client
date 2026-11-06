//! Início: saudação, seções do acervo e a temporada atual do AniList.

use gpui::{Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, prelude::*, px};
use higanbana_core::anilist::{season_at, types::MediaSummary};

use crate::{
    components::{columns, content_width, poster, section_meta, section_order, section_title, skeleton, when_visible, PosterProps},
    state::{AppState, Tone, state},
    theme,
    ui::{icon, spinner},
};

pub struct HomeView {
    state: Entity<AppState>,
    sections: Option<Result<Vec<String>, String>>,
    /// `None` = carregando.
    season: Option<Vec<MediaSummary>>,
}

impl HomeView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let core = state.read(cx).core.clone();

        let c = core.clone();
        let root = core.spawn(async move { c.cloud().list("").await.map_err(|e| e.to_string()) });
        cx.spawn(async move |this, cx| {
            let r = root.await.map_err(|e| e.to_string()).and_then(|r| r);
            let _ = this.update(cx, |v, cx| {
                v.sections = Some(r.map(|l| {
                    let mut s: Vec<String> = l.files.into_iter().filter(|f| f.is_directory).map(|f| f.name).collect();
                    s.sort_by(|a, b| section_order(a, b));
                    s
                }));
                cx.notify();
            });
        })
        .detach();

        let c = core.clone();
        let season = core.spawn(async move { c.anilist().season().await });
        cx.spawn(async move |this, cx| {
            let list = season.await.ok().and_then(Result::ok).and_then(|v| serde_json::from_value::<Vec<MediaSummary>>(v).ok()).unwrap_or_default();
            let _ = this.update(cx, |v, cx| {
                v.season = Some(list);
                cx.notify();
            });
        })
        .detach();

        Self { state, sections: None, season: None }
    }

    fn open_season(&mut self, m: MediaSummary, cx: &mut Context<Self>) {
        let state = self.state.clone();
        if let Some(Some(path)) = state.read(cx).catalog(m.id) {
            state.update(cx, |s, cx| s.go(path, cx));
            return;
        }
        let core = state.read(cx).core.clone();
        let media = m.clone();
        let task = core.clone().spawn(async move { core.check_catalog(&media).await });
        cx.spawn(async move |_, cx| {
            let path = task.await.ok().flatten();
            let _ = state.update(cx, |s, cx| match path {
                Some(p) => s.go(p, cx),
                None => s.toast(format!("\"{}\" ainda não está na nuvem.", m.title()), Tone::Info, cx),
            });
        })
        .detach();
    }
}

fn season_label(code: &str) -> &'static str {
    match code {
        "WINTER" => "inverno",
        "SPRING" => "primavera",
        "SUMMER" => "verão",
        _ => "outono",
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = content_width(window, 1280.);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        let (season_code, year) = season_at(now);
        let name = self.state.read(cx).user.as_ref().map(|u| u.name.clone());

        // --- Seções ---
        let sec_cols = columns(width, 240., 12.);
        let sections = match &self.sections {
            None => div().grid().grid_cols(sec_cols as u16).gap_3().children((0..6).map(|_| skeleton().h(px(112.)).rounded_2xl())),
            Some(Err(_)) => div().text_color(theme::danger()).child("Não foi possível abrir a nuvem. Verifique sua conexão e reabra o app."),
            Some(Ok(list)) => div().grid().grid_cols(sec_cols as u16).gap_3().children(list.iter().enumerate().map(|(i, s)| {
                let (icon_name, blurb, tint) = section_meta(s);
                let label = crate::components::section_label(s);
                let target = s.clone();
                let state = self.state.clone();
                div()
                    .id(SharedString::from(format!("section-{i}")))
                    .flex()
                    .items_center()
                    .gap_4()
                    .h(px(112.))
                    .p_5()
                    .rounded_2xl()
                    .border_1()
                    .border_color(theme::line())
                    .bg(gpui::Rgba { a: 0.7, ..theme::surface() })
                    .cursor_pointer()
                    .hover(|d| d.border_color(theme::line_strong()).bg(theme::elevated()))
                    .child(
                        div()
                            .size_12()
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_xl()
                            .bg(gpui::Rgba { a: 0.12, ..tint })
                            .child(icon(icon_name).size_6().text_color(tint)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(label))
                            .child(div().text_sm().text_color(theme::muted()).line_clamp(2).child(blurb)),
                    )
                    .child(icon("chevron-right").text_color(theme::faint()))
                    .on_click(move |_, _, cx| state.update(cx, |s, cx| s.go(target.clone(), cx)))
            })),
        };

        // --- Temporada ---
        let cols = columns(width, 150., 20.);
        let card_w = px(((width - (cols as f32 - 1.) * 20.) / cols as f32).floor());
        let season = match &self.season {
            None => div().grid().grid_cols(cols as u16).gap_x_5().gap_y_7().children((0..6).map(|_| skeleton().w(card_w).h(card_w * 1.5))),
            Some(list) if list.is_empty() => div().text_sm().text_color(theme::muted()).child("Não foi possível carregar a temporada agora."),
            Some(list) => {
                let list = list.clone();
                div().grid().grid_cols(cols as u16).gap_x_5().gap_y_7().children(list.into_iter().map(|m| {
                    let known = self.state.read(cx).catalog(m.id);
                    let id = m.id;
                    let media = m.clone();
                    let card = poster(
                        PosterProps {
                            id: SharedString::from(format!("season-{id}")).into(),
                            name: m.title(),
                            media: Some(m.clone()),
                            loading: false,
                            width: card_w,
                            badge: matches!(known, Some(Some(_))),
                            dimmed: matches!(known, Some(None)),
                        },
                        cx,
                    )
                    .on_click(cx.listener(move |v, _, _, cx| v.open_season(media.clone(), cx)));
                    let st = self.state.clone();
                    div().relative().child(card).child(when_visible(200., move |cx| st.update(cx, |s, cx| s.want_catalog(&m, cx))))
                }))
            }
        };
        let (checked, in_nuvem, total) = self.season.as_ref().map_or((0, 0, 0), |l| {
            let st = self.state.read(cx);
            let known: Vec<_> = l.iter().map(|m| st.catalog(m.id)).collect();
            (known.iter().filter(|k| k.is_some()).count(), known.iter().filter(|k| matches!(k, Some(Some(_)))).count(), l.len())
        });
        let aside = (total > 0).then(|| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_xs()
                .text_color(theme::faint())
                .when(checked < total, |d| d.child(div().size_3p5().child(spinner("season-check"))))
                .child(format!("{in_nuvem} no acervo"))
                .into_any_element()
        });

        div()
            .w(px(width))
            .mx_auto()
            .py_10()
            .child(div().text_3xl().font_weight(FontWeight::SEMIBOLD).child(match name {
                Some(n) => format!("Olá, {n}"),
                None => "Bem-vindo de volta".into(),
            }))
            .child(div().mt_2().text_color(theme::muted()).child("O que vamos assistir hoje? Busque com Ctrl K ou escolha uma seção."))
            .child(div().mt_10().child(sections))
            .child(
                div()
                    .mt_12()
                    .child(section_title(format!("Temporada de {} {year}", season_label(season_code)), aside))
                    .child(season),
            )
    }
}
