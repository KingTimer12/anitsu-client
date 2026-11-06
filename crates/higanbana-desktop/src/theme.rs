//! Tema do higanbana: as cores ficam em `themes/higanbana.json` (formato de tema
//! do GPUI Kit) e as telas leem tudo de `cx.theme()` pelo papel da cor.
//!
//! Aqui ficam só os poucos papéis que o tema do kit não tem:
//! - a tinta da marca para texto (vermelho claro legível no fundo escuro);
//! - as cores sobre mídia (vídeo, capas e banners), que não seguem o tema
//!   porque o fundo é a própria imagem.

use std::rc::Rc;

use gpui_kit::{
    App, Hsla, Window,
    component::{Theme, ThemeSet},
};

const THEME: &str = include_str!("../themes/higanbana.json");

/// Instala o tema escuro do higanbana. Chamar logo depois de `gpui_kit::init`.
pub fn install(cx: &mut App) {
    let set: ThemeSet = serde_json::from_str(THEME).expect("themes/higanbana.json válido");
    let config = Rc::new(set.themes.into_iter().next().expect("um tema em higanbana.json"));
    Theme::update(cx, |theme| theme.apply_config(&config));
}

/// Papéis do produto que o tema do kit não nomeia.
pub trait ProductColors {
    /// Texto na cor da marca (rótulos ativos, número da faixa tocando).
    fn primary_ink(&self) -> Hsla;
}

impl ProductColors for Theme {
    fn primary_ink(&self) -> Hsla {
        // `base.red.light` no arquivo do tema.
        self.red_light
    }
}

/// Texto e ícones sobre vídeo, capa ou banner: sempre claros.
pub fn on_media() -> Hsla {
    gpui_kit::white()
}

/// Texto secundário sobre mídia.
pub fn on_media_muted() -> Hsla {
    gpui_kit::white().opacity(0.72)
}

/// Véu escuro sobre mídia, para o texto ficar legível.
pub fn media_scrim(alpha: f32) -> Hsla {
    gpui_kit::black().opacity(alpha)
}

/// Trilho claro sobre mídia (barra de posição, volume).
pub fn media_track(alpha: f32) -> Hsla {
    gpui_kit::white().opacity(alpha)
}

/// O vídeo é uma janela nativa embaixo da composição do GPUI: com o player
/// aberto o `Root` não pode pintar o fundo, senão cobre o vídeo. Só o token de
/// fundo do `Root` muda (as cores do tema ficam iguais) e volta ao fechar.
pub fn set_video_backdrop(on: bool, window: &mut Window, cx: &mut App) {
    let theme = Theme::global_mut(cx);
    theme.tokens.background = if on { gpui_kit::transparent_black().into() } else { theme.colors.background.into() };
    window.refresh();
}
