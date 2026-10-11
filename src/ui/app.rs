//! Hauptansicht: Kopfzeile, Zeitraumfilter, Übersicht mit Diagrammen und Messwerttabelle.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{Local, NaiveDateTime};
use gpui::{
    AnyElement, ClickEvent, Context, Div, FocusHandle, FontWeight, IntoElement, KeyDownEvent,
    MouseDownEvent, MouseMoveEvent, Render, SharedString, Window, div, prelude::*, px, relative,
    rgb,
};

use super::chart::{ChartSpec, line_chart, sparkline};
use super::theme;
use crate::db::Db;
use crate::model::{
    Better, Metric, Range, Status, bmi_status, fat_status, fmt_date, fmt_date_time, fmt_num,
    visceral_status,
};
use crate::sdcard;
use crate::tanita::{Measurement, Profile};

/// Maximale Länge eines Profilnamens in Zeichen.
const MAX_NAME_LEN: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Overview,
    Table,
}

pub struct FettsackApp {
    db: Db,
    /// Profil-Slots mit Messungen und deren Anzahl.
    slots: Vec<(u8, usize)>,
    profile: u8,
    profile_info: Option<Profile>,
    /// Selbst vergebene Profilnamen je Slot.
    names: HashMap<u8, String>,
    /// Eingabepuffer, solange der Profilname bearbeitet wird.
    editing_name: Option<String>,
    name_focus: FocusHandle,
    /// Alle Messungen des gewählten Profils, älteste zuerst.
    data: Vec<Measurement>,
    last_import: Option<NaiveDateTime>,
    range: Range,
    tab: Tab,
    card: Option<PathBuf>,
    message: Option<String>,
}

impl FettsackApp {
    pub fn new(db: Db, cx: &mut Context<Self>) -> Self {
        let mut app = Self {
            db,
            slots: Vec::new(),
            profile: 1,
            profile_info: None,
            names: HashMap::new(),
            editing_name: None,
            name_focus: cx.focus_handle(),
            data: Vec::new(),
            last_import: None,
            range: Range::All,
            tab: Tab::Overview,
            card: None,
            message: None,
        };
        app.reload();
        app.poll_card(cx);

        // Karte im Hintergrund beobachten: Einlegen löst automatisch einen Import aus.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                if this.update(cx, |this, cx| this.poll_card(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        app
    }

    fn reload(&mut self) {
        let result = (|| -> anyhow::Result<()> {
            self.slots = self.db.profile_slots()?;
            if !self.slots.iter().any(|(s, _)| *s == self.profile)
                && let Some((first, _)) = self.slots.first()
            {
                self.profile = *first;
            }
            self.data = self.db.measurements(self.profile)?;
            self.profile_info = self.db.profile(self.profile)?;
            self.names = self.db.profile_names()?;
            self.last_import = self.db.last_import()?;
            Ok(())
        })();
        if let Err(e) = result {
            self.message = Some(format!("Archiv nicht lesbar: {e:#}"));
        }
    }

    fn poll_card(&mut self, cx: &mut Context<Self>) {
        let card = sdcard::find_card();
        if card == self.card {
            return;
        }
        let inserted = card.is_some();
        self.card = card;
        if inserted {
            self.import(cx);
        }
        cx.notify();
    }

    fn import(&mut self, cx: &mut Context<Self>) {
        let Some(card) = self.card.clone() else {
            self.message = Some("Keine SD-Karte gefunden.".into());
            cx.notify();
            return;
        };
        let result = sdcard::read_card(&card).and_then(|data| {
            let stats = self.db.import(
                &card.display().to_string(),
                &data.profiles,
                &data.measurements,
            )?;
            Ok((stats, data.warnings))
        });
        self.message = Some(match result {
            Ok((stats, warnings)) => {
                for w in &warnings {
                    eprintln!("Import-Warnung: {w}");
                }
                let mut msg = match stats.new {
                    0 => format!("{} Messungen gelesen, alle bereits archiviert", stats.seen),
                    1 => format!("{} Messungen gelesen, 1 neu archiviert", stats.seen),
                    n => format!("{} Messungen gelesen, {n} neu archiviert", stats.seen),
                };
                if !warnings.is_empty() {
                    msg.push_str(&format!(" · {} Zeilen übersprungen", warnings.len()));
                }
                msg
            }
            Err(e) => format!("Import fehlgeschlagen: {e:#}"),
        });
        self.reload();
        cx.notify();
    }

    // -------------------------------------------------------- Profilnamen

    fn profile_label(&self, slot: u8) -> String {
        self.names
            .get(&slot)
            .cloned()
            .unwrap_or_else(|| format!("Profil {slot}"))
    }

    fn start_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_name = Some(self.names.get(&self.profile).cloned().unwrap_or_default());
        window.focus(&self.name_focus);
        cx.notify();
    }

    fn finish_rename(&mut self, save: bool, cx: &mut Context<Self>) {
        let Some(name) = self.editing_name.take() else {
            return;
        };
        if save {
            if let Err(e) = self.db.set_profile_name(self.profile, &name) {
                self.message = Some(format!("Name nicht gespeichert: {e:#}"));
            }
            self.reload();
        }
        cx.notify();
    }

    fn rename_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(buffer) = self.editing_name.as_mut() else {
            return;
        };
        let key = &event.keystroke;
        match key.key.as_str() {
            "enter" => return self.finish_rename(true, cx),
            "escape" => return self.finish_rename(false, cx),
            "backspace" => {
                buffer.pop();
            }
            _ => {
                let shortcut = key.modifiers.control || key.modifiers.platform;
                if let Some(text) = key.key_char.as_ref().filter(|_| !shortcut)
                    && buffer.chars().count() + text.chars().count() <= MAX_NAME_LEN
                {
                    buffer.extend(text.chars().filter(|c| !c.is_control()));
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// Name des gewählten Profils; ein Klick macht daraus ein Eingabefeld.
    fn profile_name(&self, cx: &Context<Self>) -> AnyElement {
        let chip = || {
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .px_2()
                .py_0p5()
                .rounded(px(6.))
                .border_1()
        };
        match &self.editing_name {
            Some(buffer) => chip()
                .id("profile-name-input")
                .track_focus(&self.name_focus)
                .border_color(rgb(theme::WEIGHT))
                .bg(rgb(theme::SURFACE))
                .min_w(px(150.))
                .on_key_down(
                    cx.listener(|this, event: &KeyDownEvent, _, cx| this.rename_key(event, cx)),
                )
                .on_mouse_down_out(
                    cx.listener(|this, _: &MouseDownEvent, _, cx| this.finish_rename(true, cx)),
                )
                .child(div().text_color(rgb(theme::TEXT)).child(buffer.clone()))
                .child(div().w(px(1.)).h(px(15.)).bg(rgb(theme::TEXT)))
                .when(buffer.is_empty(), |d| {
                    d.child(
                        div()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(format!("Name für Profil {}", self.profile)),
                    )
                })
                .into_any_element(),
            None => chip()
                .id("profile-name")
                .border_color(rgb(theme::BORDER))
                .text_color(rgb(theme::TEXT_SECONDARY))
                .cursor_pointer()
                .hover(|s| s.bg(rgb(theme::SURFACE_RAISED)))
                .on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.start_rename(window, cx)),
                )
                .child(self.profile_label(self.profile))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child("umbenennen"),
                )
                .into_any_element(),
        }
    }

    // ---------------------------------------------------------------- Kopf

    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        let mut facts: Vec<String> = Vec::new();
        if let Some(p) = &self.profile_info {
            if !p.model.is_empty() {
                facts.push(format!("Tanita {}", p.model));
            }
            if let Some(h) = p.height_cm {
                facts.push(format!("{} cm", fmt_num(h, 0)));
            }
            if let Some(b) = p.birth_date {
                facts.push(format!("geboren {}", b.format("%d.%m.%Y")));
            }
        }
        if facts.is_empty() {
            facts.push("Körperanalyse-Archiv".into());
        }

        let (pill_color, pill_text) = match &self.card {
            Some(card) => (
                theme::GOOD,
                format!("SD-Karte „{}“ verbunden", sdcard::volume_name(card)),
            ),
            None => (theme::TEXT_MUTED, "Keine SD-Karte eingelegt".to_string()),
        };
        let can_import = self.card.is_some();

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .px_6()
            .pt_5()
            .pb_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(FontWeight::BOLD)
                            .child("Fettsack"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(self.profile_name(cx))
                            .child(
                                div()
                                    .text_color(rgb(theme::TEXT_MUTED))
                                    .child(facts.join("  ·  ")),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .children(self.message.clone().map(|m| {
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(m)
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_1p5()
                            .rounded_full()
                            .border_1()
                            .border_color(rgb(theme::BORDER))
                            .bg(rgb(theme::SURFACE))
                            .text_size(px(12.))
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child(div().size(px(8.)).rounded_full().bg(rgb(pill_color)))
                            .child(pill_text),
                    )
                    .child(
                        div()
                            .id("import")
                            .px_3()
                            .py_1p5()
                            .rounded(px(8.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .when(can_import, |d| {
                                d.bg(rgb(theme::WEIGHT))
                                    .text_color(rgb(theme::TEXT))
                                    .cursor_pointer()
                                    .hover(|s| s.opacity(0.85))
                                    .on_click(
                                        cx.listener(|this, _: &ClickEvent, _, cx| this.import(cx)),
                                    )
                            })
                            .when(!can_import, |d| {
                                d.bg(rgb(theme::SURFACE_RAISED))
                                    .text_color(rgb(theme::TEXT_MUTED))
                            })
                            .child("Jetzt importieren"),
                    ),
            )
    }

    fn segment_button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        active: bool,
        cx: &Context<Self>,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .id(id.into())
            .px_3()
            .py_1()
            .rounded(px(6.))
            .text_size(px(12.))
            .cursor_pointer()
            .when(active, |d| {
                d.bg(rgb(theme::SURFACE_RAISED))
                    .text_color(rgb(theme::TEXT))
                    .font_weight(FontWeight::SEMIBOLD)
            })
            .when(!active, |d| {
                d.text_color(rgb(theme::TEXT_SECONDARY))
                    .hover(|s| s.bg(rgb(theme::GRID)))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                on_click(this, cx);
                cx.notify();
            }))
            .child(label.into())
    }

    fn toolbar(&self, cx: &Context<Self>) -> impl IntoElement {
        let group = || {
            div()
                .flex()
                .items_center()
                .gap_1()
                .p_1()
                .rounded(px(9.))
                .bg(rgb(theme::SURFACE))
                .border_1()
                .border_color(rgb(theme::BORDER))
        };

        let ranges = group().children(Range::ALL.into_iter().map(|r| {
            self.segment_button(
                format!("range-{}", r.label()),
                r.label(),
                self.range == r,
                cx,
                move |this, _| this.range = r,
            )
        }));

        let tabs = group()
            .child(self.segment_button(
                "tab-overview",
                "Übersicht",
                self.tab == Tab::Overview,
                cx,
                |this, _| this.tab = Tab::Overview,
            ))
            .child(self.segment_button(
                "tab-table",
                "Messungen",
                self.tab == Tab::Table,
                cx,
                |this, _| this.tab = Tab::Table,
            ));

        let profiles = (self.slots.len() > 1).then(|| {
            group().children(self.slots.iter().map(|(slot, _)| {
                let slot = *slot;
                self.segment_button(
                    format!("profile-{slot}"),
                    self.profile_label(slot),
                    self.profile == slot,
                    cx,
                    move |this, _| {
                        this.editing_name = None;
                        this.profile = slot;
                        this.reload();
                    },
                )
            }))
        });

        div()
            .flex()
            .items_center()
            .gap_3()
            .px_6()
            .pb_4()
            .child(ranges)
            .children(profiles)
            .child(div().flex_1())
            .child(tabs)
    }

    fn footer(&self) -> impl IntoElement {
        let mut parts = vec![
            format!("Archiv: {}", crate::db::display_path(self.db.path())),
            format!("{} Messungen", self.data.len()),
        ];
        if let Some(ts) = self.last_import {
            parts.push(format!("letzter Import {}", fmt_date_time(ts)));
        }
        div()
            .px_6()
            .py_2()
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .text_size(px(11.))
            .text_color(rgb(theme::TEXT_MUTED))
            .child(parts.join("  ·  "))
    }

    // ----------------------------------------------------------- Übersicht

    fn overview(&self, visible: &[Measurement]) -> AnyElement {
        let Some(latest) = visible.last() else {
            return self.empty_state();
        };

        let fat_extra = latest
            .fat_pct
            .map(|f| fat_status(f, latest.gender, latest.age));
        let bmi_extra = latest.bmi.map(bmi_status);
        let visceral_extra = latest.visceral_fat.map(visceral_status);

        let age_note = match (latest.metabolic_age, latest.age) {
            (Some(met), Some(age)) => {
                let diff = age as f64 - met;
                Some(if diff >= 1.0 {
                    format!("{} Jahre jünger als tatsächlich", fmt_num(diff, 0))
                } else if diff <= -1.0 {
                    format!("{} Jahre älter als tatsächlich", fmt_num(-diff, 0))
                } else {
                    "entspricht dem tatsächlichen Alter".to_string()
                })
            }
            _ => None,
        };

        let tiles = div()
            .grid()
            .grid_cols(4)
            .gap_4()
            .child(tile(Metric::Fat, visible, fat_extra, None))
            .child(tile(Metric::Muscle, visible, None, None))
            .child(tile(Metric::Water, visible, None, None))
            .child(tile(Metric::Bone, visible, None, None))
            .child(tile(Metric::Bmi, visible, bmi_extra, None))
            .child(tile(Metric::Visceral, visible, visceral_extra, None))
            .child(tile(Metric::MetabolicAge, visible, None, age_note))
            .child(tile(Metric::Dci, visible, None, None));

        let small_charts = div()
            .grid()
            .grid_cols(3)
            .gap_4()
            .child(chart_card(Metric::Fat, theme::FAT, visible, 180.))
            .child(chart_card(Metric::Muscle, theme::MUSCLE, visible, 180.))
            .child(chart_card(Metric::Water, theme::WATER, visible, 180.));

        div()
            .id("overview")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .px_6()
                    .pb_6()
                    .child(div().flex().gap_4().child(hero(visible, latest)).child(
                        div().flex_1().min_w(px(0.)).child(chart_card(
                            Metric::Weight,
                            theme::WEIGHT,
                            visible,
                            250.,
                        )),
                    ))
                    .child(tiles)
                    .child(small_charts)
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_4()
                            .child(segments_card(latest))
                            .child(composition_card(latest)),
                    ),
            )
            .into_any_element()
    }

    fn empty_state(&self) -> AnyElement {
        let (title, hint) = if self.data.is_empty() {
            (
                "Noch keine Messungen archiviert".to_string(),
                "Lege die SD-Karte der Waage ein. Die Daten werden automatisch importiert."
                    .to_string(),
            )
        } else {
            let last = self.data.last().map(|m| fmt_date(m.measured_at));
            (
                format!("Keine Messungen im Zeitraum „{}“", self.range.label()),
                format!(
                    "Die letzte Messung ist vom {}. Wähle oben einen längeren Zeitraum.",
                    last.unwrap_or_default()
                ),
            )
        };
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(div().text_color(rgb(theme::TEXT_MUTED)).child(hint))
            .into_any_element()
    }

    // ------------------------------------------------------------- Tabelle

    fn table(&self, visible: &[Measurement]) -> AnyElement {
        if visible.is_empty() {
            return self.empty_state();
        }
        type Cell = fn(&Measurement) -> String;
        fn opt(m: Metric, x: &Measurement) -> String {
            m.value(x)
                .map(|v| m.format(v))
                .unwrap_or_else(|| "–".into())
        }
        let columns: [(&str, f32, Cell); 11] = [
            ("Datum", 1.3, |m| fmt_date(m.measured_at)),
            ("Uhrzeit", 0.9, |m| {
                m.measured_at.format("%H:%M").to_string()
            }),
            ("Gewicht kg", 1.0, |m| opt(Metric::Weight, m)),
            ("BMI", 0.8, |m| opt(Metric::Bmi, m)),
            ("Fett %", 0.8, |m| opt(Metric::Fat, m)),
            ("Muskeln kg", 1.0, |m| opt(Metric::Muscle, m)),
            ("Wasser %", 0.9, |m| opt(Metric::Water, m)),
            ("Knochen kg", 1.0, |m| opt(Metric::Bone, m)),
            ("Viszeralfett", 1.0, |m| opt(Metric::Visceral, m)),
            ("Stoffw.-Alter", 1.1, |m| opt(Metric::MetabolicAge, m)),
            ("Bedarf kcal", 1.0, |m| opt(Metric::Dci, m)),
        ];

        let row = |header: bool| {
            div()
                .flex()
                .items_center()
                .px_4()
                .py_2()
                .border_b_1()
                .border_color(rgb(theme::GRID))
                .when(header, |d| {
                    d.text_size(px(11.))
                        .text_color(rgb(theme::TEXT_MUTED))
                        .font_weight(FontWeight::SEMIBOLD)
                })
                .when(!header, |d| d.hover(|s| s.bg(rgb(theme::GRID))))
        };
        // Spaltenbreiten über flex-grow-Gewichte
        let weights: Vec<f32> = columns.iter().map(|c| c.1).collect();
        let weighted = |cells: Vec<String>, header: bool| {
            let mut r = row(header);
            for (i, text) in cells.into_iter().enumerate() {
                let mut cell = div()
                    .flex_basis(relative(0.))
                    .min_w(px(0.))
                    .when(i >= 2, |d| d.flex().justify_end())
                    .when(i < 2 && !header, |d| {
                        d.text_color(rgb(theme::TEXT_SECONDARY))
                    })
                    .child(text);
                cell.style().flex_grow = Some(weights[i]);
                r = r.child(cell);
            }
            r
        };

        let header = weighted(columns.iter().map(|c| c.0.to_string()).collect(), true);
        let rows = visible
            .iter()
            .rev()
            .map(|m| weighted(columns.iter().map(|c| (c.2)(m)).collect(), false));

        div()
            .flex_1()
            .min_h(px(0.))
            .px_6()
            .pb_6()
            .flex()
            .flex_col()
            .child(
                card()
                    .p_0()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(header)
                    .child(
                        div()
                            .id("table-rows")
                            .flex_1()
                            .min_h(px(0.))
                            .overflow_y_scroll()
                            .children(rows),
                    ),
            )
            .into_any_element()
    }
}

impl Render for FettsackApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Local::now().naive_local();
        let visible = self.range.filter(&self.data, now);
        let body = match self.tab {
            Tab::Overview => self.overview(visible),
            Tab::Table => self.table(visible),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(theme::PAGE))
            .text_color(rgb(theme::TEXT))
            .text_size(px(13.))
            // Diagramme lesen die Mausposition beim Zeichnen; jede Bewegung zeichnet neu.
            .on_mouse_move(cx.listener(|_, _: &MouseMoveEvent, _, cx| cx.notify()))
            .child(self.header(cx))
            .child(self.toolbar(cx))
            .child(body)
            .child(self.footer())
    }
}

// ------------------------------------------------------------------ Bausteine

fn card() -> Div {
    div()
        .bg(rgb(theme::SURFACE))
        .border_1()
        .border_color(rgb(theme::BORDER))
        .rounded(px(12.))
        .p_4()
}

fn card_title(title: impl Into<SharedString>, note: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_baseline()
        .gap_2()
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.into()),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(theme::TEXT_MUTED))
                .child(note.into()),
        )
}

fn status_color(status: Status) -> u32 {
    match status {
        Status::Good => theme::GOOD,
        Status::Warning => theme::WARNING,
        Status::Serious => theme::SERIOUS,
    }
}

/// Veränderung zwischen erster und letzter Messung des Zeitraums.
fn delta_row(metric: Metric, data: &[Measurement]) -> Div {
    let row = div()
        .flex()
        .items_center()
        .gap_1p5()
        .text_size(px(12.))
        .text_color(rgb(theme::TEXT_SECONDARY));
    let mut values = data
        .iter()
        .filter_map(|m| Some((m.measured_at, metric.value(m)?)));
    let first = values.next();
    let last = values.next_back();
    let (Some((since, a)), Some((_, b))) = (first, last) else {
        return row
            .text_color(rgb(theme::TEXT_MUTED))
            .child("einzige Messung im Zeitraum");
    };
    let diff = b - a;
    let threshold = 0.5 * 10f64.powi(-(metric.decimals() as i32));
    if diff.abs() < threshold {
        return row.child(format!("unverändert seit {}", fmt_date(since)));
    }
    let improved = match metric.better() {
        Better::Lower => Some(diff < 0.0),
        Better::Higher => Some(diff > 0.0),
        Better::Neutral => None,
    };
    let color = match improved {
        Some(true) => theme::GOOD,
        Some(false) => theme::CRITICAL,
        None => theme::TEXT_MUTED,
    };
    let sign = if diff > 0.0 { "+" } else { "−" };
    row.child(
        div()
            .text_size(px(9.))
            .text_color(rgb(color))
            .child(if diff > 0.0 { "▲" } else { "▼" }),
    )
    .child(format!(
        "{sign}{} seit {}",
        metric.format_with_unit(diff.abs()),
        fmt_date(since)
    ))
}

fn status_row(label: impl Into<SharedString>, status: Status) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1p5()
        .text_size(px(12.))
        .text_color(rgb(theme::TEXT_SECONDARY))
        .child(
            div()
                .size(px(7.))
                .rounded_full()
                .bg(rgb(status_color(status))),
        )
        .child(label.into())
}

fn hero(visible: &[Measurement], latest: &Measurement) -> Div {
    let weights: Vec<f64> = visible.iter().map(|m| m.weight_kg).collect();
    let min = weights.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = weights.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let avg = weights.iter().sum::<f64>() / weights.len() as f64;
    let stat = |label: &'static str, v: f64| {
        div()
            .flex()
            .justify_between()
            .text_size(px(12.))
            .child(div().text_color(rgb(theme::TEXT_MUTED)).child(label))
            .child(
                div()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(Metric::Weight.format_with_unit(v)),
            )
    };

    card()
        .w(px(280.))
        .flex_none()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child("Aktuelles Gewicht"),
        )
        .child(
            div()
                .flex()
                .items_baseline()
                .gap_2()
                .mt_2()
                .child(
                    div()
                        .text_size(px(54.))
                        .line_height(px(60.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(Metric::Weight.format(latest.weight_kg)),
                )
                .child(
                    div()
                        .text_size(px(18.))
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child("kg"),
                ),
        )
        .child(div().mt_1().child(delta_row(Metric::Weight, visible)))
        .child(
            div()
                .mt_1()
                .text_size(px(12.))
                .text_color(rgb(theme::TEXT_MUTED))
                .child(format!("gemessen am {}", fmt_date_time(latest.measured_at))),
        )
        .child(div().flex_1().min_h(px(16.)))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1p5()
                .pt_3()
                .border_t_1()
                .border_color(rgb(theme::GRID))
                .child(stat("Tiefstwert", min))
                .child(stat("Höchstwert", max))
                .child(stat("Durchschnitt", avg))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .text_size(px(12.))
                        .child(div().text_color(rgb(theme::TEXT_MUTED)).child("Messungen"))
                        .child(
                            div()
                                .text_color(rgb(theme::TEXT_SECONDARY))
                                .child(visible.len().to_string()),
                        ),
                ),
        )
}

fn tile(
    metric: Metric,
    data: &[Measurement],
    status: Option<(&'static str, Status)>,
    note: Option<String>,
) -> Div {
    let latest = data.iter().rev().find_map(|m| metric.value(m));
    let spark: Vec<f64> = {
        let all: Vec<f64> = data.iter().filter_map(|m| metric.value(m)).collect();
        all[all.len().saturating_sub(30)..].to_vec()
    };

    card()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(metric.label()),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap_1p5()
                        .child(
                            div()
                                .text_size(px(26.))
                                .line_height(px(32.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(latest.map(|v| metric.format(v)).unwrap_or("–".into())),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(rgb(theme::TEXT_MUTED))
                                .child(metric.unit()),
                        ),
                )
                .child(
                    div()
                        .w(px(84.))
                        .h(px(30.))
                        .flex_none()
                        .child(sparkline(spark)),
                ),
        )
        .child(delta_row(metric, data))
        .children(status.map(|(label, s)| status_row(label, s)))
        .children(note.map(|n| {
            div()
                .text_size(px(12.))
                .text_color(rgb(theme::TEXT_MUTED))
                .child(n)
        }))
}

fn chart_card(metric: Metric, color: u32, data: &[Measurement], height: f32) -> Div {
    let points = metric.series(data);
    let unit = if metric.unit().is_empty() {
        String::new()
    } else {
        format!("in {}", metric.unit())
    };
    card()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(12.)).h(px(2.)).bg(rgb(color)))
                .child(card_title(metric.label(), unit)),
        )
        .child(div().h(px(height)).w_full().child(line_chart(ChartSpec {
            points,
            color,
            unit: metric.unit(),
            decimals: metric.decimals(),
        })))
}

/// Waagerechter Balken ab der Grundlinie mit dem Wert an der Spitze.
fn bar(value: Option<f64>, max: f64, color: u32, decimals: usize) -> Div {
    let wrap = div().flex_1().min_w(px(0.)).flex().items_center().gap_2();
    let Some(v) = value else {
        return wrap.text_color(rgb(theme::TEXT_MUTED)).child("–");
    };
    // 78 % der Breite bleiben für den Balken, der Rest für die Beschriftung.
    let frac = ((v / max) as f32).clamp(0.01, 1.0) * 0.78;
    wrap.child(
        div()
            .h(px(10.))
            .w(relative(frac))
            .rounded_r(px(4.))
            .bg(rgb(color)),
    )
    .child(
        div()
            .text_size(px(12.))
            .text_color(rgb(theme::TEXT_SECONDARY))
            .child(fmt_num(v, decimals)),
    )
}

fn segments_card(m: &Measurement) -> Div {
    let rows: [(&str, Option<f64>, Option<f64>); 5] = [
        ("Rechter Arm", m.fat_right_arm_pct, m.muscle_right_arm_kg),
        ("Linker Arm", m.fat_left_arm_pct, m.muscle_left_arm_kg),
        ("Rumpf", m.fat_trunk_pct, m.muscle_trunk_kg),
        ("Rechtes Bein", m.fat_right_leg_pct, m.muscle_right_leg_kg),
        ("Linkes Bein", m.fat_left_leg_pct, m.muscle_left_leg_kg),
    ];
    let max_of = |vals: Vec<Option<f64>>| vals.into_iter().flatten().fold(1e-9, f64::max);
    let fat_max = max_of(rows.iter().map(|r| r.1).collect());
    let muscle_max = max_of(rows.iter().map(|r| r.2).collect());

    let heading = |text: &'static str| {
        div()
            .flex_1()
            .text_size(px(11.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(theme::TEXT_MUTED))
            .child(text)
    };

    card()
        .flex()
        .flex_col()
        .gap_3()
        .child(card_title(
            "Körpersegmente",
            format!("Messung vom {}", fmt_date(m.measured_at)),
        ))
        .child(
            div()
                .flex()
                .gap_4()
                .child(div().w(px(96.)).flex_none())
                .child(heading("Fettanteil in %"))
                .child(heading("Muskelmasse in kg")),
        )
        .children(rows.into_iter().map(|(label, fat, muscle)| {
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(
                    div()
                        .w(px(96.))
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(label),
                )
                .child(bar(fat, fat_max, theme::FAT, 1))
                .child(bar(muscle, muscle_max, theme::MUSCLE, 1))
        }))
}

fn composition_card(m: &Measurement) -> Div {
    let fat_kg = m.fat_pct.map(|f| m.weight_kg * f / 100.0);
    let parts: Vec<(&str, f64, u32)> = [
        ("Fettmasse", fat_kg, theme::FAT),
        ("Muskelmasse", m.muscle_kg, theme::MUSCLE),
        ("Knochenmasse", m.bone_kg, theme::BONE),
    ]
    .into_iter()
    .filter_map(|(label, v, color)| Some((label, v?, color)))
    .collect();
    let total: f64 = parts.iter().map(|p| p.1).sum::<f64>().max(1e-9);
    let last = parts.len().saturating_sub(1);

    let stack =
        div()
            .flex()
            .gap(px(2.))
            .h(px(22.))
            .w_full()
            .children(parts.iter().enumerate().map(|(i, (_, v, color))| {
                let mut seg = div().h_full().bg(rgb(*color)).min_w(px(2.));
                seg.style().flex_grow = Some((*v / total) as f32);
                seg.style().flex_basis = Some(relative(0.).into());
                seg.when(i == last, |d| d.rounded_r(px(4.)))
            }));

    let legend = parts.iter().map(|(label, v, color)| {
        div()
            .flex()
            .items_center()
            .gap_2()
            .text_size(px(12.))
            .child(div().size(px(10.)).rounded(px(2.)).bg(rgb(*color)))
            .child(
                div()
                    .flex_1()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(*label),
            )
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("{} kg", fmt_num(*v, 1))),
            )
            .child(
                div()
                    .w(px(56.))
                    .flex()
                    .justify_end()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(format!("{} %", fmt_num(v / m.weight_kg * 100.0, 1))),
            )
    });

    let lean = fat_kg.map(|f| m.weight_kg - f);

    card()
        .flex()
        .flex_col()
        .gap_3()
        .child(card_title(
            "Zusammensetzung",
            format!("von {} kg Körpergewicht", fmt_num(m.weight_kg, 1)),
        ))
        .child(stack)
        .child(div().flex().flex_col().gap_2().children(legend))
        .children(lean.map(|lean| {
            div()
                .flex()
                .justify_between()
                .pt_3()
                .border_t_1()
                .border_color(rgb(theme::GRID))
                .text_size(px(12.))
                .child(
                    div()
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child("Fettfreie Masse"),
                )
                .child(
                    div()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(format!("{} kg", fmt_num(lean, 1))),
                )
        }))
}
