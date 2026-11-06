//! Peças das telas do acervo: capa, pôster, migalhas, lista de arquivos, abas.

use gpui::{
    AnyElement, App, Bounds, Div, ElementId, FontWeight, IntoElement, ObjectFit, ParentElement, Pixels, SharedString,
    Stateful, Styled, Window, canvas, div, img, prelude::*, px, relative,
};
use higanbana_core::{
    anilist::{MediaType, text, types::MediaSummary},
    library::{folder_title, folder_year, join_path, natural_cmp, parse_episode, split_path},
    nuvem::FileEntry,
    player::logic::VIDEO_EXT,
};

use crate::{
    state::{AppState, DlStatus, Img, state},
    theme,
    ui::{icon, truncate},
};

/// Chama `f` (fora do desenho) quando o elemento entra na área visível, com folga.
pub fn when_visible(margin: f32, f: impl FnOnce(&mut App) + 'static) -> impl IntoElement {
    canvas(
        move |bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App| {
            let view = window.viewport_size();
            let top = f32::from(bounds.origin.y);
            let bottom = top + f32::from(bounds.size.height);
            if bottom >= -margin && top <= f32::from(view.height) + margin {
                cx.defer(f);
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
}

/// Imagem de uma URL pública, com um fundo enquanto carrega.
pub fn remote_image(url: Option<&str>, cx: &mut App) -> AnyElement {
    let Some(url) = url else { return div().size_full().into_any_element() };
    let s = state(cx);
    let found = s.update(cx, |s, cx| s.image(url, cx));
    match found {
        Img::Ready(image) => img(image).size_full().object_fit(ObjectFit::Cover).into_any_element(),
        _ => div().size_full().into_any_element(),
    }
}

pub fn hex_color(c: Option<&str>) -> Option<gpui::Rgba> {
    let c = c?.trim_start_matches('#');
    u32::from_str_radix(c, 16).ok().filter(|_| c.len() == 6).map(gpui::rgb)
}

/// Capa tipográfica para títulos sem correspondência no AniList.
fn type_poster(name: &str) -> Div {
    div()
        .size_full()
        .flex()
        .items_end()
        .p_3p5()
        .bg(theme::elevated())
        .child(div().text_xl().font_weight(FontWeight::BOLD).line_height(relative(1.)).child(folder_title(name)))
}

/// Capa 2:3 do AniList ou tipográfica.
pub fn cover(media: Option<&MediaSummary>, name: &str, width: Option<Pixels>, cx: &mut App) -> Div {
    let bg = media.and_then(|m| hex_color(m.color())).unwrap_or(theme::surface());
    let base = div().relative().overflow_hidden().rounded_xl().bg(bg);
    let base = match width {
        Some(w) => base.w(w).h(w * 1.5),
        None => base.w_full(),
    };
    match media.and_then(|m| m.cover()) {
        Some(url) => base.child(remote_image(Some(url), cx)),
        None => base.child(type_poster(name)),
    }
}

/// Selo de "está no acervo", sobre a capa.
pub fn nuvem_badge() -> Div {
    div()
        .absolute()
        .top_2()
        .left_2()
        .size_7()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme::accent())
        .child(icon("folder-check").size_3p5().text_color(theme::white()))
}

pub struct PosterProps {
    pub id: ElementId,
    pub name: String,
    pub media: Option<MediaSummary>,
    pub loading: bool,
    pub width: Pixels,
    pub badge: bool,
    pub dimmed: bool,
}

/// Card de pôster (capa + título + formato/ano).
pub fn poster(p: PosterProps, cx: &mut App) -> Stateful<Div> {
    let title = folder_title(&p.name);
    let media = p.media.as_ref();
    let year = media.and_then(|m| m.year()).or_else(|| folder_year(&p.name).map(|y| y as i32));
    let format = media.and_then(|m| m.format.as_deref()).map(|f| text::format(Some(f)));
    let alt = media.map(|m| m.title()).filter(|t| *t != title);
    let cover_el = if p.loading {
        div().w(p.width).h(p.width * 1.5).rounded_xl().bg(theme::surface())
    } else {
        cover(media, &p.name, Some(p.width), cx)
    };
    div()
        .id(p.id)
        .flex()
        .flex_col()
        .gap_2()
        .w(p.width)
        .cursor_pointer()
        .when(p.dimmed, |d| d.opacity(0.55))
        .child(
            div()
                .relative()
                .child(cover_el)
                .when(p.badge, |d| d.child(nuvem_badge()))
                .children(media.and_then(|m| m.average_score).map(|s| {
                    div()
                        .absolute()
                        .top_2()
                        .right_2()
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_1p5()
                        .py_0p5()
                        .rounded_md()
                        .bg(theme::black_a(0.7))
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(icon("star").size_3().text_color(gpui::rgb(0xfcd34d)))
                        .child(s.to_string())
                })),
        )
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).line_clamp(2).child(title))
        .child(
            div()
                .flex()
                .gap_2()
                .text_xs()
                .text_color(theme::faint())
                .children(format.filter(|f| !f.is_empty()))
                .children(year.map(|y| y.to_string())),
        )
        .children(alt.map(|a| truncate(a).text_xs().text_color(theme::muted())))
}

/// Colunas que cabem em `width` com itens de pelo menos `min` e espaço `gap`.
pub fn columns(width: f32, min: f32, gap: f32) -> usize {
    (((width + gap) / (min + gap)).floor() as usize).max(1)
}

/// Largura útil do conteúdo (janela − barra lateral − margens), no máximo `max`.
pub fn content_width(window: &Window, max: f32) -> f32 {
    (f32::from(window.viewport_size().width) - 240. - 64.).clamp(200., max)
}

/// Rótulo de seção na barra lateral, no início e nas migalhas.
pub fn section_label(name: &str) -> String {
    match name {
        "[#Temp]" => "Recentes".into(),
        "AniGUI (Downloader Oficial)" => "AniGUI".into(),
        n => n.into(),
    }
}

/// Ícone, descrição e cor de cada seção da raiz.
pub fn section_meta(name: &str) -> (&'static str, &'static str, gpui::Rgba) {
    match name {
        "Animes" => ("clapperboard", "Séries, filmes e OVAs, por letra.", gpui::rgb(0xfda4af)),
        "Mangás" => ("book-open", "Séries completas e volumes avulsos.", gpui::rgb(0x7dd3fc)),
        "Doramas" => ("tv", "Dramas asiáticos com legenda.", gpui::rgb(0xc4b5fd)),
        "Desenhos" => ("palette", "Animações ocidentais.", gpui::rgb(0xfcd34d)),
        "OSTs" => ("music", "Trilhas de animes, jogos e filmes.", gpui::rgb(0x6ee7b7)),
        "[#Temp]" => ("inbox", "Envios novos, antes de organizar.", gpui::rgb(0xcbd5e1)),
        _ => ("folder-open", "", theme::muted()),
    }
}

/// Ordem na barra lateral: conteúdo principal primeiro.
pub fn section_order(a: &str, b: &str) -> std::cmp::Ordering {
    const ORDER: [&str; 7] = ["Animes", "Mangás", "Doramas", "Desenhos", "OSTs", "[#Temp]", "AniGUI (Downloader Oficial)"];
    let pos = |s: &str| ORDER.iter().position(|o| *o == s).unwrap_or(99);
    pos(a).cmp(&pos(b)).then_with(|| natural_cmp(a, b))
}

/// Migalhas: Início › Seção › ... › atual.
pub fn breadcrumbs(path: &str, cx: &mut App) -> Div {
    let segs: Vec<String> = split_path(path).into_iter().map(str::to_owned).collect();
    let s = state(cx);
    let mut row = div().flex().flex_wrap().items_center().gap_1().text_sm().text_color(theme::muted()).child(
        div().id("crumb-home").px_1().rounded_md().cursor_pointer().hover(|d| d.text_color(theme::fg())).child("Início").on_click({
            let s = s.clone();
            move |_, _, cx| s.update(cx, |s, cx| s.go("", cx))
        }),
    );
    let n = segs.len();
    for (i, seg) in segs.iter().enumerate() {
        let label = if i == 0 { section_label(seg) } else { seg.clone() };
        row = row.child(icon("chevron-right").size_3p5().text_color(theme::faint()));
        if i + 1 == n {
            row = row.child(truncate(label).max_w(px(420.)).px_1().text_color(theme::fg()));
        } else {
            let target = segs[..=i].join("/");
            let s = s.clone();
            row = row.child(
                div()
                    .id(SharedString::from(format!("crumb-{i}")))
                    .max_w(px(280.))
                    .px_1()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|d| d.text_color(theme::fg()))
                    .child(truncate(label))
                    .on_click(move |_, _, cx| s.update(cx, |s, cx| s.go(target.clone(), cx))),
            );
        }
    }
    row
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024. && i < UNITS.len() - 1 {
        v /= 1024.;
        i += 1;
    }
    let num = if i >= 3 { format!("{v:.2}").trim_end_matches('0').trim_end_matches('.').to_owned() } else { format!("{v:.0}") };
    format!("{} {}", num.replace('.', ","), UNITS[i])
}

pub fn is_comic(f: &FileEntry) -> bool {
    !f.is_directory
        && f.extension.as_deref().is_some_and(|e| matches!(e.to_lowercase().as_str(), ".cbz" | ".zip" | ".cbr" | ".rar"))
}

/// Botão "Baixar" de um arquivo, com o estado do download.
pub fn download_button(id: impl Into<ElementId>, path: &str, cx: &mut App) -> AnyElement {
    let s = state(cx);
    let d = s.read(cx).download(path).cloned();
    let path = path.to_owned();
    let start = {
        let s = s.clone();
        move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| s.update(cx, |s, cx| s.start_downloads(std::slice::from_ref(&path), cx))
    };
    match d.map(|d| (d.status, d)) {
        Some((DlStatus::Queued, _)) => div().px_3().text_sm().text_color(theme::faint()).child("Na fila").into_any_element(),
        Some((DlStatus::Running, d)) => div()
            .w(px(64.))
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .text_xs()
            .text_color(theme::muted())
            .child(format!("{}%", (d.progress() * 100.).round()))
            .child(div().w_full().h_1().rounded_full().bg(theme::white_a(0.1)).child(div().h_full().rounded_full().bg(theme::brand()).w(relative(d.progress()))))
            .into_any_element(),
        Some((DlStatus::Done, _)) => div()
            .flex()
            .items_center()
            .gap_1p5()
            .px_2()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme::ok())
            .child(icon("check").size_4().text_color(theme::ok()))
            .child("Baixado")
            .into_any_element(),
        Some((DlStatus::Error, _)) => div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .h_9()
            .px_3()
            .rounded_lg()
            .cursor_pointer()
            .text_sm()
            .text_color(theme::danger())
            .child(icon("refresh-cw").size_4().text_color(theme::danger()))
            .child("Tentar de novo")
            .on_click(start)
            .into_any_element(),
        None => div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .h_9()
            .px_3()
            .rounded_lg()
            .border_1()
            .border_color(theme::line_strong())
            .cursor_pointer()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .hover(|d| d.border_color(theme::accent()).bg(theme::accent_soft()))
            .child(icon("download").size_4())
            .child("Baixar")
            .on_click(start)
            .into_any_element(),
    }
}

/// Caminhos dos arquivos (não pastas) de uma listagem.
pub fn file_paths(dir: &str, files: &[FileEntry]) -> Vec<String> {
    files.iter().filter(|f| !f.is_directory).map(|f| join_path(dir, &f.name)).collect()
}

fn accent_button(id: impl Into<ElementId>, icon_name: &'static str, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap_2()
        .h_9()
        .px_3()
        .rounded_lg()
        .bg(theme::accent())
        .cursor_pointer()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .hover(|d| d.bg(theme::accent_ink()))
        .child(icon(icon_name).size_4().text_color(theme::white()))
        .child(label.into())
}

pub type FileAction = std::rc::Rc<dyn Fn(&FileEntry, &mut Window, &mut App)>;

/// Lista de uma pasta: subpastas primeiro, depois arquivos como episódios.
pub fn file_list(dir: &str, files: &[FileEntry], on_play: Option<FileAction>, on_read: Option<FileAction>, cx: &mut App) -> Div {
    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| b.is_directory.cmp(&a.is_directory).then_with(|| natural_cmp(&a.name, &b.name)));
    if sorted.is_empty() {
        return div()
            .rounded_2xl()
            .border_1()
            .border_dashed()
            .border_color(theme::line())
            .px_6()
            .py_10()
            .flex()
            .justify_center()
            .text_sm()
            .text_color(theme::muted())
            .child("Esta pasta está vazia.");
    }
    // Quando todos os episódios têm a mesma qualidade, mostra uma vez só no topo.
    let eps: Vec<_> = sorted.iter().filter(|f| !f.is_directory).map(|f| parse_episode(&f.name)).filter(|e| e.number.is_some()).collect();
    let shared = (eps.len() >= 2)
        .then(|| eps[0].quality.clone())
        .flatten()
        .filter(|q| eps.iter().all(|e| e.quality.as_deref() == Some(q)));

    let s = state(cx);
    let (progress, reading) = {
        let st = s.read(cx);
        (st.player_data.progress.clone(), crate::reader::progress_map(st))
    };

    let mut list = div().flex().flex_col().rounded_2xl().border_1().border_color(theme::line()).bg(gpui::Rgba { a: 0.6, ..theme::surface() }).overflow_hidden();
    for (i, f) in sorted.into_iter().enumerate() {
        let path = join_path(dir, &f.name);
        let row_id = SharedString::from(format!("file-{i}"));
        let border = |d: Div| if i > 0 { d.border_t_1().border_color(theme::line()) } else { d };
        if f.is_directory {
            let s = s.clone();
            list = list.child(border(div()).child(
                div()
                    .id(row_id)
                    .flex()
                    .items_center()
                    .gap_4()
                    .px_4()
                    .py_3()
                    .cursor_pointer()
                    .hover(|d| d.bg(theme::white_a(0.04)))
                    .child(div().size_10().flex_none().rounded_lg().bg(theme::white_a(0.06)).flex().items_center().justify_center().child(icon("folder").text_color(theme::muted())))
                    .child(truncate(f.name.clone()).flex_1().font_weight(FontWeight::MEDIUM))
                    .on_click(move |_, _, cx| s.update(cx, |s, cx| s.go(path.clone(), cx))),
            ));
            continue;
        }
        let ep = parse_episode(&f.name);
        let label = match (&ep.number, &shared) {
            (None, _) => f.name.clone(),
            (Some(n), Some(_)) => format!("Episódio {}", n.parse::<f64>().map_or(n.clone(), |v| v.to_string())),
            (Some(_), None) => ep.quality.clone().unwrap_or(ep.label.clone()),
        };
        let ext = f.extension.clone().unwrap_or_default().to_lowercase();
        let playable = on_play.is_some() && VIDEO_EXT.contains(&ext.as_str());
        let readable = on_read.is_some() && is_comic(&f);
        let read_p = if readable { reading.get(&path).cloned() } else { None };
        let (watched, frac) = match (&read_p, progress.get(&path)) {
            (Some(r), _) => (r.read, (r.page as f64 + 1.) / (r.total.max(1) as f64)),
            (None, Some(p)) => (p.watched, p.position / p.duration.max(1.)),
            _ => (false, -1.),
        };
        let mut row = div()
            .flex()
            .items_center()
            .gap_4()
            .px_4()
            .py_3()
            .hover(|d| d.bg(theme::white_a(0.04)))
            .child(
                div()
                    .w_10()
                    .flex_none()
                    .flex()
                    .justify_center()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if ep.number.is_some() { theme::accent_ink() } else { theme::faint() })
                    .child(match &ep.number {
                        Some(n) => div().child(n.clone()),
                        None => div().child(icon("file-video").text_color(theme::faint())),
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(truncate(label.clone()).font_weight(FontWeight::MEDIUM))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .text_xs()
                            .text_color(theme::faint())
                            .child(ext.trim_start_matches('.').to_uppercase())
                            .child(format_bytes(f.size))
                            .when(watched, |d| {
                                d.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .text_color(theme::ok())
                                        .child(icon("check").size_3p5().text_color(theme::ok()))
                                        .child(if readable { "Lido" } else { "Visto" }),
                                )
                            }),
                    )
                    .when(frac >= 0. && !watched, |d| {
                        d.child(
                            div()
                                .mt_1p5()
                                .h(px(2.))
                                .max_w(px(192.))
                                .rounded_full()
                                .bg(theme::white_a(0.1))
                                .child(div().h_full().bg(theme::accent()).w(relative(frac.min(1.) as f32))),
                        )
                    }),
            );
        if playable {
            let (act, f2) = (on_play.clone().unwrap(), f.clone());
            row = row.child(accent_button(SharedString::from(format!("play-{i}")), "play", "Assistir").on_click(move |_, w, cx| act(&f2, w, cx)));
        }
        if readable {
            let (act, f2) = (on_read.clone().unwrap(), f.clone());
            let label = if read_p.as_ref().is_some_and(|r| !r.read) { "Continuar" } else { "Ler" };
            row = row.child(accent_button(SharedString::from(format!("read-{i}")), "book-open", label).on_click(move |_, w, cx| act(&f2, w, cx)));
        }
        row = row.child(download_button(SharedString::from(format!("dl-{i}")), &path, cx));
        list = list.child(border(div()).child(row));
    }
    let mut out = div().flex().flex_col();
    if let Some(q) = shared {
        out = out.child(div().mb_2().text_sm().text_color(theme::muted()).child(format!("Todos os episódios em {q}")));
    }
    out.child(list)
}

/// Grupo de chips com rótulo à esquerda (temporada, versão).
pub fn chip_group(label: &'static str) -> Div {
    div().flex().items_start().gap_4().child(div().w_20().flex_none().pt_1p5().text_xs().text_color(theme::faint()).child(label))
}

pub fn chip(id: impl Into<ElementId>, active: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .h_8()
        .px_3p5()
        .rounded_full()
        .border_1()
        .cursor_pointer()
        .text_sm()
        .map(|d| {
            if active {
                d.border_color(theme::accent()).bg(theme::accent_soft()).text_color(theme::fg())
            } else {
                d.border_color(theme::line()).text_color(theme::muted()).hover(|s| s.border_color(theme::line_strong()).text_color(theme::fg()))
            }
        })
}

/// Título de seção com algo à direita.
pub fn section_title(title: impl Into<SharedString>, aside: Option<AnyElement>) -> Div {
    div()
        .mb_4()
        .flex()
        .items_end()
        .justify_between()
        .gap_4()
        .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(title.into()))
        .children(aside)
}

pub fn skeleton() -> Div {
    div().rounded_xl().bg(theme::surface())
}

pub fn want_lookup(name: String, media: MediaType) -> impl FnOnce(&mut App) {
    move |cx| state(cx).update(cx, |s: &mut AppState, cx| s.want_lookup(&name, media, cx))
}
