//! Peças de interface reaproveitadas pelas telas.

use std::{borrow::Cow, time::Duration};

use gpui::{
    Animation, AnimationExt, AssetSource, Div, ElementId, IntoElement, ParentElement, SharedString, Stateful,
    Styled, Svg, Transformation, div, percentage, prelude::*, svg,
};
use rust_embed::RustEmbed;

use crate::theme;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/lucide/*.svg"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(Self::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Self::iter().filter(|p| p.starts_with(path)).map(SharedString::from).collect())
    }
}

/// Ícone do lucide (`assets/icons/lucide/<nome>.svg`), na cor do texto.
pub fn icon(name: &'static str) -> Svg {
    // O svg do GPUI não herda a cor do texto: começa com a cor normal.
    svg().path(SharedString::from(format!("icons/lucide/{name}.svg"))).size_5().flex_none().text_color(theme::fg())
}

/// Ícone de carregando, girando.
pub fn spinner(id: impl Into<ElementId>) -> impl IntoElement {
    icon("loader-circle").size_full().with_animation(id, Animation::new(Duration::from_millis(900)).repeat(), |s, d| {
        s.with_transformation(Transformation::rotate(percentage(d)))
    })
}

/// Botão quadrado só com ícone (barra de controles, cabeçalhos).
pub fn icon_button(id: impl Into<ElementId>, name: &'static str) -> Stateful<Div> {
    icon_toggle(id, name, false)
}

/// Botão de ícone que fica na cor de destaque quando ativo (menu aberto etc.).
pub fn icon_toggle(id: impl Into<ElementId>, name: &'static str, active: bool) -> Stateful<Div> {
    div()
        .id(id)
        .size_10()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_lg()
        .cursor_pointer()
        .text_color(theme::fg())
        .hover(|s| s.bg(theme::white_a(0.10)))
        .child(icon(name).when(active, |i| i.text_color(theme::accent_ink())))
}

/// Botão de texto: principal (cor de destaque) ou discreto (borda).
pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, primary: bool) -> Stateful<Div> {
    let base = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap_1p5()
        .px_4()
        .py_2()
        .rounded_lg()
        .cursor_pointer()
        .text_sm()
        .font_weight(gpui::FontWeight::SEMIBOLD);
    if primary {
        base.bg(theme::accent()).text_color(theme::white()).hover(|s| s.bg(theme::accent_ink()))
    } else {
        base.border_1()
            .border_color(theme::line_strong())
            .text_color(theme::fg())
            .hover(|s| s.bg(theme::white_a(0.08)))
    }
    .child(label.into())
}

/// Texto em uma linha só, cortado com reticências.
pub fn truncate(text: impl Into<SharedString>) -> Div {
    div().min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap().child(text.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_are_embedded() {
        assert!(Assets::get("icons/lucide/play.svg").is_some(), "{:?}", Assets::iter().take(5).collect::<Vec<_>>());
    }
}
