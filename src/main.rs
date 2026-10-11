//! Fettsack – archiviert die Messungen einer Tanita-Körperanalysewaage
//! von der SD-Karte in einer lokalen SQLite-Datenbank und stellt sie dar.

mod db;
mod model;
mod sdcard;
mod tanita;
mod ui;

use gpui::{
    App, Application, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions, WindowBounds,
    WindowOptions, actions, prelude::*, px, size,
};

use crate::db::Db;
use crate::ui::app::FettsackApp;

actions!(fettsack, [Quit]);

fn main() {
    let db = match Db::open_default() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Fehler: {e:#}");
            std::process::exit(1);
        }
    };

    Application::new().run(move |cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus(vec![Menu {
            name: "Fettsack".into(),
            items: vec![MenuItem::action("Fettsack beenden", Quit)],
        }]);
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
                    title: Some("Fettsack".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| FettsackApp::new(db, cx)),
        )
        .expect("Fenster konnte nicht geöffnet werden");
        cx.activate(true);
    });
}
