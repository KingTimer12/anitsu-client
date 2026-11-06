//! Pasta comum. Só pastas de nome curto (ex.: `Letra A`, `0-9`) viram índice em
//! blocos; pasta com músicas vira encarte.

use std::rc::Rc;

use gpui::{Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, prelude::*, px};
use higanbana_core::{
    library::{join_path, natural_cmp, split_path},
    nuvem::FileEntry,
};

use crate::{
    components::{breadcrumbs, columns, content_width, file_list, section_label, skeleton, FileAction},
    music::album::{AlbumView, looks_like_album},
    reader::ReadTarget,
    state::{AppState, state},
    theme,
    video::PlayTarget,
    views::load_listing,
};

pub struct FolderView {
    state: Entity<AppState>,
    path: String,
    listing: Option<Result<Vec<FileEntry>, String>>,
    album: Option<Entity<AlbumView>>,
    /// O encarte avisou que não era álbum: mostra a lista.
    not_album: bool,
}

impl FolderView {
    pub fn new(path: String, cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        load_listing(&state, path.clone(), cx, |v: &mut Self, r| {
            v.listing = Some(r);
        });
        Self { state, path, listing: None, album: None, not_album: false }
    }

    fn segs(&self) -> Vec<String> {
        split_path(&self.path).into_iter().map(str::to_owned).collect()
    }

    /// Nome da obra para o player: a pasta de cima quando esta é "Temporada"/release.
    fn anime_title(&self) -> String {
        let segs = self.segs();
        let last = segs.last().cloned().unwrap_or_default();
        let lower = last.to_lowercase();
        match segs.len().checked_sub(2).map(|i| segs[i].clone()) {
            Some(up) if lower.starts_with("temporada") || lower.starts_with("season") || last.starts_with('[') || last.starts_with('(') => up,
            _ => last,
        }
    }
}

impl Render for FolderView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = content_width(window, 1280.);
        let segs = self.segs();
        let title = if segs.len() == 1 { section_label(&segs[0]) } else { segs.last().cloned().unwrap_or_default() };

        let files = match &self.listing {
            Some(Ok(f)) => Some(f.clone()),
            _ => None,
        };
        // Pasta com músicas vira encarte (se não for álbum, o encarte avisa e volta à lista).
        if let Some(f) = &files
            && looks_like_album(f)
            && self.album.is_none()
        {
            let (path, files, work) = (self.path.clone(), f.clone(), segs.len().checked_sub(2).map(|i| segs[i].clone()));
            let view = cx.new(|cx| AlbumView::new(path, files, work, cx));
            cx.subscribe(&view, |v, _, e: &crate::music::album::AlbumEvent, cx| {
                if matches!(e, crate::music::album::AlbumEvent::NotAlbum) {
                    v.not_album = true;
                    cx.notify();
                }
            })
            .detach();
            self.album = Some(view);
        }
        let album_active = self.album.is_some() && !self.not_album;

        let body = match (&self.listing, files) {
            (None, _) | (Some(Ok(_)), None) => skeleton().h(px(288.)).rounded_2xl().into_any_element(),
            (Some(Err(e)), _) => div().text_color(theme::danger()).child(e.clone()).into_any_element(),
            (_, Some(_)) if album_active => self.album.clone().unwrap().into_any_element(),
            (_, Some(files)) => {
                let dirs: Vec<&FileEntry> = files.iter().filter(|f| f.is_directory).collect();
                let is_index = dirs.len() >= 6 && dirs.len() == files.len() && dirs.iter().all(|d| d.name.chars().count() <= 16);
                if is_index {
                    let mut names: Vec<String> = dirs.iter().map(|d| d.name.clone()).collect();
                    names.sort_by(|a, b| natural_cmp(a, b));
                    let cols = columns(width, 96., 12.);
                    div()
                        .grid()
                        .grid_cols(cols as u16)
                        .gap_3()
                        .children(names.into_iter().enumerate().map(|(i, name)| {
                            let short = name.trim_start_matches("Letra ").trim_start_matches('[').trim_end_matches(']').to_owned();
                            let big = short.chars().count() <= 3;
                            let target = join_path(&self.path, &name);
                            let st = self.state.clone();
                            let side = px(((width - (cols as f32 - 1.) * 12.) / cols as f32).floor());
                            div()
                                .id(SharedString::from(format!("tile-{i}")))
                                .w(side)
                                .h(side)
                                .flex()
                                .items_center()
                                .justify_center()
                                .p_2()
                                .rounded_2xl()
                                .border_1()
                                .border_color(theme::line())
                                .bg(gpui::Rgba { a: 0.7, ..theme::surface() })
                                .cursor_pointer()
                                .font_weight(FontWeight::SEMIBOLD)
                                .hover(|d| d.border_color(theme::brand()).text_color(theme::brand()))
                                .map(|d| if big { d.text_3xl() } else { d.text_sm() })
                                .child(short)
                                .on_click(move |_, _, cx| st.update(cx, |s, cx| s.go(target.clone(), cx)))
                        }))
                        .into_any_element()
                } else {
                    let (dir, all) = (self.path.clone(), files.clone());
                    let st = self.state.clone();
                    let anime_title = self.anime_title();
                    let on_play: FileAction = Rc::new({
                        let (dir, all, st) = (dir.clone(), all.clone(), st.clone());
                        move |f, _, cx| {
                            let t = PlayTarget { dir: dir.clone(), file: f.clone(), files: all.clone(), mal_id: None, is_anime: false, anime_title: anime_title.clone() };
                            st.update(cx, |s, cx| s.play(t, cx));
                        }
                    });
                    let title = segs.last().cloned().unwrap_or_default();
                    let on_read: FileAction = Rc::new(move |f, _, cx| {
                        let t = ReadTarget { dir: dir.clone(), file: f.clone(), files: all.clone(), title_path: dir.clone(), title: title.clone() };
                        st.update(cx, |s, cx| s.read(t, cx));
                    });
                    file_list(&self.path, &files, Some(on_play), Some(on_read), cx).into_any_element()
                }
            }
        };

        div()
            .w(px(width))
            .mx_auto()
            .py_6()
            .child(breadcrumbs(&self.path, cx))
            .when(!album_active, |d| d.child(div().mt_5().mb_8().text_3xl().font_weight(FontWeight::SEMIBOLD).child(title)))
            .when(album_active, |d| d.child(div().mt_6()))
            .child(body)
    }
}
