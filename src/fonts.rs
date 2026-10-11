//! Embedded Google Fonts (SIL OFL): VT323 + Share Tech Mono for terminal text,
//! Monoton + Bungee Shade for titles/logo.
use bevy::prelude::*;
use std::sync::OnceLock;

struct Fonts {
    term: Handle<Font>,
    mono: Handle<Font>,
    logo: Handle<Font>,
    shade: Handle<Font>,
}
static FONTS: OnceLock<Fonts> = OnceLock::new();

pub fn init(app: &mut App) {
    let mut assets = app.world_mut().resource_mut::<Assets<Font>>();
    let mut load = |b: &'static [u8]| assets.add(Font::try_from_bytes(b.to_vec()).expect("font"));
    let f = Fonts {
        term: load(include_bytes!("fonts/VT323-Regular.ttf")),
        mono: load(include_bytes!("fonts/ShareTechMono-Regular.ttf")),
        logo: load(include_bytes!("fonts/Monoton-Regular.ttf")),
        shade: load(include_bytes!("fonts/BungeeShade-Regular.ttf")),
    };
    let _ = FONTS.set(f);
}

/// VT323: 8-bit terminal (HUD, menus, dialogue).
pub fn term() -> Handle<Font> {
    FONTS.get().map(|f| f.term.clone()).unwrap_or_default()
}
/// Share Tech Mono: DOS terminal (console, small text).
pub fn mono() -> Handle<Font> {
    FONTS.get().map(|f| f.mono.clone()).unwrap_or_default()
}
/// Monoton: logo / big titles.
pub fn logo() -> Handle<Font> {
    FONTS.get().map(|f| f.logo.clone()).unwrap_or_default()
}
/// Bungee Shade: headers (PAUSED, KERNEL PANIC...).
pub fn shade() -> Handle<Font> {
    FONTS.get().map(|f| f.shade.clone()).unwrap_or_default()
}
