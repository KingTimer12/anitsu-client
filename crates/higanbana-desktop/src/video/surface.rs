//! Janela filha onde o mpv desenha (`--wid`). Fica embaixo do que o GPUI desenha:
//! o GPUI compõe por cima das janelas filhas (DirectComposition) e deixa
//! transparente o que não pinta, então os controles aparecem sobre o vídeo.
//!
//! A janela é desabilitada: cliques e o mouse passam para a janela principal.

#[cfg(windows)]
mod imp {
    use std::sync::OnceLock;

    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{BLACK_BRUSH, GetStockObject, HBRUSH},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, HWND_BOTTOM, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE,
            SWP_NOACTIVATE, SetWindowPos, ShowCursor, ShowWindow, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS,
            WS_DISABLED,
        },
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    unsafe extern "system" fn proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        unsafe { DefWindowProcW(h, m, w, l) }
    }

    fn class() -> &'static [u16] {
        static CLASS: OnceLock<Vec<u16>> = OnceLock::new();
        CLASS.get_or_init(|| {
            let name = wide("HiganbanaVideo");
            // SAFETY: classe registrada uma vez, com nome estático e procedimento padrão.
            unsafe {
                let wc = WNDCLASSW {
                    lpfnWndProc: Some(proc),
                    hInstance: GetModuleHandleW(std::ptr::null()),
                    lpszClassName: name.as_ptr(),
                    hbrBackground: GetStockObject(BLACK_BRUSH) as HBRUSH,
                    ..std::mem::zeroed()
                };
                RegisterClassW(&wc);
            }
            name
        })
    }

    pub struct Surface {
        hwnd: HWND,
        last: std::cell::Cell<(i32, i32, i32, i32, bool)>,
        cursor_hidden: std::cell::Cell<bool>,
    }

    impl Surface {
        pub fn new(parent: usize) -> Option<Self> {
            let class = class();
            // SAFETY: filha da janela principal, criada na thread da interface.
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    std::ptr::null(),
                    WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN | WS_DISABLED,
                    0,
                    0,
                    1,
                    1,
                    parent as HWND,
                    std::ptr::null_mut(),
                    GetModuleHandleW(std::ptr::null()),
                    std::ptr::null(),
                )
            };
            (!hwnd.is_null()).then(|| Self { hwnd, last: std::cell::Cell::new((0, 0, 0, 0, false)), cursor_hidden: std::cell::Cell::new(false) })
        }

        pub fn wid(&self) -> usize {
            self.hwnd as usize
        }

        /// Posição em pixels físicos da área cliente da janela principal.
        pub fn place(&self, x: i32, y: i32, w: i32, h: i32) {
            let next = (x, y, w.max(1), h.max(1), true);
            if self.last.get() == next {
                return;
            }
            let was_visible = self.last.get().4;
            self.last.set(next);
            // SAFETY: hwnd nosso, mesma thread.
            unsafe {
                SetWindowPos(self.hwnd, HWND_BOTTOM, x, y, next.2, next.3, SWP_NOACTIVATE);
                if !was_visible {
                    ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                }
            }
        }

        /// Esconde o cursor do sistema sobre o vídeo parado de mexer. O GPUI não
        /// tem um estilo de cursor vazio; `ShowCursor` usa um contador por thread,
        /// então só chamamos quando o estado muda, para ele voltar ao equilíbrio.
        pub fn set_cursor_hidden(&self, hidden: bool) {
            if self.cursor_hidden.replace(hidden) != hidden {
                // SAFETY: chamada na thread da interface, sempre em pares.
                unsafe { ShowCursor(if hidden { 0 } else { 1 }) };
            }
        }

        pub fn hide(&self) {
            self.set_cursor_hidden(false);
            let mut l = self.last.get();
            if l.4 {
                l.4 = false;
                self.last.set(l);
                // SAFETY: hwnd nosso, mesma thread.
                unsafe { ShowWindow(self.hwnd, SW_HIDE) };
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct Surface;
    impl Surface {
        pub fn new(_parent: usize) -> Option<Self> {
            None
        }
        pub fn wid(&self) -> usize {
            0
        }
        pub fn place(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}
        pub fn hide(&self) {}
        pub fn set_cursor_hidden(&self, _hidden: bool) {}
    }
}

pub use imp::Surface;
