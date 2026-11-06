//! Pasta cujos filhos são títulos (ex.: `Animes/Letra B`) → grade de pôsteres.

use gpui::{Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, prelude::*, px};
use higanbana_core::{
    anilist::MediaType,
    library::{base_name, join_path, natural_cmp, parent_path},
    nuvem::FileEntry,
};

use crate::{
    components::{breadcrumbs, columns, content_width, poster, skeleton, want_lookup, when_visible, PosterProps},
    input::{InputEvent, TextInput},
    state::{AppState, state},
    theme,
    ui::icon,
    views::load_listing,
};

pub struct ShelfView {
    state: Entity<AppState>,
    path: String,
    media: MediaType,
    listing: Option<Result<Vec<FileEntry>, String>>,
    siblings: Option<Vec<FileEntry>>,
    filter: Entity<TextInput>,
}

impl ShelfView {
    pub fn new(path: String, media: MediaType, cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let name = base_name(&path).to_owned();
        let filter = cx.new(|cx| TextInput::new(format!("Filtrar em {name}"), cx));
        cx.subscribe(&filter, |_, _, e: &InputEvent, cx| {
            if matches!(e, InputEvent::Changed) {
                cx.notify();
            }
        })
        .detach();
        load_listing(&state, path.clone(), cx, |v: &mut Self, r| v.listing = Some(r));
        load_listing(&state, parent_path(&path), cx, |v: &mut Self, r| v.siblings = r.ok());
        Self { state, path, media, listing: None, siblings: None, filter }
    }
}

impl Render for ShelfView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = content_width(window, 1280.);
        let name = base_name(&self.path).to_owned();
        let q = self.filter.read(cx).text().trim().to_lowercase();
        let titles: Vec<FileEntry> = match &self.listing {
            Some(Ok(files)) => {
                let mut t: Vec<FileEntry> =
                    files.iter().filter(|f| f.is_directory && (q.is_empty() || f.name.to_lowercase().contains(&q))).cloned().collect();
                t.sort_by(|a, b| natural_cmp(&a.name, &b.name));
                t
            }
            _ => Vec::new(),
        };
        let cols = columns(width, 160., 20.);
        let card_w = px(((width - (cols as f32 - 1.) * 20.) / cols as f32).floor());
        let parent = parent_path(&self.path);

        let letters = self.siblings.as_ref().map(|sib| {
            div().mt_6().py_3().border_b_1().border_color(theme::line()).flex().flex_wrap().gap_1().children(
                sib.iter().filter(|f| f.is_directory).enumerate().map(|(i, s)| {
                    let active = s.name == name;
                    let target = join_path(&parent, &s.name);
                    let st = self.state.clone();
                    div()
                        .id(SharedString::from(format!("letter-{i}")))
                        .h_9()
                        .min_w_9()
                        .px_2p5()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_lg()
                        .cursor_pointer()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .map(|d| if active { d.bg(theme::brand()).text_color(theme::white()) } else { d.text_color(theme::muted()).hover(|s| s.bg(theme::white_a(0.08)).text_color(theme::fg())) })
                        .child(s.name.trim_start_matches("Letra ").to_owned())
                        .on_click(move |_, _, cx| st.update(cx, |s, cx| s.go(target.clone(), cx)))
                }),
            )
        });

        let grid = match &self.listing {
            None => div().grid().grid_cols(cols as u16).gap_x_5().gap_y_8().children((0..12).map(|_| skeleton().w(card_w).h(card_w * 1.5))),
            Some(Err(e)) => div().text_color(theme::danger()).child(e.clone()),
            Some(Ok(_)) if titles.is_empty() => div()
                .py_16()
                .flex()
                .justify_center()
                .text_color(theme::muted())
                .child(format!("Nenhum título com \"{}\" em {name}.", self.filter.read(cx).text())),
            Some(Ok(_)) => div().grid().grid_cols(cols as u16).gap_x_5().gap_y_8().children(titles.into_iter().enumerate().map(|(i, t)| {
                let looked = self.state.read(cx).lookup(&t.name, self.media);
                let target = join_path(&self.path, &t.name);
                let st = self.state.clone();
                let card = poster(
                    PosterProps {
                        id: SharedString::from(format!("title-{i}")).into(),
                        name: t.name.clone(),
                        media: looked.clone().flatten(),
                        loading: looked.is_none(),
                        width: card_w,
                        badge: false,
                        dimmed: false,
                    },
                    cx,
                )
                .on_click(move |_, _, cx| st.update(cx, |s, cx| s.go(target.clone(), cx)));
                div().relative().child(card).when(looked.is_none(), |d| d.child(when_visible(400., want_lookup(t.name.clone(), self.media))))
            })),
        };

        div()
            .w(px(width))
            .mx_auto()
            .py_6()
            .child(breadcrumbs(&self.path, cx))
            .child(
                div()
                    .mt_5()
                    .flex()
                    .flex_wrap()
                    .items_end()
                    .justify_between()
                    .gap_4()
                    .child(
                        div().child(div().text_3xl().font_weight(FontWeight::SEMIBOLD).child(name.clone())).children(
                            matches!(self.listing, Some(Ok(_))).then(|| {
                                let n = match &self.listing {
                                    Some(Ok(f)) => f.iter().filter(|f| f.is_directory).count(),
                                    _ => 0,
                                };
                                div().mt_1().text_sm().text_color(theme::muted()).child(format!("{n} títulos"))
                            }),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .h_10()
                            .w(px(288.))
                            .px_3()
                            .rounded_xl()
                            .border_1()
                            .border_color(theme::line())
                            .bg(gpui::Rgba { a: 0.7, ..theme::surface() })
                            .child(icon("search").size_4().text_color(theme::faint()))
                            .child(self.filter.clone()),
                    ),
            )
            .children(letters)
            .child(div().mt_8().child(grid))
    }
}
