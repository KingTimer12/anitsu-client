// Sem console extra no Windows em release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod chrome;
mod components;
mod input;
mod music;
mod reader;
mod state;
mod theme;
mod ui;
mod video;
mod views;

use std::{rc::Rc, sync::Arc};

use futures::channel::mpsc;
use gpui::{App, AppContext, Application, Bounds, TitlebarOptions, Window, WindowBounds, WindowOptions, point, px, size};
use higanbana_core::{Core, Event, Paths};

fn main() {
    let paths = Paths::system().expect("pastas do app");
    let (tx, rx) = mpsc::unbounded::<Event>();
    let events = Arc::new(move |e| {
        let _ = tx.unbounded_send(e);
    });
    let core = Arc::new(Core::new(paths, events).expect("núcleo do higanbana"));

    Application::new().with_assets(ui::Assets).run(move |cx: &mut App| {
        input::bind_keys(cx);
        app::bind_keys(cx);
        state::install(core.clone(), cx);
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // Barra de título própria (a moldura desenha os botões da janela).
            titlebar: Some(TitlebarOptions { title: Some("higanbana".into()), appears_transparent: true, traffic_light_position: Some(point(px(12.), px(12.))) }),
            window_min_size: Some(size(px(960.), px(600.))),
            ..Default::default()
        };
        cx.open_window(options, |window, cx| {
            let hwnd = hwnd(window);
            core.music().attach_media_keys(hwnd);
            let surface = Rc::new(hwnd.and_then(video::Surface::new).expect("janela do vídeo"));
            let handle = window.window_handle();
            cx.new(|cx| app::AppView::new(core.clone(), surface, rx, handle, window, cx))
        })
        .expect("janela principal");
        cx.on_window_closed(|cx| cx.quit()).detach();
        cx.activate(true);
    });
}

/// HWND da janela (teclas de mídia / SMTC e vídeo no Windows).
fn hwnd(window: &Window) -> Option<usize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match HasWindowHandle::window_handle(window).ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as usize),
        _ => None,
    }
}
