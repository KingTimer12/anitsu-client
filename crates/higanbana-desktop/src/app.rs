//! Raiz da janela: moldura + tela atual, ou o player/leitor por cima de tudo.

use std::{rc::Rc, sync::Arc};

use futures::{StreamExt, channel::mpsc::UnboundedReceiver};
use gpui::{
    AnyView, AnyWindowHandle, Context, Entity, FontWeight, IntoElement, KeyBinding, MouseButton, NavigationDirection,
    ParentElement, Render, Styled, Subscription, Window, WindowControlArea, actions, div, prelude::*, px,
};
use higanbana_core::{Core, Event, library::{Place, classify}};

use crate::{
    chrome::{DownloadsMenu, SearchBox, Sidebar, toasts, window_controls},
    music::bar::MusicBar,
    reader::{ReadTarget, ReaderEvent, ReaderView},
    state::{AppEvent, AppState, state},
    theme,
    ui::icon_button,
    video::{PlayTarget, PlayerEvent, PlayerView, Surface},
    views::{folder::FolderView, home::HomeView, login::LoginView, shelf::ShelfView, title::TitleView},
};

actions!(higanbana, [FocusSearch, Back, Forward]);

pub fn bind_keys(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-k", FocusSearch, None),
        KeyBinding::new("alt-left", Back, None),
        KeyBinding::new("alt-right", Forward, None),
    ]);
}

pub struct AppView {
    state: Entity<AppState>,
    surface: Rc<Surface>,
    page: Option<(String, AnyView)>,
    login: Option<Entity<LoginView>>,
    search: Entity<SearchBox>,
    downloads: Entity<DownloadsMenu>,
    sidebar: Entity<Sidebar>,
    music: Entity<MusicBar>,
    player: Option<(Entity<PlayerView>, Subscription)>,
    reader: Option<(Entity<ReaderView>, Subscription)>,
    _subs: Vec<Subscription>,
}

impl AppView {
    pub fn new(core: Arc<Core>, surface: Rc<Surface>, mut events: UnboundedReceiver<Event>, window_handle: AnyWindowHandle, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = state(cx);
        let _ = core;
        let subs = vec![
            cx.observe(&state, |_, _, cx| cx.notify()),
            cx.subscribe_in(&state, window, |this, _, e: &AppEvent, window, cx| match e {
                AppEvent::Play(t) => this.play(t.clone(), window, cx),
                AppEvent::Read(t) => this.read(t.clone(), window, cx),
                AppEvent::Navigated => {}
            }),
        ];

        // Eventos do núcleo (outras threads) viram mudanças de estado aqui.
        let st = state.clone();
        cx.spawn(async move |_, cx| {
            while let Some(e) = events.next().await {
                let _ = st.update(cx, |s, cx| match &e {
                    Event::AuthChanged(session) => s.set_session(session.as_ref(), cx),
                    Event::DownloadProgress(p) => s.on_download_progress(p, cx),
                    Event::MusicState(m) => s.on_music_state(m.clone(), cx),
                    Event::LoginReturned => {}
                });
                if matches!(e, Event::LoginReturned) {
                    let _ = window_handle.update(cx, |_, w, _| w.activate_window());
                }
            }
        })
        .detach();

        Self {
            search: cx.new(SearchBox::new),
            downloads: cx.new(DownloadsMenu::new),
            sidebar: cx.new(Sidebar::new),
            music: cx.new(MusicBar::new),
            state,
            surface,
            page: None,
            login: None,
            player: None,
            reader: None,
            _subs: subs,
        }
    }

    pub fn play(&mut self, target: PlayTarget, window: &mut Window, cx: &mut Context<Self>) {
        let core = self.state.read(cx).core.clone();
        // Música e vídeo não tocam juntos.
        let _ = core.music().command("pause");
        let surface = self.surface.clone();
        let player = cx.new(|cx| PlayerView::new(core, target, surface, window, cx));
        let sub = cx.subscribe_in(&player, window, |this, _, e: &PlayerEvent, _, cx| match e {
            PlayerEvent::Close => {
                this.player = None;
                this.state.update(cx, |s, cx| s.refresh_player_data(cx));
                cx.notify();
            }
        });
        self.player = Some((player, sub));
        cx.notify();
    }

    pub fn read(&mut self, target: ReadTarget, window: &mut Window, cx: &mut Context<Self>) {
        let reader = cx.new(|cx| ReaderView::new(target, window, cx));
        let sub = cx.subscribe_in(&reader, window, |this, _, e: &ReaderEvent, _, cx| match e {
            ReaderEvent::Close => {
                this.reader = None;
                cx.notify();
            }
        });
        self.reader = Some((reader, sub));
        cx.notify();
    }

    /// Tela da rota atual (recriada quando o caminho muda).
    fn page(&mut self, cx: &mut Context<Self>) -> AnyView {
        let path = self.state.read(cx).path().to_owned();
        if let Some((p, v)) = &self.page
            && *p == path
        {
            return v.clone();
        }
        let view: AnyView = match classify(&path) {
            Place::Home => cx.new(HomeView::new).into(),
            Place::Shelf { path, media } => cx.new(|cx| ShelfView::new(path, media, cx)).into(),
            Place::Title { title_path, rest, media, .. } => cx.new(|cx| TitleView::new(title_path, rest, media, cx)).into(),
            Place::Folder { path } => cx.new(|cx| FolderView::new(path, cx)).into(),
        };
        self.page = Some((path, view.clone()));
        view
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().size_full().font_family("Segoe UI").text_color(theme::fg());
        if let Some((player, _)) = &self.player {
            // Sem fundo: o vídeo aparece pelo que o GPUI deixa transparente.
            return root.child(player.clone()).into_any_element();
        }
        if let Some((reader, _)) = &self.reader {
            return root.bg(theme::bg()).child(reader.clone()).into_any_element();
        }
        let (ready, logged) = {
            let s = self.state.read(cx);
            (s.session_ready, s.user.is_some())
        };
        if !ready {
            return root.bg(theme::bg()).into_any_element();
        }
        let titlebar = |inner: gpui::Div| {
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap_1()
                .h_12()
                .pl_3()
                .border_b_1()
                .border_color(theme::line())
                .bg(theme::bg())
                .child(inner)
                .child(div().h_full().flex_1().window_control_area(WindowControlArea::Drag))
                .child(window_controls(window))
        };
        if !logged {
            let login = self.login.get_or_insert_with(|| cx.new(LoginView::new)).clone();
            return root
                .bg(theme::bg())
                .flex()
                .flex_col()
                .child(titlebar(div().text_xl().font_weight(FontWeight::BOLD).text_color(theme::brand()).child("higanbana")))
                .child(div().flex_1().min_h_0().child(login))
                .into_any_element();
        }
        self.login = None;
        let page = self.page(cx);
        let st = self.state.clone();
        let (can_back, can_fwd, has_music) = {
            let s = self.state.read(cx);
            (s.can_back(), s.can_forward(), s.music.as_ref().is_some_and(|m| m.album.is_some()))
        };
        let path_key = self.state.read(cx).path().to_owned();

        let bar = div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .h_12()
            .pl_3()
            .border_b_1()
            .border_color(theme::line())
            .bg(theme::bg())
            .child(
                div()
                    .id("brand")
                    .mr_2()
                    .px_1()
                    .cursor_pointer()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme::brand())
                    .child("higanbana")
                    .on_click({
                        let st = st.clone();
                        move |_, _, cx| st.update(cx, |s, cx| s.go("", cx))
                    }),
            )
            .child(icon_button("back", "chevron-left").size_8().when(!can_back, |d| d.opacity(0.35)).on_click({
                let st = st.clone();
                move |_, _, cx| st.update(cx, |s, cx| s.back(cx))
            }))
            .child(icon_button("forward", "chevron-right").size_8().when(!can_fwd, |d| d.opacity(0.35)).on_click({
                let st = st.clone();
                move |_, _, cx| st.update(cx, |s, cx| s.forward(cx))
            }))
            .child(
                div()
                    .h_full()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .window_control_area(WindowControlArea::Drag)
                    .child(self.search.clone()),
            )
            .child(div().pr_2().child(self.downloads.clone()))
            .child(window_controls(window));

        root.relative()
            .bg(theme::bg())
            .flex()
            .flex_col()
            .on_action(cx.listener(|v, _: &FocusSearch, window, cx| v.search.update(cx, |s, cx| s.focus(window, cx))))
            .on_action({
                let st = st.clone();
                move |_: &Back, _, cx| st.update(cx, |s, cx| s.back(cx))
            })
            .on_action({
                let st = st.clone();
                move |_: &Forward, _, cx| st.update(cx, |s, cx| s.forward(cx))
            })
            .on_mouse_up(MouseButton::Navigate(NavigationDirection::Back), {
                let st = st.clone();
                move |_, _, cx| st.update(cx, |s, cx| s.back(cx))
            })
            .on_mouse_up(MouseButton::Navigate(NavigationDirection::Forward), {
                let st = st.clone();
                move |_, _, cx| st.update(cx, |s, cx| s.forward(cx))
            })
            .child(bar)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.sidebar.clone())
                    .child(
                        div()
                            .id(gpui::SharedString::from(format!("main-{path_key}")))
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .overflow_y_scroll()
                            .bg(theme::canvas())
                            .child(page),
                    ),
            )
            .when(has_music, |d| d.child(self.music.clone()))
            .child(toasts(&self.state, cx))
            .child(div().absolute().size(px(0.)))
            .into_any_element()
    }
}
