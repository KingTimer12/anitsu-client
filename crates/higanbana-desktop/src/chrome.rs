//! Moldura da janela: barra de título própria, busca, downloads, barra lateral e avisos.

use std::time::Duration;

use gpui::{
    Context, Entity, FocusHandle, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Task, Window,
    WindowControlArea, deferred, div, prelude::*, px, relative,
};
use higanbana_core::{
    library::{media_for_kind, split_path},
    nuvem::{SearchKind, SearchResult},
};

use crate::{
    components::{remote_image, section_label, section_meta, section_order, want_lookup, when_visible},
    input::{InputEvent, TextInput},
    state::{AppState, DlStatus, Tone, state},
    theme,
    ui::{button, icon, icon_button, spinner, truncate},
};

// --- Botões de janela (padrão do Windows 11) ---

pub fn window_controls(window: &Window) -> impl IntoElement {
    let base = |id: &'static str, name: &'static str, area: WindowControlArea| {
        div()
            .id(id)
            .h_full()
            .w_12()
            .flex()
            .items_center()
            .justify_center()
            .window_control_area(area)
            .child(crate::ui::icon(name).size_4().text_color(theme::muted()))
    };
    div()
        .flex()
        .h_full()
        .child(base("min", "minus", WindowControlArea::Min).hover(|d| d.bg(theme::white_a(0.08))))
        .child(base("max", if window.is_maximized() { "copy" } else { "square" }, WindowControlArea::Max).hover(|d| d.bg(theme::white_a(0.08))))
        .child(base("close", "x", WindowControlArea::Close).hover(|d| d.bg(gpui::rgb(0xc42b1c))))
}

// --- Busca ---

const DEBOUNCE: Duration = Duration::from_millis(250);

pub struct SearchBox {
    state: Entity<AppState>,
    input: Entity<TextInput>,
    query: String,
    results: Option<Vec<SearchResult>>,
    loading: bool,
    open: bool,
    active: usize,
    debounce: Option<Task<()>>,
}

impl SearchBox {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let input = cx.new(|cx| TextInput::new("Buscar animes, mangás, OSTs...", cx));
        cx.subscribe(&input, |v: &mut Self, _, e: &InputEvent, cx| v.on_input(e, cx)).detach();
        Self { state, input, query: String::new(), results: None, loading: false, open: false, active: 0, debounce: None }
    }

    pub fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        use gpui::Focusable;
        self.input.read(cx).focus_handle(cx)
    }

    /// Ctrl+K: foca e seleciona o texto.
    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle(cx));
        self.input.update(cx, |i, cx| i.select_everything(cx));
        if !self.query.is_empty() {
            self.open = true;
        }
        cx.notify();
    }

    fn on_input(&mut self, e: &InputEvent, cx: &mut Context<Self>) {
        match e {
            InputEvent::Changed => {
                let text = self.input.read(cx).text().trim().to_owned();
                self.open = true;
                self.debounce = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(DEBOUNCE).await;
                    let _ = this.update(cx, |v, cx| v.search(text, cx));
                }));
            }
            InputEvent::Down => {
                let n = self.results.as_ref().map_or(0, Vec::len);
                self.active = (self.active + 1).min(n.saturating_sub(1));
            }
            InputEvent::Up => self.active = self.active.saturating_sub(1),
            InputEvent::Submit => {
                if let Some(r) = self.results.as_ref().and_then(|r| r.get(self.active)).cloned() {
                    self.choose(r, cx);
                }
            }
            InputEvent::Escape => self.open = false,
        }
        cx.notify();
    }

    fn search(&mut self, q: String, cx: &mut Context<Self>) {
        self.query = q.clone();
        self.active = 0;
        if q.is_empty() {
            self.results = Some(Vec::new());
            return;
        }
        self.loading = true;
        let core = self.state.read(cx).core.clone();
        let task = core.clone().spawn(async move { core.cloud().search(&q).await });
        cx.spawn(async move |this, cx| {
            let r = task.await.ok().and_then(Result::ok);
            let _ = this.update(cx, |v, cx| {
                v.loading = false;
                v.results = Some(r.map(|r| r.results).unwrap_or_default());
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn choose(&mut self, r: SearchResult, cx: &mut Context<Self>) {
        self.open = false;
        self.state.update(cx, |s, cx| s.go(r.path, cx));
    }
}

fn kind_label(k: SearchKind) -> (&'static str, gpui::Rgba) {
    match k {
        SearchKind::Anime => ("Anime", gpui::rgb(0xfda4af)),
        SearchKind::Manga => ("Mangá", gpui::rgb(0x7dd3fc)),
        SearchKind::Dorama => ("Dorama", gpui::rgb(0xc4b5fd)),
        SearchKind::Ost => ("OST", gpui::rgb(0x6ee7b7)),
        SearchKind::Desenho => ("Desenho", gpui::rgb(0xfcd34d)),
        SearchKind::Folder => ("Pasta", theme::muted()),
        SearchKind::Other => ("Outro", theme::muted()),
    }
}

impl Render for SearchBox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle(cx).is_focused(window);
        let field = div()
            .flex()
            .items_center()
            .gap_2()
            .h_8()
            .px_2p5()
            .rounded_lg()
            .border_1()
            .border_color(if focused { gpui::Rgba { a: 0.6, ..theme::accent() } } else { theme::line() })
            .bg(theme::white_a(if focused { 0.08 } else { 0.05 }))
            .child(if self.loading {
                div().size_4().text_color(theme::faint()).child(spinner("search-spin")).into_any_element()
            } else {
                icon("search").size_4().text_color(theme::faint()).into_any_element()
            })
            .child(div().flex_1().min_w_0().child(self.input.clone()))
            .child(div().px_1p5().rounded_sm().border_1().border_color(theme::line()).text_xs().text_color(theme::faint()).child("Ctrl K"));

        let popup = (self.open && !self.query.is_empty() && focused).then(|| {
            let body = match &self.results {
                None => div().p_4().text_sm().text_color(theme::muted()).child("Buscando...").into_any_element(),
                Some(list) if list.is_empty() => div()
                    .p_4()
                    .text_sm()
                    .text_color(theme::muted())
                    .child(format!("Nada encontrado para \"{}\". Tente o título em romaji, como \"Shingeki no Kyojin\".", self.query))
                    .into_any_element(),
                Some(list) => div()
                    .children(list.clone().into_iter().enumerate().map(|(i, r)| {
                        let media = media_for_kind(r.kind);
                        let thumb = match media.filter(|_| i < 8) {
                            Some(m) => {
                                let looked = self.state.read(cx).lookup(&r.name, m);
                                let url = looked.clone().flatten().and_then(|m| m.cover_small().map(str::to_owned));
                                div()
                                    .relative()
                                    .child(match url {
                                        Some(u) => remote_image(Some(&u), cx),
                                        None => icon("folder-open").size_4().text_color(theme::faint()).into_any_element(),
                                    })
                                    .when(looked.is_none(), |d| d.child(when_visible(0., want_lookup(r.name.clone(), m))))
                                    .into_any_element()
                            }
                            None => icon("folder-open").size_4().text_color(theme::faint()).into_any_element(),
                        };
                        let (label, color) = kind_label(r.kind);
                        let parent = split_path(&r.path).iter().rev().skip(1).rev().copied().collect::<Vec<_>>().join(" / ");
                        let active = i == self.active;
                        div()
                            .id(SharedString::from(format!("result-{i}")))
                            .flex()
                            .items_center()
                            .gap_3()
                            .p_2()
                            .rounded_xl()
                            .cursor_pointer()
                            .when(active, |d| d.bg(theme::white_a(0.08)))
                            .on_hover(cx.listener(move |v, h: &bool, _, cx| {
                                if *h {
                                    v.active = i;
                                    cx.notify();
                                }
                            }))
                            .child(
                                div().w_9().h_12().flex_none().rounded_md().overflow_hidden().bg(theme::elevated()).flex().items_center().justify_center().child(thumb),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(truncate(r.name.clone()).font_weight(FontWeight::MEDIUM))
                                    .child(truncate(parent).text_xs().text_color(theme::faint())),
                            )
                            .child(div().flex_none().px_2().py_0p5().rounded_full().bg(gpui::Rgba { a: 0.15, ..color }).text_color(color).text_xs().font_weight(FontWeight::SEMIBOLD).child(label))
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |v, _, _, cx| {
                                cx.stop_propagation();
                                v.choose(r.clone(), cx);
                            }))
                    }))
                    .into_any_element(),
            };
            deferred(
                div()
                    .id("search-results")
                    .occlude()
                    .absolute()
                    .top(px(40.))
                    .left(px(-40.))
                    .w(px(600.))
                    .max_h(px(520.))
                    .overflow_y_scroll()
                    .p_1p5()
                    .rounded_2xl()
                    .border_1()
                    .border_color(theme::line_strong())
                    .bg(gpui::Rgba { a: 0.97, ..theme::elevated() })
                    .shadow_lg()
                    .child(body),
            )
            .with_priority(1)
        });

        div().relative().w_full().max_w(px(520.)).child(field).children(popup)
    }
}

// --- Downloads ---

pub struct DownloadsMenu {
    state: Entity<AppState>,
    open: bool,
}

impl DownloadsMenu {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self { state, open: false }
    }
}

impl Render for DownloadsMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let pending = st.pending_downloads();
        let overall = st.overall_progress();
        let items = st.downloads.clone();
        let any_finished = items.iter().any(|d| matches!(d.status, DlStatus::Done | DlStatus::Error));

        let trigger = div()
            .id("downloads")
            .relative()
            .size_9()
            .flex()
            .items_center()
            .justify_center()
            .rounded_lg()
            .cursor_pointer()
            .hover(|d| d.bg(theme::white_a(0.08)))
            .when(self.open, |d| d.bg(theme::white_a(0.08)))
            .child(icon("download").size(px(18.)).text_color(if pending > 0 { theme::fg() } else { theme::muted() }))
            .when(pending > 0, |d| {
                d.child(div().absolute().bottom(px(4.)).left(px(8.)).right(px(8.)).h(px(2.)).rounded_full().bg(theme::white_a(0.15)).child(div().h_full().rounded_full().bg(theme::brand()).w(relative(overall))))
                    .child(
                        div()
                            .absolute()
                            .top(px(-2.))
                            .right(px(-2.))
                            .h_4()
                            .min_w_4()
                            .px_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(theme::brand())
                            .text_size(px(10.))
                            .font_weight(FontWeight::BOLD)
                            .child(pending.to_string()),
                    )
            })
            .on_click(cx.listener(|v, _, _, cx| {
                v.open = !v.open;
                cx.notify();
            }));

        let panel = self.open.then(|| {
            let st = self.state.clone();
            let list = if items.is_empty() {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .px_6()
                    .py_10()
                    .child(icon("download").size_8().text_color(theme::faint()))
                    .child(div().text_sm().text_color(theme::muted()).child("Nenhum download ainda."))
                    .child(div().text_xs().text_color(theme::faint()).child("Abra um anime e escolha os episódios para baixar."))
            } else {
                div().p_1p5().children(items.into_iter().enumerate().map(|(i, d)| {
                    let (ic, color) = match d.status {
                        DlStatus::Done => ("check", theme::ok()),
                        DlStatus::Error => ("triangle-alert", theme::danger()),
                        _ => ("download", theme::muted()),
                    };
                    let status = match d.status {
                        DlStatus::Queued => "Na fila".to_owned(),
                        DlStatus::Running => match d.total {
                            Some(t) => format!("{} de {}", crate::components::format_bytes(d.downloaded), crate::components::format_bytes(t)),
                            None => crate::components::format_bytes(d.downloaded),
                        },
                        DlStatus::Done => "Concluído".into(),
                        DlStatus::Error => d.error.clone().unwrap_or_default(),
                    };
                    let action = match d.status {
                        DlStatus::Done => d.dest.clone().map(|dest| {
                            icon_button(SharedString::from(format!("reveal-{i}")), "folder-open").size_8().on_click(move |_, _, cx| cx.reveal_path(&dest)).into_any_element()
                        }),
                        DlStatus::Error => {
                            let (st, path) = (st.clone(), d.path.clone());
                            Some(
                                icon_button(SharedString::from(format!("retry-{i}")), "refresh-cw")
                                    .size_8()
                                    .on_click(move |_, _, cx| st.update(cx, |s, cx| s.start_downloads(std::slice::from_ref(&path), cx)))
                                    .into_any_element(),
                            )
                        }
                        _ => None,
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_3()
                        .py_2p5()
                        .rounded_xl()
                        .child(div().size_8().flex_none().rounded_lg().bg(theme::white_a(0.06)).flex().items_center().justify_center().child(icon(ic).size_4().text_color(color)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(truncate(d.name.clone()).text_sm().font_weight(FontWeight::MEDIUM))
                                .when(matches!(d.status, DlStatus::Running | DlStatus::Queued), |x| {
                                    x.child(div().mt_1p5().h_1().rounded_full().bg(theme::white_a(0.1)).child(div().h_full().rounded_full().bg(theme::brand()).w(relative(d.progress()))))
                                })
                                .child(div().mt_1().text_xs().text_color(if d.status == DlStatus::Error { theme::danger() } else { theme::faint() }).child(status)),
                        )
                        .children(action)
                }))
            };
            deferred(
                div()
                    .id("downloads-panel")
                    .occlude()
                    .absolute()
                    .top(px(44.))
                    .right_0()
                    .w(px(384.))
                    .rounded_2xl()
                    .border_1()
                    .border_color(theme::line_strong())
                    .bg(gpui::Rgba { a: 0.97, ..theme::elevated() })
                    .shadow_lg()
                    .on_mouse_down_out(cx.listener(|v, _, _, cx| {
                        v.open = false;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(theme::line())
                            .child(div().font_weight(FontWeight::SEMIBOLD).child("Downloads"))
                            .when(any_finished, |d| {
                                let st = self.state.clone();
                                d.child(button("clear-dl", "Limpar concluídos", false).h_8().text_xs().on_click(move |_, _, cx| st.update(cx, |s, cx| s.clear_finished(cx))))
                            }),
                    )
                    .child(div().id("downloads-list").max_h(px(420.)).overflow_y_scroll().child(list)),
            )
            .with_priority(1)
        });

        div().relative().child(trigger).children(panel)
    }
}

// --- Barra lateral ---

pub struct Sidebar {
    state: Entity<AppState>,
    sections: Option<Vec<String>>,
}

impl Sidebar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        crate::views::load_listing(&state, String::new(), cx, |v: &mut Self, r| {
            v.sections = r.ok().map(|f| {
                let mut s: Vec<String> = f.into_iter().filter(|f| f.is_directory).map(|f| f.name).collect();
                s.sort_by(|a, b| section_order(a, b));
                s
            });
        });
        Self { state, sections: None }
    }
}

fn nav_item(id: impl Into<gpui::ElementId>, active: bool, icon_name: &'static str, label: String) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap_3()
        .h_10()
        .w_full()
        .px_3()
        .rounded_xl()
        .cursor_pointer()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .map(|d| if active { d.bg(theme::white_a(0.08)).text_color(theme::fg()) } else { d.text_color(theme::muted()).hover(|s| s.bg(theme::white_a(0.05)).text_color(theme::fg())) })
        .when(active, |d| d.child(div().absolute().top(px(10.)).bottom(px(10.)).left_0().w(px(3.)).rounded_r_full().bg(theme::brand())))
        .child(icon(icon_name).size(px(18.)).text_color(if active { theme::brand() } else { theme::faint() }))
        .child(truncate(label))
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let current = split_path(st.path()).first().map(|s| s.to_string()).unwrap_or_default();
        let user = st.user.clone();
        let core = st.core.clone();
        let home = {
            let s = self.state.clone();
            nav_item("nav-home", current.is_empty(), "house", "Início".into()).on_click(move |_, _, cx| s.update(cx, |s, cx| s.go("", cx)))
        };
        let sections = match &self.sections {
            None => div().flex().flex_col().gap_1().children((0..5).map(|_| div().mx_1().my_1().h_8().rounded_lg().bg(theme::surface()))),
            Some(list) => div().flex().flex_col().gap_0p5().children(list.iter().enumerate().map(|(i, name)| {
                let (ic, _, _) = section_meta(name);
                let s = self.state.clone();
                let target = name.clone();
                nav_item(SharedString::from(format!("nav-{i}")), current == *name, ic, section_label(name))
                    .on_click(move |_, _, cx| s.update(cx, |s, cx| s.go(target.clone(), cx)))
            })),
        };
        let avatar = user.as_ref().and_then(|u| u.avatar.clone());
        div()
            .w(px(240.))
            .flex_none()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(theme::line())
            .bg(gpui::Rgba { a: 0.6, ..theme::bg() })
            .child(
                div()
                    .id("nav")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(home)
                    .child(div().mt_5().mb_1p5().px_3().text_xs().font_weight(FontWeight::MEDIUM).text_color(theme::faint()).child("Biblioteca"))
                    .child(sections),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_3()
                    .border_t_1()
                    .border_color(theme::line())
                    .child(div().size_9().flex_none().rounded_full().overflow_hidden().bg(theme::white_a(0.08)).child(remote_image(avatar.as_deref(), cx)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(truncate(user.map(|u| u.name).unwrap_or_default()).text_sm().font_weight(FontWeight::SEMIBOLD))
                            .child(div().text_xs().text_color(theme::faint()).child("Via Discord")),
                    )
                    .child(icon_button("logout", "log-out").size_9().on_click(move |_, _, _| {
                        let c = core.clone();
                        core.spawn(async move { c.auth().logout().await });
                    })),
            )
    }
}

// --- Avisos ---

pub fn toasts(state: &Entity<AppState>, cx: &mut gpui::App) -> impl IntoElement {
    let list = state.read(cx).toasts.clone();
    let st = state.clone();
    div()
        .absolute()
        .bottom_5()
        .left_0()
        .right_0()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .children(list.into_iter().map(move |t| {
            let (ic, color) = match t.tone {
                Tone::Ok => ("check", theme::ok()),
                Tone::Error => ("triangle-alert", theme::danger()),
                Tone::Info => ("info", theme::muted()),
            };
            let st = st.clone();
            div()
                .id(SharedString::from(format!("toast-{}", t.id)))
                .occlude()
                .flex()
                .items_center()
                .gap_3()
                .py_2p5()
                .pr_2()
                .pl_3p5()
                .rounded_xl()
                .border_1()
                .border_color(theme::line_strong())
                .bg(gpui::Rgba { a: 0.97, ..theme::elevated() })
                .shadow_lg()
                .text_sm()
                .child(icon(ic).size_4().text_color(color))
                .child(div().max_w(px(560.)).child(t.message.clone()))
                .child(icon_button(SharedString::from(format!("toast-x-{}", t.id)), "x").size_6().on_click(move |_, _, cx| st.update(cx, |s, cx| s.dismiss(t.id, cx))))
        }))
}

