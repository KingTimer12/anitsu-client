//! Pasta cujos filhos são títulos (ex.: `Animes/Letra B`) → grade de pôsteres.

use gpui_kit::{
    Context, ElementId, Entity, FontWeight, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    assets::IconName, div, prelude::*, px,
};
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Selectable as _, Sizable as _, h_flex, v_flex,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
};
use higanbana_core::{
    anilist::MediaType,
    library::{base_name, join_path, natural_cmp, parent_path},
    nuvem::FileEntry,
};

use crate::{
    components::{breadcrumbs, columns, content_width, poster, want_lookup, when_visible, PosterProps},
    state::{AppState, state},
    ui::empty_state,
    views::load_listing,
};

pub struct ShelfView {
    state: Entity<AppState>,
    path: String,
    media: MediaType,
    listing: Option<Result<Vec<FileEntry>, String>>,
    siblings: Option<Vec<FileEntry>>,
    filter: Entity<InputState>,
    _subs: Vec<Subscription>,
}

impl ShelfView {
    pub fn new(path: String, media: MediaType, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        let name = base_name(&path).to_owned();
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder(format!("Filtrar em {name}")));
        let subs = vec![
            cx.observe(&state, |_, _, cx| cx.notify()),
            cx.subscribe(&filter, |_, _, e: &InputEvent, cx| {
                if matches!(e, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        load_listing(&state, path.clone(), cx, |v: &mut Self, r, _| v.listing = Some(r));
        load_listing(&state, parent_path(&path), cx, |v: &mut Self, r, _| v.siblings = r.ok());
        Self { state, path, media, listing: None, siblings: None, filter, _subs: subs }
    }
}

impl Render for ShelfView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let rem = f32::from(window.rem_size());
        let width = content_width(window, 80. * rem);
        let name = base_name(&self.path).to_owned();
        let query = self.filter.read(cx).value().trim().to_owned();
        let q = query.to_lowercase();
        let titles: Vec<FileEntry> = match &self.listing {
            Some(Ok(files)) => {
                let mut t: Vec<FileEntry> =
                    files.iter().filter(|f| f.is_directory && (q.is_empty() || f.name.to_lowercase().contains(&q))).cloned().collect();
                t.sort_by(|a, b| natural_cmp(&a.name, &b.name));
                t
            }
            _ => Vec::new(),
        };
        let gap = 1.25 * rem;
        let cols = columns(width, 10. * rem, gap);
        let card_w = px(((width - (cols as f32 - 1.) * gap) / cols as f32).floor());
        let parent = parent_path(&self.path);

        // Letras irmãs: escolha de uma entre muitas, com a atual selecionada.
        let letters = self.siblings.as_ref().map(|sib| {
            h_flex().flex_wrap().gap_1().children(sib.iter().filter(|f| f.is_directory).map(|s| {
                let target = join_path(&parent, &s.name);
                let st = self.state.clone();
                Button::new(ElementId::Name(format!("letter:{}", s.name).into()))
                    .ghost()
                    .small()
                    .label(s.name.trim_start_matches("Letra ").to_owned())
                    .selected(s.name == name)
                    .on_click(move |_, _, cx| st.update(cx, |s, cx| s.go(target.clone(), cx)))
            }))
        });

        let grid = match &self.listing {
            None => div().grid().grid_cols(cols as u16).gap_x_5().gap_y_8().children((0..cols * 2).map(|_| Skeleton::new().w(card_w).h(card_w * 1.5).rounded(theme.radius_lg))),
            Some(Err(e)) => empty_state(IconName::WifiOff, "Não foi possível abrir esta pasta", Some(e.clone().into()), cx),
            Some(Ok(_)) if titles.is_empty() => empty_state(IconName::Search, format!("Nenhum título com “{query}” em {name}"), None, cx),
            Some(Ok(_)) => div().grid().grid_cols(cols as u16).gap_x_5().gap_y_8().children(titles.into_iter().map(|t| {
                let looked = self.state.read(cx).lookup(&t.name, self.media);
                let target = join_path(&self.path, &t.name);
                let st = self.state.clone();
                let card = poster(
                    PosterProps {
                        id: ElementId::Name(format!("title:{}", t.name).into()),
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
        let count = match &self.listing {
            Some(Ok(f)) => Some(f.iter().filter(|f| f.is_directory).count()),
            _ => None,
        };

        v_flex()
            .w(px(width))
            .mx_auto()
            .py_6()
            .gap_6()
            .child(breadcrumbs(&self.path, cx))
            .child(
                h_flex()
                    .flex_wrap()
                    .items_end()
                    .justify_between()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().text_2xl().font_weight(FontWeight::SEMIBOLD).child(name.clone()))
                            .children(count.map(|n| div().text_sm().text_color(theme.muted_foreground).child(format!("{n} títulos")))),
                    )
                    .child(div().w_72().child(Input::new(&self.filter).prefix(Icon::new(IconName::Search).small()).cleanable(true))),
            )
            .children(letters)
            .child(grid)
    }
}
