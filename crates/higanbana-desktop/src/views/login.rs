//! Entrada: parede de capas da temporada e "Entrar com Discord".

use gpui::{Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, prelude::*, px, relative};
use higanbana_core::anilist::types::MediaSummary;

use crate::{
    components::{hex_color, remote_image},
    state::{AppState, state},
    theme,
    ui::{button, icon, spinner},
};

pub struct LoginView {
    state: Entity<AppState>,
    covers: Vec<MediaSummary>,
    busy: bool,
    error: Option<SharedString>,
}

impl LoginView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let core = state.read(cx).core.clone();
        let task = core.clone().spawn(async move { core.anilist().season().await });
        cx.spawn(async move |this, cx| {
            let list: Vec<MediaSummary> = task.await.ok().and_then(Result::ok).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
            let _ = this.update(cx, |v, cx| {
                v.covers = list.into_iter().filter(|m| m.cover_small().is_some()).take(30).collect();
                cx.notify();
            });
        })
        .detach();
        Self { state, covers: Vec::new(), busy: false, error: None }
    }

    fn login(&mut self, cx: &mut Context<Self>) {
        self.busy = true;
        self.error = None;
        cx.notify();
        let core = self.state.read(cx).core.clone();
        let task = core.clone().spawn(async move { core.auth().login().await });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            let _ = this.update(cx, |v, cx| {
                v.busy = false;
                match r {
                    Ok(Err(e)) if e.kind() != "cancelled" => v.error = Some(format!("{e}. Tente entrar de novo.").into()),
                    Err(e) => v.error = Some(e.to_string().into()),
                    _ => {}
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for LoginView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vw = f32::from(window.viewport_size().width);
        let core = self.state.read(cx).core.clone();
        let cover_w = px(((vw * 0.62) / 5. - 16.).max(80.));
        let wall = div()
            .absolute()
            .top(px(-120.))
            .left(relative(0.38))
            .right(px(-80.))
            .flex()
            .gap_4()
            .opacity(0.6)
            .children((0..5).map(|col| {
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .when(col % 2 == 1, |d| d.mt(cover_w * 0.5))
                    .children(self.covers.iter().skip(col).step_by(5).map(|m| {
                        let bg = hex_color(m.color()).unwrap_or(theme::elevated());
                        div().w(cover_w).h(cover_w * 1.5).flex_none().rounded_2xl().overflow_hidden().bg(bg).child(remote_image(m.cover_small(), cx))
                    }))
            }));

        let action = if self.busy {
            let (c1, c2) = (core.clone(), core.clone());
            div()
                .p_5()
                .rounded_2xl()
                .border_1()
                .border_color(theme::line_strong())
                .bg(gpui::Rgba { a: 0.85, ..theme::surface() })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(div().size_5().text_color(theme::brand()).child(spinner("login")))
                        .child(div().font_weight(FontWeight::SEMIBOLD).child("Continue no seu navegador")),
                )
                .child(
                    div()
                        .mt_2()
                        .text_sm()
                        .text_color(theme::muted())
                        .child("Abrimos o Discord no navegador em que você já está logado. Clique em Autorizar e volte para cá."),
                )
                .child(
                    div()
                        .mt_4()
                        .flex()
                        .gap_2()
                        .child(button("reopen", "Abrir de novo", false).on_click(move |_, _, _| {
                            let c = c1.clone();
                            c1.spawn(async move { c.auth().reopen_login().await });
                        }))
                        .child(button("cancel", "Cancelar", false).on_click(move |_, _, _| {
                            let c = c2.clone();
                            c2.spawn(async move { c.auth().cancel_login().await });
                        })),
                )
                .into_any_element()
        } else {
            div()
                .id("discord")
                .flex()
                .items_center()
                .gap_3()
                .h_12()
                .px_6()
                .rounded_2xl()
                .bg(gpui::rgb(0x5865f2))
                .cursor_pointer()
                .font_weight(FontWeight::SEMIBOLD)
                .hover(|d| d.opacity(0.9))
                .child(icon("log-in").text_color(theme::white()))
                .child("Entrar com Discord")
                .on_click(cx.listener(|v, _, _, cx| v.login(cx)))
                .into_any_element()
        };

        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(theme::bg())
            .child(wall)
            .child(div().absolute().inset_0().bg(gpui::linear_gradient(
                90.,
                gpui::linear_color_stop(theme::bg(), 0.35),
                gpui::linear_color_stop(gpui::Rgba { a: 0.25, ..theme::bg() }, 1.),
            )))
            .child(
                div().relative().size_full().flex().items_center().px(px((vw * 0.08).clamp(32., 120.))).child(
                    div()
                        .w(px(448.))
                        .child(div().text_size(px(72.)).font_weight(FontWeight::BOLD).text_color(theme::brand()).child("higanbana"))
                        .child(
                            div()
                                .mt_4()
                                .max_w(px(360.))
                                .text_lg()
                                .text_color(theme::muted())
                                .child("A nuvem de animes da Anitsu, com capa, sinopse e ficha técnica de cada título."),
                        )
                        .child(div().mt_10().child(action))
                        .children(self.error.clone().map(|e| div().mt_4().text_sm().text_color(theme::danger()).child(e)))
                        .child(div().mt_16().text_xs().text_color(theme::faint()).child("Capas e informações do AniList.")),
                ),
            )
    }
}
