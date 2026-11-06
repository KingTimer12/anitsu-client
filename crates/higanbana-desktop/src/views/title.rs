//! Página de um título. Estrutura esperada na nuvem:
//!   <Título>/[ANO] <Temporada>/<Release [tags]>/arquivos
//! Temporadas = subpastas com ano; releases = subpastas da temporada.
//! Cada temporada casa com sua própria entrada no AniList.

use std::{collections::HashSet, rc::Rc};

use gpui::{
    AnyElement, Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
    prelude::*, px, relative,
};
use higanbana_core::{
    anilist::{
        MediaType, text,
        types::{MediaDetail, MediaSummary},
    },
    library::{base_name, folder_title, folder_year, join_path, natural_cmp, release_tags},
    nuvem::FileEntry,
    player::logic::{episode_number, format_time, video_files},
    reader::logic::{chapter_label, comic_files, label_text},
};

use crate::{
    components::{
        breadcrumbs, chip, chip_group, columns, content_width, cover, file_list, file_paths, hex_color, remote_image, skeleton,
        FileAction,
    },
    music::album::{AlbumEvent, AlbumView, looks_like_album},
    reader::{ReadTarget, progress_map},
    state::{AppState, DlStatus, state},
    theme,
    ui::{icon, truncate},
    video::PlayTarget,
    views::load_listing,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Files,
    Cast,
    Staff,
    Related,
    Details,
}

type Listing = Option<Result<Vec<FileEntry>, String>>;

pub struct TitleView {
    state: Entity<AppState>,
    title_path: String,
    rest: Vec<String>,
    media_type: MediaType,
    title_listing: Listing,
    season_listing: Listing,
    files_listing: Listing,
    /// Caminho cuja listagem está em `files_listing` (ou sendo buscada).
    files_for: Option<String>,
    season_for: Option<String>,
    detail: Option<Option<MediaDetail>>,
    detail_for: Option<i64>,
    tab: Tab,
    expanded: bool,
    album: Option<Entity<AlbumView>>,
    not_album: bool,
}

fn dirs(files: &Listing) -> Vec<FileEntry> {
    let mut v: Vec<FileEntry> = match files {
        Some(Ok(f)) => f.iter().filter(|f| f.is_directory).cloned().collect(),
        _ => Vec::new(),
    };
    v.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    v
}

fn files_of(l: &Listing) -> Option<&Vec<FileEntry>> {
    match l {
        Some(Ok(f)) => Some(f),
        _ => None,
    }
}

impl TitleView {
    pub fn new(title_path: String, rest: Vec<String>, media_type: MediaType, cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |v: &mut Self, _, cx| {
            v.sync(cx);
            cx.notify();
        })
        .detach();
        load_listing(&state, title_path.clone(), cx, |v: &mut Self, r| v.title_listing = Some(r));
        Self {
            state,
            title_path,
            rest,
            media_type,
            title_listing: None,
            season_listing: None,
            files_listing: None,
            files_for: None,
            season_for: None,
            detail: None,
            detail_for: None,
            tab: Tab::Files,
            expanded: false,
            album: None,
            not_album: false,
        }
    }

    fn seasons(&self) -> Vec<FileEntry> {
        dirs(&self.title_listing).into_iter().filter(|d| folder_year(&d.name).is_some()).collect()
    }

    fn season_name(&self) -> Option<String> {
        let seasons = self.seasons();
        let first = seasons.first()?.name.clone();
        let wanted = self.rest.first();
        Some(wanted.filter(|w| seasons.iter().any(|s| &&s.name == w)).cloned().unwrap_or(first))
    }

    fn season_path(&self) -> String {
        self.season_name().map_or_else(|| self.title_path.clone(), |s| join_path(&self.title_path, &s))
    }

    fn rest_after_season(&self) -> Vec<String> {
        match (self.season_name(), self.rest.first()) {
            (Some(s), Some(r)) if &s == r => self.rest[1..].to_vec(),
            _ => self.rest.clone(),
        }
    }

    fn releases(&self) -> Vec<FileEntry> {
        if self.season_name().is_some() {
            dirs(&self.season_listing)
        } else {
            dirs(&self.title_listing).into_iter().filter(|d| folder_year(&d.name).is_none()).collect()
        }
    }

    fn release_name(&self) -> Option<String> {
        let releases = self.releases();
        let first = releases.first()?.name.clone();
        let rest = self.rest_after_season();
        Some(rest.first().filter(|w| releases.iter().any(|r| &&r.name == w)).cloned().unwrap_or(first))
    }

    /// Pasta dos arquivos, ou `None` enquanto as listagens de cima não chegaram.
    fn files_path(&self) -> Option<String> {
        self.title_listing.as_ref()?;
        if self.season_name().is_some() && self.season_listing.is_none() {
            return None;
        }
        let season_path = self.season_path();
        let Some(release) = self.release_name() else { return Some(season_path) };
        let rest = self.rest_after_season();
        let deeper: Vec<String> = if rest.first() == Some(&release) { rest[1..].to_vec() } else { Vec::new() };
        Some(std::iter::once(season_path).chain(std::iter::once(release)).chain(deeper).collect::<Vec<_>>().join("/"))
    }

    fn lookup_name(&self) -> String {
        self.season_name().unwrap_or_else(|| base_name(&self.title_path).to_owned())
    }

    fn summary(&self, cx: &gpui::App) -> Option<Option<MediaSummary>> {
        self.state.read(cx).lookup(&self.lookup_name(), self.media_type)
    }

    fn media(&self, cx: &gpui::App) -> Option<MediaSummary> {
        match &self.detail {
            Some(Some(d)) => Some(d.summary.clone()),
            _ => self.summary(cx).flatten(),
        }
    }

    fn full(&self) -> Option<&MediaDetail> {
        self.detail.as_ref().and_then(|d| d.as_ref())
    }

    /// Busca o que falta conforme as listagens e o AniList respondem.
    fn sync(&mut self, cx: &mut Context<Self>) {
        if self.title_listing.is_none() {
            return;
        }
        if let Some(season) = self.season_name()
            && self.season_for.as_deref() != Some(season.as_str())
        {
            self.season_for = Some(season);
            self.season_listing = None;
            load_listing(&self.state, self.season_path(), cx, |v: &mut Self, r| v.season_listing = Some(r));
        }
        if let Some(path) = self.files_path()
            && self.files_for.as_deref() != Some(path.as_str())
        {
            self.files_for = Some(path.clone());
            self.files_listing = None;
            self.album = None;
            self.not_album = false;
            load_listing(&self.state, path, cx, |v: &mut Self, r| v.files_listing = Some(r));
        }
        let name = self.lookup_name();
        let media_type = self.media_type;
        let summary = self.summary(cx);
        if summary.is_none() {
            self.state.update(cx, |s, cx| s.want_lookup(&name, media_type, cx));
        }
        if let Some(Some(m)) = summary
            && self.detail_for != Some(m.id)
        {
            self.detail_for = Some(m.id);
            let core = self.state.read(cx).core.clone();
            let id = m.id;
            let task = core.clone().spawn(async move { core.anilist().media(id).await });
            cx.spawn(async move |this, cx| {
                let d = task.await.ok().and_then(Result::ok).and_then(|v| serde_json::from_value::<MediaDetail>(v).ok());
                let _ = this.update(cx, |v, cx| {
                    v.detail = Some(d);
                    cx.notify();
                });
            })
            .detach();
        }
    }

    fn heading(&self, cx: &gpui::App) -> String {
        self.media(cx).map_or_else(|| folder_title(&self.lookup_name()), |m| m.title())
    }

    fn play(&self, file: FileEntry, cx: &mut gpui::App) {
        let (Some(dir), Some(files)) = (self.files_for.clone(), files_of(&self.files_listing).cloned()) else { return };
        let t = PlayTarget {
            dir,
            file,
            files,
            mal_id: self.full().and_then(|d| d.id_mal),
            is_anime: self.media_type == MediaType::Anime,
            anime_title: self.heading(cx),
        };
        self.state.update(cx, |s, cx| s.play(t, cx));
    }

    fn read(&self, file: FileEntry, cx: &mut gpui::App) {
        let (Some(dir), Some(files)) = (self.files_for.clone(), files_of(&self.files_listing).cloned()) else { return };
        let t = ReadTarget { dir, file, files, title_path: self.title_path.clone(), title: self.heading(cx) };
        self.state.update(cx, |s, cx| s.read(t, cx));
    }
}

impl Render for TitleView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync(cx);
        let width = content_width(window, 1152.);
        let media = self.media(cx);
        let full = self.full().cloned();
        let heading = self.heading(cx);
        let accent = media.as_ref().and_then(|m| hex_color(m.color())).unwrap_or(theme::accent());
        let files = files_of(&self.files_listing).cloned();
        let files_path = self.files_for.clone().unwrap_or_default();
        let st = self.state.read(cx);
        let progress = st.player_data.progress.clone();
        let reading = progress_map(st);

        // --- Assistir / continuar ---
        let videos = files.as_ref().map(|f| video_files(f)).unwrap_or_default();
        let resume = {
            let mut last: Option<(usize, f64, bool, i64)> = None;
            for (i, f) in videos.iter().enumerate() {
                if let Some(p) = progress.get(&join_path(&files_path, &f.name))
                    && last.is_none_or(|l| p.updated_at >= l.3)
                {
                    last = Some((i, p.position, p.watched, p.updated_at));
                }
            }
            match last {
                Some((i, at, false, _)) => Some((videos[i].clone(), at)),
                Some((i, _, true, _)) => videos.get(i + 1).cloned().map(|f| (f, 0.)),
                None => videos.first().cloned().map(|f| (f, 0.)),
            }
        };
        let resume_label = |f: &FileEntry, at: f64| {
            let ep = episode_number(&f.name).map(|n| format!("ep. {n}")).unwrap_or_default();
            if at > 0. { format!("Continuar {ep} ({})", format_time(at)).replace("  ", " ") } else { format!("Assistir {ep}").trim().to_owned() }
        };

        // --- Ler / continuar ---
        let comics = files.as_ref().map(|f| comic_files(f)).unwrap_or_default();
        let resume_read = {
            let mut best: Option<(usize, usize, bool, i64, Option<String>)> = None;
            for (i, f) in comics.iter().enumerate() {
                if let Some(p) = reading.get(&join_path(&files_path, &f.name))
                    && best.as_ref().is_none_or(|b| p.updated_at >= b.3)
                {
                    best = Some((i, p.page, p.read, p.updated_at, p.unit_name.clone()));
                }
            }
            match best {
                None => comics.first().map(|f| (f.clone(), 0, false, None)),
                Some((i, page, false, _, unit)) => Some((comics[i].clone(), page, true, unit)),
                Some((i, _, true, _, _)) => comics.get(i + 1).map(|f| (f.clone(), 0, false, None)),
            }
        };
        let read_label = |f: &FileEntry, page: usize, resume: bool, unit: &Option<String>| {
            if resume && let Some(u) = unit {
                return format!("Continuar {}", label_text(&chapter_label(u)));
            }
            let l = label_text(&chapter_label(&f.name));
            if resume { format!("Continuar {l} (pág. {})", page + 1) } else { format!("Ler {l}") }
        };

        // --- Downloads ---
        let episode_paths = files.as_ref().map(|f| file_paths(&files_path, f)).unwrap_or_default();
        let remaining: Vec<String> = episode_paths
            .iter()
            .filter(|p| !matches!(st.download(p).map(|d| d.status), Some(DlStatus::Done | DlStatus::Running | DlStatus::Queued)))
            .cloned()
            .collect();

        // --- Abas ---
        let mut tabs = vec![(Tab::Files, if self.media_type == MediaType::Manga { "Arquivos" } else { "Episódios" }, (!episode_paths.is_empty()).then_some(episode_paths.len()))];
        if let Some(d) = &full {
            if !d.characters.edges.is_empty() {
                tabs.push((Tab::Cast, "Personagens", Some(d.characters.edges.len())));
            }
            if !d.staff.edges.is_empty() {
                let n = d.staff.edges.iter().map(|e| e.node.id).collect::<HashSet<_>>().len();
                tabs.push((Tab::Staff, "Equipe", Some(n)));
            }
            if !d.relations.edges.is_empty() {
                tabs.push((Tab::Related, "Relacionadas", Some(d.relations.edges.len())));
            }
            tabs.push((Tab::Details, "Detalhes", None));
        }
        let active_tab = if tabs.iter().any(|t| t.0 == self.tab) { self.tab } else { Tab::Files };

        let meta: Vec<String> = media
            .as_ref()
            .map(|m| {
                [
                    m.format.as_deref().map(|f| text::format(Some(f))),
                    m.year().map(|y| y.to_string()),
                    m.episodes.map(|e| format!("{e} {}", if e == 1 { "episódio" } else { "episódios" })),
                    m.chapters.map(|c| format!("{c} capítulos")),
                    full.as_ref().and_then(|d| d.studios.nodes.first().map(|s| s.name.clone())),
                ]
                .into_iter()
                .flatten()
                .filter(|s| !s.is_empty())
                .collect()
            })
            .unwrap_or_default();

        let banner = media.as_ref().and_then(|m| m.banner_image.clone().or_else(|| m.cover().map(str::to_owned)));
        let has_banner = media.as_ref().is_some_and(|m| m.banner_image.is_some());
        let not_found = matches!(self.summary(cx), Some(None)) && self.title_listing.is_some();

        // --- Cabeçalho: botões ---
        let mut actions = div().mt_6().flex().flex_wrap().items_center().gap_2();
        if let Some((f, at)) = resume.clone() {
            let label = resume_label(&f, at);
            actions = actions.child(
                big_button("play-resume", Some("play"), label, true, accent).on_click(cx.listener(move |v, _, _, cx| v.play(f.clone(), cx))),
            );
        }
        if let Some((f, page, res, unit)) = resume_read.clone() {
            let label = read_label(&f, page, res, &unit);
            actions = actions.child(
                big_button("read-resume", Some("book-open"), label, true, accent).on_click(cx.listener(move |v, _, _, cx| v.read(f.clone(), cx))),
            );
        }
        let dl_label = if !remaining.is_empty() {
            format!("Baixar {} ({})", if remaining.len() == episode_paths.len() { "todos" } else { "restantes" }, remaining.len())
        } else if !episode_paths.is_empty() {
            "Tudo na fila ou baixado".into()
        } else {
            "Nada para baixar aqui".into()
        };
        let primary_dl = videos.is_empty() && comics.is_empty();
        let st_entity = self.state.clone();
        let rem = remaining.clone();
        actions = actions.child(
            big_button("download-all", Some("download"), dl_label, primary_dl, accent)
                .when(remaining.is_empty(), |d| d.opacity(0.5))
                .on_click(move |_, _, cx| {
                    if !rem.is_empty() {
                        st_entity.update(cx, |s, cx| s.start_downloads(&rem, cx));
                    }
                }),
        );
        if let Some(t) = full.as_ref().and_then(|d| d.trailer.clone()).filter(|t| t.site.as_deref() == Some("youtube")) {
            let url = format!("https://www.youtube.com/watch?v={}", t.id);
            actions = actions.child(ghost_button("trailer", "play", "Trailer").on_click(move |_, _, cx| cx.open_url(&url)));
        }
        if let Some(url) = full.as_ref().and_then(|d| d.site_url.clone()) {
            actions = actions.child(ghost_button("anilist", "external-link", "AniList").on_click(move |_, _, cx| cx.open_url(&url)));
        }

        // --- Sinopse ---
        let synopsis = full.as_ref().map(|d| text::clean_description(d.description.as_deref())).filter(|(p, _)| !p.is_empty()).map(|(paras, source)| {
            let long = paras.join(" ").len() > 300;
            let expanded = self.expanded;
            div()
                .mt_5()
                .max_w(px(680.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .text_color(theme::white_a(0.8))
                        .line_height(relative(1.6))
                        .when(long && !expanded, |d| d.line_clamp(3))
                        .children(if long && !expanded { vec![paras.join(" ")] } else { paras.clone() }),
                )
                .when(long || source.is_some(), |d| {
                    d.child(
                        div()
                            .mt_1p5()
                            .flex()
                            .items_center()
                            .gap_3()
                            .text_sm()
                            .when(long, |d| {
                                d.child(
                                    div()
                                        .id("more")
                                        .cursor_pointer()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme::accent_ink())
                                        .child(if expanded { "Mostrar menos" } else { "Ler mais" })
                                        .on_click(cx.listener(|v, _, _, cx| {
                                            v.expanded = !v.expanded;
                                            cx.notify();
                                        })),
                                )
                            })
                            .when_some(source.filter(|_| expanded || !long), |d, s| d.child(div().text_xs().text_color(theme::faint()).child(format!("Fonte: {s}")))),
                    )
                })
        });

        // --- Aba de arquivos ---
        let files_tab = {
            let seasons = self.seasons();
            let releases = self.releases();
            let season_name = self.season_name();
            let release_name = self.release_name();
            let single_tags = if releases.len() == 1 { release_tags(&releases[0].name) } else { Vec::new() };
            let mut col = div().flex().flex_col().gap_5();
            let mut chips = div().flex().flex_col().gap_3();
            if seasons.len() > 1 {
                chips = chips.child(chip_group("Temporada").child(div().flex().flex_wrap().gap_2().children(seasons.iter().enumerate().map(|(i, s)| {
                    let target = join_path(&self.title_path, &s.name);
                    let st = self.state.clone();
                    chip(SharedString::from(format!("season-{i}")), Some(&s.name) == season_name.as_ref())
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(folder_year(&s.name).map(|y| y.to_string()).unwrap_or_default()))
                        .child(truncate(folder_title(&s.name)).max_w(px(224.)))
                        .on_click(move |_, _, cx| st.update(cx, |s, cx| s.go(target.clone(), cx)))
                }))));
            }
            if releases.len() > 1 {
                let season_path = self.season_path();
                chips = chips.child(chip_group("Versão").child(div().flex().flex_wrap().gap_2().children(releases.iter().enumerate().map(|(i, r)| {
                    let tags = release_tags(&r.name);
                    let target = join_path(&season_path, &r.name);
                    let st = self.state.clone();
                    chip(SharedString::from(format!("release-{i}")), Some(&r.name) == release_name.as_ref())
                        .child(if tags.is_empty() { folder_title(&r.name) } else { tags.join(" ") })
                        .on_click(move |_, _, cx| st.update(cx, |s, cx| s.go(target.clone(), cx)))
                }))));
            }
            if !single_tags.is_empty() {
                chips = chips.child(chip_group("Versão").child(div().pt_1p5().text_sm().text_color(theme::muted()).child(single_tags.join(" "))));
            }
            col = col.child(chips);
            match (&self.files_listing, files.clone()) {
                (Some(Err(e)), _) => col = col.child(div().text_color(theme::danger()).child(e.clone())),
                (_, None) => col = col.child(skeleton().h(px(320.)).rounded_2xl()),
                (_, Some(list)) => {
                    if looks_like_album(&list) && !self.not_album {
                        if self.album.is_none() {
                            let (p, f, w) = (files_path.clone(), list.clone(), Some(heading.clone()));
                            let view = cx.new(|cx| AlbumView::new(p, f, w, cx));
                            cx.subscribe(&view, |v, _, e: &AlbumEvent, cx| {
                                if matches!(e, AlbumEvent::NotAlbum) {
                                    v.not_album = true;
                                    cx.notify();
                                }
                            })
                            .detach();
                            self.album = Some(view);
                        }
                        col = col.child(self.album.clone().unwrap());
                    } else {
                        let this = cx.entity().downgrade();
                        let on_play: FileAction = Rc::new({
                            let this = this.clone();
                            move |f, _, cx| {
                                let _ = this.update(cx, |v, cx| v.play(f.clone(), cx));
                            }
                        });
                        let on_read: FileAction = Rc::new(move |f, _, cx| {
                            let _ = this.update(cx, |v, cx| v.read(f.clone(), cx));
                        });
                        col = col.child(file_list(&files_path, &list, Some(on_play), Some(on_read), cx));
                    }
                }
            }
            col
        };

        let panel: AnyElement = match (active_tab, &full) {
            (Tab::Cast, Some(d)) => cast(d, width, cx).into_any_element(),
            (Tab::Staff, Some(d)) => staff(d, width, cx).into_any_element(),
            (Tab::Related, Some(d)) => related(d, width, cx).into_any_element(),
            (Tab::Details, Some(d)) => facts(d).into_any_element(),
            _ => files_tab.into_any_element(),
        };

        let native = media.as_ref().and_then(|m| m.title.native.clone()).filter(|n| *n != heading);
        let english = media.as_ref().and_then(|m| m.title.english.clone()).filter(|n| *n != heading);
        let genres = media.as_ref().map(|m| m.genres.iter().map(|g| text::genre(g)).collect::<Vec<_>>().join(", ")).filter(|g| !g.is_empty());
        let score = media.as_ref().and_then(|m| m.average_score);

        div()
            .relative()
            .min_h_full()
            // Fundo: banner esmaecido.
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(380.))
                    .overflow_hidden()
                    .children(banner.map(|b| div().absolute().inset_0().opacity(if has_banner { 0.45 } else { 0.25 }).child(remote_image(Some(&b), cx))))
                    .child(div().absolute().inset_0().bg(gpui::linear_gradient(
                        180.,
                        gpui::linear_color_stop(gpui::Rgba { a: 0.3, ..theme::canvas() }, 0.),
                        gpui::linear_color_stop(theme::canvas(), 1.),
                    ))),
            )
            .child(
                div()
                    .relative()
                    .w(px(width))
                    .mx_auto()
                    .pt_5()
                    .pb_16()
                    .child(breadcrumbs(&self.title_path, cx))
                    .child(
                        div()
                            .mt_20()
                            .flex()
                            .items_start()
                            .gap_9()
                            .child(div().flex_none().shadow_lg().child(cover(media.as_ref(), &self.lookup_name(), Some(px(184.)), cx)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .pt_2()
                                    .children(native.map(|n| div().mb_1().text_sm().text_color(theme::muted()).child(n)))
                                    .child(div().text_size(px(44.)).line_height(relative(1.05)).font_weight(FontWeight::BOLD).child(heading.clone()))
                                    .children(english.map(|n| div().mt_1p5().text_color(theme::muted()).child(n)))
                                    .when(media.is_some(), |d| {
                                        let mut row = div().mt_4().flex().flex_wrap().items_center().gap_x_3().gap_y_1().text_sm().text_color(theme::muted());
                                        if let Some(s) = score {
                                            row = row.child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(theme::fg())
                                                    .child(icon("star").size_3p5().text_color(gpui::rgb(0xfcd34d)))
                                                    .child(format!("{s}%")),
                                            );
                                        }
                                        for (i, m) in meta.iter().enumerate() {
                                            if i > 0 || score.is_some() {
                                                row = row.child(div().size_1().rounded_full().bg(theme::faint()));
                                            }
                                            row = row.child(m.clone());
                                        }
                                        d.child(row)
                                    })
                                    .children(genres.map(|g| div().mt_2().text_sm().text_color(theme::accent_ink()).child(g)))
                                    .children(synopsis)
                                    .when(not_found, |d| {
                                        d.child(
                                            div()
                                                .mt_5()
                                                .max_w(px(600.))
                                                .text_sm()
                                                .text_color(theme::muted())
                                                .child("Não achamos este título no AniList. Os arquivos continuam disponíveis abaixo."),
                                        )
                                    })
                                    .child(actions),
                            ),
                    )
                    // Abas
                    .child(div().mt_12().flex().gap_1().border_b_1().border_color(theme::line()).children(tabs.into_iter().map(|(t, label, count)| {
                        let active = t == active_tab;
                        div()
                            .id(SharedString::from(format!("tab-{label}")))
                            .relative()
                            .flex()
                            .items_center()
                            .gap_2()
                            .h_12()
                            .px_3()
                            .cursor_pointer()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if active { theme::fg() } else { theme::muted() })
                            .hover(|d| d.text_color(theme::fg()))
                            .child(label)
                            .children(count.map(|c| div().text_xs().text_color(if active { theme::accent_ink() } else { theme::faint() }).child(c.to_string())))
                            .when(active, |d| d.child(div().absolute().bottom(px(-1.)).left_3().right_3().h(px(2.)).rounded_full().bg(accent)))
                            .on_click(cx.listener(move |v, _, _, cx| {
                                v.tab = t;
                                cx.notify();
                            }))
                    })))
                    .child(div().pt_8().child(panel)),
            )
    }
}

fn big_button(id: &'static str, icon_name: Option<&'static str>, label: String, primary: bool, accent: gpui::Rgba) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .h_10()
        .px_4()
        .rounded_xl()
        .cursor_pointer()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .map(|d| {
            if primary {
                d.bg(accent).text_color(theme::white()).hover(|s| s.opacity(0.9))
            } else {
                d.border_1().border_color(theme::line_strong()).bg(theme::white_a(0.05)).hover(|s| s.bg(theme::white_a(0.1)))
            }
        })
        .children(icon_name.map(|n| icon(n).size_4().text_color(theme::white())))
        .child(label)
}

fn ghost_button(id: &'static str, icon_name: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .h_10()
        .px_4()
        .rounded_xl()
        .cursor_pointer()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::muted())
        .hover(|d| d.bg(theme::white_a(0.08)).text_color(theme::fg()))
        .child(icon(icon_name).size_4().text_color(theme::muted()))
        .child(label)
}

fn is_placeholder(url: Option<&str>) -> bool {
    url.is_none_or(|u| u.ends_with("/default.jpg") || u.ends_with("/default.png"))
}

fn initials(name: Option<&str>) -> String {
    name.unwrap_or("?").split_whitespace().take(2).filter_map(|p| p.chars().next()).flat_map(char::to_uppercase).collect()
}

/// Foto de pessoa/personagem; monograma quando não há imagem.
fn portrait(url: Option<&str>, name: Option<&str>, cx: &mut gpui::App) -> gpui::Div {
    let base = div().size_12().flex_none().rounded_full().overflow_hidden();
    if is_placeholder(url) {
        base.flex().items_center().justify_center().bg(theme::accent_soft()).text_sm().font_weight(FontWeight::SEMIBOLD).text_color(theme::accent_ink()).child(initials(name))
    } else {
        base.child(remote_image(url, cx))
    }
}

fn cast(d: &MediaDetail, width: f32, cx: &mut gpui::App) -> gpui::Div {
    let cols = columns(width, 320., 32.);
    div().grid().grid_cols(cols as u16).gap_x_8().children(d.characters.edges.iter().map(|e| {
        let va = e.voice_actors.first();
        div()
            .flex()
            .items_center()
            .gap_3()
            .py_3()
            .border_b_1()
            .border_color(theme::line())
            .child(portrait(e.node.image.as_ref().and_then(|i| i.large.as_deref()), e.node.name.full.as_deref(), cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(truncate(e.node.name.full.clone().unwrap_or_default()).text_sm().font_weight(FontWeight::SEMIBOLD))
                    .child(div().text_xs().text_color(theme::faint()).child(text::character_role(Some(&e.role)))),
            )
            .children(va.map(|va| {
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .items_end()
                            .child(truncate(va.name.full.clone().unwrap_or_default()).text_sm())
                            .child(div().text_xs().text_color(theme::faint()).child(va.name.native.clone().unwrap_or_default())),
                    )
                    .child(portrait(va.image.as_ref().and_then(|i| i.large.as_deref()), va.name.full.as_deref(), cx))
            }))
    }))
}

fn staff(d: &MediaDetail, width: f32, cx: &mut gpui::App) -> gpui::Div {
    // Agrupa papéis da mesma pessoa: "Direção, Criação original".
    let mut people: Vec<(&higanbana_core::anilist::types::Person, Vec<String>)> = Vec::new();
    for e in &d.staff.edges {
        match people.iter_mut().find(|(p, _)| p.id == e.node.id) {
            Some((_, roles)) => roles.push(text::staff_role(&e.role)),
            None => people.push((&e.node, vec![text::staff_role(&e.role)])),
        }
    }
    let cols = columns(width, 260., 32.);
    div().grid().grid_cols(cols as u16).gap_x_8().children(people.into_iter().map(|(p, roles)| {
        div()
            .flex()
            .items_center()
            .gap_3()
            .py_3()
            .border_b_1()
            .border_color(theme::line())
            .child(portrait(p.image.as_ref().and_then(|i| i.large.as_deref()), p.name.full.as_deref(), cx))
            .child(
                div()
                    .min_w_0()
                    .child(truncate(p.name.full.clone().unwrap_or_default()).text_sm().font_weight(FontWeight::SEMIBOLD))
                    .child(div().text_xs().text_color(theme::muted()).line_clamp(2).child(roles.join(", "))),
            )
    }))
}

fn related(d: &MediaDetail, width: f32, cx: &mut gpui::App) -> gpui::Div {
    let cols = columns(width, 132., 20.);
    let card_w = px(((width - (cols as f32 - 1.) * 20.) / cols as f32).floor());
    div().child(div().mb_5().text_sm().text_color(theme::faint()).child("Abrem no AniList.")).child(
        div().grid().grid_cols(cols as u16).gap_x_5().gap_y_7().children(d.relations.edges.iter().enumerate().map(|(i, e)| {
            let n = &e.node;
            let kind = n.media_type.map_or("anime", |t| if t == MediaType::Manga { "manga" } else { "anime" });
            let url = format!("https://anilist.co/{kind}/{}", n.id);
            let bg = n.cover_image.as_ref().and_then(|c| hex_color(c.color.as_deref())).unwrap_or(theme::surface());
            div()
                .id(SharedString::from(format!("rel-{i}")))
                .w(card_w)
                .cursor_pointer()
                .child(div().mb_2().w(card_w).h(card_w * 1.5).rounded_xl().overflow_hidden().bg(bg).child(remote_image(n.cover_image.as_ref().and_then(|c| c.large.as_deref()), cx)))
                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme::accent_ink()).child(text::relation(Some(&e.relation_type))))
                .child(div().text_sm().font_weight(FontWeight::MEDIUM).line_clamp(2).child(n.title.user_preferred.clone().unwrap_or_default()))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::faint())
                        .child(format!("{} {}", text::format(n.format.as_deref()), n.start_date.and_then(|d| d.year).map(|y| y.to_string()).unwrap_or_default())),
                )
                .on_click(move |_, _, cx| cx.open_url(&url))
        })),
    )
}

type FactGroup<'a> = (&'a str, Vec<(&'a str, Option<String>)>);

fn facts(d: &MediaDetail) -> gpui::Div {
    let m = &d.summary;
    let groups: Vec<FactGroup> = vec![
        (
            "Obra",
            vec![
                ("Formato", m.format.as_deref().map(|f| text::format(Some(f)))),
                ("Origem", d.source.as_deref().map(|s| text::source(Some(s)))),
                ("Estúdio", Some(d.studios.nodes.iter().map(|s| s.name.clone()).collect::<Vec<_>>().join(", ")).filter(|s| !s.is_empty())),
                ("Episódios", m.episodes.map(|e| e.to_string())),
                ("Duração", d.duration.map(|x| format!("{x} min por episódio"))),
                ("Capítulos", m.chapters.map(|c| c.to_string())),
                ("Volumes", d.volumes.map(|v| v.to_string())),
            ],
        ),
        (
            "Exibição",
            vec![
                ("Situação", m.status.as_deref().map(|s| text::status(Some(s)))),
                ("Temporada", d.season.as_deref().zip(m.season_year).map(|(s, y)| format!("{} de {y}", text::season(Some(s))))),
                ("Estreia", text::date(m.start_date)),
                ("Fim", text::date(d.end_date)),
            ],
        ),
        (
            "AniList",
            vec![
                ("Nota média", m.average_score.map(|s| format!("{s}%"))),
                ("Popularidade", m.popularity.map(|p| format!("{} pessoas", text::number(p)))),
                ("Outros nomes", Some(m.synonyms.iter().take(4).cloned().collect::<Vec<_>>().join("; ")).filter(|s| !s.is_empty())),
            ],
        ),
    ];
    let tags: Vec<&str> = d.tags.iter().filter(|t| !t.spoiler && t.rank >= 60).take(12).map(|t| t.name.as_str()).collect();
    div()
        .flex()
        .flex_col()
        .gap_10()
        .child(div().grid().grid_cols(3).gap_10().children(groups.into_iter().filter_map(|(g, rows)| {
            let rows: Vec<(&str, String)> = rows.into_iter().filter_map(|(k, v)| v.map(|v| (k, v))).collect();
            (!rows.is_empty()).then(|| {
                div().child(div().mb_3().text_sm().font_weight(FontWeight::SEMIBOLD).child(g)).children(rows.into_iter().map(|(k, v)| {
                    div()
                        .flex()
                        .justify_between()
                        .gap_6()
                        .py_2p5()
                        .border_b_1()
                        .border_color(theme::line())
                        .text_sm()
                        .child(div().flex_none().text_color(theme::faint()).child(k))
                        .child(div().text_right().child(v))
                }))
            })
        })))
        .when(!tags.is_empty(), |d| {
            d.child(
                div().child(div().mb_3().text_sm().font_weight(FontWeight::SEMIBOLD).child("Temas")).child(
                    div().flex().flex_wrap().gap_1p5().children(
                        tags.into_iter().map(|t| div().px_3().py_1().rounded_full().border_1().border_color(theme::line()).text_xs().text_color(theme::muted()).child(t.to_owned())),
                    ),
                ),
            )
        })
}
