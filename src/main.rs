//! Chonk – archiviert die Messungen einer Tanita-Körperanalysewaage
//! von der SD-Karte in einer lokalen SQLite-Datenbank und stellt sie dar.

// Unter Windows kein Konsolenfenster neben der App öffnen.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod db;
mod i18n;
mod model;
mod sdcard;
mod tanita;
mod ui;

use gpui::{
    App, Application, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions, WindowBounds,
    WindowOptions, actions, prelude::*, px, size,
};

use crate::db::Db;
use crate::i18n::Lang;
use crate::ui::app::ChonkApp;

actions!(chonk, [Quit]);

/// Baut das App-Menü in der aktuellen Sprache auf.
pub fn set_menus(cx: &mut App) {
    cx.set_menus(vec![Menu {
        name: "Chonk".into(),
        items: vec![MenuItem::action(i18n::t().quit, Quit)],
    }]);
}

fn main() {
    let db = match Db::open_default() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    };

    // Gespeicherte Sprachwahl, sonst die Sprache des Betriebssystems.
    let saved = db.setting("language").ok().flatten();
    i18n::set_lang(
        saved
            .and_then(|code| Lang::from_code(&code))
            .unwrap_or_else(Lang::system),
    );

    Application::new().run(move |cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("ctrl-q", Quit, None),
        ]);
        set_menus(cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1320.), px(920.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(1040.), px(680.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Chonk".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| ChonkApp::new(db, cx)),
        )
        .expect("could not open the window");
        cx.activate(true);
    });
}
