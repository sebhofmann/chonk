//! Selbst gezeichnete Diagramme: Zeitreihen-Liniendiagramm und Sparkline.

use chrono::{DateTime, Datelike, Duration, NaiveDate};
use gpui::{
    App, BorderStyle, Bounds, Hsla, IntoElement, PathBuilder, Pixels, ShapedLine, SharedString,
    Styled, TextRun, Window, canvas, fill, point, px, quad, rgb, size,
};

use super::theme;
use crate::i18n::{Lang, lang, text};
use crate::model::{fmt_date_time, fmt_num, with_unit};

const DAY: i64 = 86_400;

pub struct ChartSpec {
    /// (Unix-Sekunden, Wert), aufsteigend sortiert.
    pub points: Vec<(i64, f64)>,
    pub color: u32,
    pub unit: &'static str,
    pub decimals: usize,
}

/// Eine Kennzahl im gemeinsamen Diagramm.
pub struct Series {
    pub label: SharedString,
    /// (Unix-Sekunden, Wert), aufsteigend sortiert.
    pub points: Vec<(i64, f64)>,
    pub color: u32,
    pub unit: &'static str,
    pub decimals: usize,
    /// Dünnere Linie für untergeordnete Reihen.
    pub thin: bool,
}

/// Mehrere Kennzahlen in einem Diagramm. Weil ihre Einheiten nicht zusammenpassen, zeigt die
/// y-Achse die Veränderung in Prozent gegenüber dem ersten Wert des Zeitraums.
pub fn multi_chart(series: Vec<Series>) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| paint_multi_chart(&series, bounds, window, cx),
    )
    .size_full()
}

pub fn line_chart(spec: ChartSpec) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| paint_line_chart(&spec, bounds, window, cx),
    )
    .size_full()
}

/// Kleine Verlaufslinie ohne Achsen; x ist der Index, nicht die Zeit.
pub fn sparkline(values: Vec<f64>) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            if values.len() < 2 {
                return;
            }
            let (x0, y0, w, h) = rect(bounds);
            let pad = 4.0;
            let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let span = (max - min).max(1e-9);
            let pos = |i: usize, v: f64| {
                let x = x0 + pad + (w - 2.0 * pad) * i as f32 / (values.len() - 1) as f32;
                let y = y0 + pad + (h - 2.0 * pad) * (1.0 - ((v - min) / span) as f32);
                point(px(x), px(y))
            };
            let mut path = PathBuilder::stroke(px(1.5));
            path.move_to(pos(0, values[0]));
            for (i, v) in values.iter().enumerate().skip(1) {
                path.line_to(pos(i, *v));
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(theme::TEXT_MUTED));
            }
            let end = pos(values.len() - 1, values[values.len() - 1]);
            dot(window, f32::from(end.x), f32::from(end.y), 2.5, theme::TEXT);
        },
    )
    .size_full()
}

fn rect(bounds: Bounds<Pixels>) -> (f32, f32, f32, f32) {
    (
        f32::from(bounds.origin.x),
        f32::from(bounds.origin.y),
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
    )
}

fn alpha(color: u32, a: f32) -> Hsla {
    Hsla::from(rgb(color)).opacity(a)
}

fn dot(window: &mut Window, x: f32, y: f32, r: f32, color: u32) {
    let b = Bounds::new(point(px(x - r), px(y - r)), size(px(2.0 * r), px(2.0 * r)));
    window.paint_quad(quad(
        b,
        px(r),
        rgb(color),
        px(0.),
        gpui::transparent_black(),
        BorderStyle::default(),
    ));
}

/// Punkt mit 2px-Ring in Flächenfarbe, damit er sich von der Linie abhebt.
fn ringed_dot(window: &mut Window, x: f32, y: f32, r: f32, color: u32) {
    dot(window, x, y, r + 2.0, theme::SURFACE);
    dot(window, x, y, r, color);
}

fn shape(window: &Window, text: &str, font_size: f32, color: u32, bold: bool) -> ShapedLine {
    let mut font = window.text_style().font();
    if bold {
        font.weight = gpui::FontWeight::SEMIBOLD;
    }
    let text: SharedString = text.to_string().into();
    let run = TextRun {
        len: text.len(),
        font,
        color: rgb(color).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(text, px(font_size), &[run], None)
}

fn draw(line: &ShapedLine, x: f32, y: f32, font_size: f32, window: &mut Window, cx: &mut App) {
    let _ = line.paint(point(px(x), px(y)), px(font_size * 1.3), window, cx);
}

fn nice_step(raw: f64) -> f64 {
    let pow = 10f64.powf(raw.log10().floor());
    let f = raw / pow;
    let nice = if f <= 1.0 {
        1.0
    } else if f <= 2.0 {
        2.0
    } else if f <= 2.5 {
        2.5
    } else if f <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * pow
}

/// Liefert (unten, oben, Schrittweite) mit runden Achsenwerten.
fn y_scale(min: f64, max: f64, target: usize) -> (f64, f64, f64) {
    let (min, max) = if (max - min).abs() < 1e-9 {
        (min - 1.0, max + 1.0)
    } else {
        (min, max)
    };
    let step = nice_step((max - min) / target as f64);
    let lo = (min / step).floor() * step;
    let hi = (max / step).ceil() * step;
    (lo, hi, step)
}

fn date_of(ts: i64) -> NaiveDate {
    DateTime::from_timestamp(ts, 0)
        .map(|d| d.date_naive())
        .unwrap_or_default()
}

fn ts_of(date: NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp()
}

/// Achsenmarken für die Zeitachse, je nach Spanne Tage, Wochen, Monate oder Jahre.
fn time_ticks(t0: i64, t1: i64, lang: Lang) -> Vec<(i64, String)> {
    let months = text(lang).months;
    let day_format = match lang {
        Lang::De => "%d.%m.",
        Lang::En => "%d %b",
    };
    let span_days = (t1 - t0) / DAY;
    let start = date_of(t0);
    let end = date_of(t1);
    let mut ticks = Vec::new();

    if span_days <= 120 {
        let step = if span_days <= 21 { 1 } else { 7 };
        let mut d = start;
        if step == 7 {
            while d.weekday() != chrono::Weekday::Mon {
                d += Duration::days(1);
            }
        }
        while d <= end {
            ticks.push((ts_of(d), d.format(day_format).to_string()));
            d += Duration::days(step);
        }
    } else {
        let month_step: u32 = if span_days <= 500 {
            1
        } else if span_days <= 1500 {
            3
        } else {
            12
        };
        let (mut y, mut m) = (start.year(), start.month());
        while let Some(d) = NaiveDate::from_ymd_opt(y, m, 1) {
            if d > end {
                break;
            }
            if d >= start && (m - 1) % month_step == 0 {
                let label = if month_step == 12 {
                    format!("{y}")
                } else if m == 1 || month_step == 3 {
                    format!("{} {:02}", months[(m - 1) as usize], y % 100)
                } else {
                    months[(m - 1) as usize].to_string()
                };
                ticks.push((ts_of(d), label));
            }
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
    }
    ticks.retain(|(ts, _)| *ts >= t0 && *ts <= t1);
    ticks
}

/// Zeichenfläche mit Skalen: rechnet Zeit und Wert in Pixel um.
struct Plot {
    x0: f32,
    y0: f32,
    w: f32,
    h: f32,
    t0: i64,
    t1: i64,
    lo: f64,
    hi: f64,
}

impl Plot {
    /// Legt die Zeichenfläche in `bounds` an; `None`, wenn der Platz nicht reicht.
    fn new(bounds: Bounds<Pixels>, left: f32, t: (i64, i64), v: (f64, f64)) -> Option<Self> {
        let (bx, by, bw, bh) = rect(bounds);
        let (w, h) = (bw - left - PLOT_RIGHT, bh - PLOT_TOP - PLOT_BOTTOM);
        if w < 40.0 || h < 40.0 {
            return None;
        }
        let (mut t0, mut t1) = t;
        if t1 - t0 < DAY {
            t0 -= DAY;
            t1 += DAY;
        }
        Some(Self {
            x0: bx + left,
            y0: by + PLOT_TOP,
            w,
            h,
            t0,
            t1,
            lo: v.0,
            hi: v.1,
        })
    }

    fn sx(&self, t: i64) -> f32 {
        self.x0 + self.w * ((t - self.t0) as f64 / (self.t1 - self.t0) as f64) as f32
    }

    fn sy(&self, v: f64) -> f32 {
        self.y0 + self.h * (1.0 - ((v - self.lo) / (self.hi - self.lo)) as f32)
    }

    fn pos(&self, p: (i64, f64)) -> gpui::Point<Pixels> {
        point(px(self.sx(p.0)), px(self.sy(p.1)))
    }

    /// Ob die Maus über der Zeichenfläche (samt Zeitachse) steht.
    fn hovered(&self, mx: f32, my: f32) -> bool {
        mx >= self.x0 - 10.0
            && mx <= self.x0 + self.w + 10.0
            && my >= self.y0
            && my <= self.y0 + self.h + PLOT_BOTTOM
    }
}

const PLOT_RIGHT: f32 = 14.0;
const PLOT_TOP: f32 = 10.0;
const PLOT_BOTTOM: f32 = 26.0;

/// Gitterlinien und y-Beschriftung; die Linie bei `base` wird als Grundlinie betont.
fn paint_grid(
    plot: &Plot,
    step: f64,
    base: f64,
    label: impl Fn(f64) -> String,
    window: &mut Window,
    cx: &mut App,
) {
    let mut v = plot.lo;
    while v <= plot.hi + step * 0.01 {
        let y = plot.sy(v).round();
        let is_base = (v - base).abs() < step * 0.01;
        window.paint_quad(fill(
            Bounds::new(point(px(plot.x0), px(y)), size(px(plot.w), px(1.))),
            rgb(if is_base { theme::AXIS } else { theme::GRID }),
        ));
        let label = shape(window, &label(v), 11.0, theme::TEXT_MUTED, false);
        let lw = f32::from(label.width);
        draw(&label, plot.x0 - 8.0 - lw, y - 7.0, 11.0, window, cx);
        v += step;
    }
}

/// x-Beschriftung, überlappende Marken werden ausgelassen.
fn paint_time_axis(plot: &Plot, window: &mut Window, cx: &mut App) {
    let mut last_right = f32::NEG_INFINITY;
    for (ts, text) in time_ticks(plot.t0, plot.t1, lang()) {
        let x = plot.sx(ts);
        let label = shape(window, &text, 11.0, theme::TEXT_MUTED, false);
        let lw = f32::from(label.width);
        let lx = (x - lw / 2.0).clamp(plot.x0, plot.x0 + plot.w - lw);
        if lx < last_right + 14.0 {
            continue;
        }
        window.paint_quad(fill(
            Bounds::new(
                point(px(x.round()), px(plot.y0 + plot.h)),
                size(px(1.), px(4.)),
            ),
            rgb(theme::AXIS),
        ));
        draw(&label, lx, plot.y0 + plot.h + 8.0, 11.0, window, cx);
        last_right = lx + lw;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Line {
    /// Linie mit Fläche bis zur Grundlinie.
    Area,
    Plain,
    Thin,
}

/// Zeichnet eine Zeitreihe als Linie.
fn paint_series(plot: &Plot, pts: &[(i64, f64)], color: u32, style: Line, window: &mut Window) {
    if pts.is_empty() {
        return;
    }
    // Messpausen: lange Lücken unterbrechen die Linie, statt Werte vorzutäuschen.
    let gap = ((plot.t1 - plot.t0) as f64 * 0.12).max(45.0 * DAY as f64) as i64;
    let mut runs: Vec<&[(i64, f64)]> = Vec::new();
    let mut run_start = 0;
    for i in 1..pts.len() {
        if pts[i].0 - pts[i - 1].0 > gap {
            runs.push(&pts[run_start..i]);
            run_start = i;
        }
    }
    runs.push(&pts[run_start..]);

    for pair in runs.windows(2) {
        let mut bridge = PathBuilder::stroke(px(1.)).dash_array(&[px(3.), px(4.)]);
        bridge.move_to(plot.pos(pair[0][pair[0].len() - 1]));
        bridge.line_to(plot.pos(pair[1][0]));
        if let Ok(path) = bridge.build() {
            window.paint_path(path, alpha(color, 0.55));
        }
    }

    let bottom = plot.y0 + plot.h;
    for run in &runs {
        if run.len() == 1 {
            dot(window, plot.sx(run[0].0), plot.sy(run[0].1), 3.0, color);
            continue;
        }
        if style == Line::Area {
            let mut area = PathBuilder::fill();
            area.move_to(point(px(plot.sx(run[0].0)), px(bottom)));
            for p in run.iter() {
                area.line_to(plot.pos(*p));
            }
            area.line_to(point(px(plot.sx(run[run.len() - 1].0)), px(bottom)));
            area.close();
            if let Ok(path) = area.build() {
                window.paint_path(path, alpha(color, 0.10));
            }
        }

        let width = if style == Line::Thin { 1.25 } else { 2.0 };
        let mut line = PathBuilder::stroke(px(width));
        line.move_to(plot.pos(run[0]));
        for p in run.iter().skip(1) {
            line.line_to(plot.pos(*p));
        }
        if let Ok(path) = line.build() {
            window.paint_path(path, rgb(color));
        }
    }

    // Letzter Wert als markierter Endpunkt
    let last = pts[pts.len() - 1];
    let r = if style == Line::Thin { 3.0 } else { 4.0 };
    ringed_dot(window, plot.sx(last.0), plot.sy(last.1), r, color);
}

/// Die Messung, deren Zeitpunkt der Maus waagerecht am nächsten liegt.
fn nearest(plot: &Plot, times: impl Iterator<Item = i64>, mx: f32) -> Option<i64> {
    times.min_by(|a, b| {
        (plot.sx(*a) - mx)
            .abs()
            .partial_cmp(&(plot.sx(*b) - mx).abs())
            .unwrap()
    })
}

fn paint_crosshair(plot: &Plot, t: i64, window: &mut Window) {
    window.paint_quad(fill(
        Bounds::new(
            point(px(plot.sx(t).round()), px(plot.y0)),
            size(px(1.), px(plot.h)),
        ),
        rgb(theme::TEXT_MUTED),
    ));
}

fn paint_tooltip_box(x: f32, y: f32, w: f32, h: f32, window: &mut Window) {
    window.paint_quad(quad(
        Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
        px(6.),
        rgb(theme::SURFACE_RAISED),
        px(1.),
        rgb(theme::AXIS),
        BorderStyle::default(),
    ));
}

fn date_label(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0)
        .map(|d| fmt_date_time(d.naive_utc()))
        .unwrap_or_default()
}

fn paint_line_chart(spec: &ChartSpec, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let pts = &spec.points;
    if pts.is_empty() {
        return;
    }
    let vmin = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let vmax = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let (lo, hi, step) = y_scale(vmin, vmax, 4);
    let span = (pts[0].0, pts[pts.len() - 1].0);
    let Some(plot) = Plot::new(bounds, 40.0, span, (lo, hi)) else {
        return;
    };

    let tick_decimals = if step.fract().abs() < 1e-9 { 0 } else { 1 };
    paint_grid(&plot, step, lo, |v| fmt_num(v, tick_decimals), window, cx);
    paint_time_axis(&plot, window, cx);
    paint_series(&plot, pts, spec.color, Line::Area, window);

    // Hover: Fadenkreuz rastet auf die nächstgelegene Messung ein
    let mouse = window.mouse_position();
    let (mx, my) = (f32::from(mouse.x), f32::from(mouse.y));
    if !plot.hovered(mx, my) {
        return;
    }
    let Some(hit) =
        nearest(&plot, pts.iter().map(|p| p.0), mx).and_then(|t| pts.iter().find(|p| p.0 == t))
    else {
        return;
    };
    let (hx, hy) = (plot.sx(hit.0), plot.sy(hit.1));
    paint_crosshair(&plot, hit.0, window);
    ringed_dot(window, hx, hy, 4.0, spec.color);

    let value = shape(
        window,
        &with_unit(&fmt_num(hit.1, spec.decimals), spec.unit),
        13.0,
        theme::TEXT,
        true,
    );
    let date = shape(
        window,
        &date_label(hit.0),
        11.0,
        theme::TEXT_SECONDARY,
        false,
    );
    let tw = f32::from(value.width).max(f32::from(date.width)) + 20.0;
    let th = 48.0;
    let mut tx = hx + 12.0;
    if tx + tw > plot.x0 + plot.w {
        tx = hx - 12.0 - tw;
    }
    let ty = (hy - th - 10.0).clamp(plot.y0, plot.y0 + plot.h - th);
    paint_tooltip_box(tx, ty, tw, th, window);
    draw(&value, tx + 10.0, ty + 8.0, 13.0, window, cx);
    draw(&date, tx + 10.0, ty + 27.0, 11.0, window, cx);
}

/// Veränderung jedes Werts in Prozent gegenüber dem ersten Wert der Reihe.
fn percent_change(points: &[(i64, f64)]) -> Vec<(i64, f64)> {
    let Some(base) = points.iter().map(|p| p.1).find(|v| v.abs() > 1e-9) else {
        return Vec::new();
    };
    points
        .iter()
        .map(|(t, v)| (*t, (v / base - 1.0) * 100.0))
        .collect()
}

fn fmt_percent(v: f64, decimals: usize) -> String {
    let number = fmt_num(v.abs(), decimals);
    if v.abs() < 0.5 * 10f64.powi(-(decimals as i32)) {
        format!("{number} %")
    } else if v > 0.0 {
        format!("+{number} %")
    } else {
        format!("−{number} %")
    }
}

fn paint_multi_chart(series: &[Series], bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let changes: Vec<Vec<(i64, f64)>> = series.iter().map(|s| percent_change(&s.points)).collect();
    let all = || changes.iter().flatten();
    let (Some(t0), Some(t1)) = (all().map(|p| p.0).min(), all().map(|p| p.0).max()) else {
        return;
    };
    // Die Nulllinie gehört immer ins Bild, sie ist der Bezugspunkt aller Reihen.
    let vmin = all().map(|p| p.1).fold(0.0, f64::min);
    let vmax = all().map(|p| p.1).fold(0.0, f64::max);
    let (lo, hi, step) = y_scale(vmin, vmax, 4);
    let Some(plot) = Plot::new(bounds, 52.0, (t0, t1), (lo, hi)) else {
        return;
    };

    let tick_decimals = if step.fract().abs() < 1e-9 { 0 } else { 1 };
    paint_grid(
        &plot,
        step,
        0.0,
        |v| fmt_percent(v, tick_decimals),
        window,
        cx,
    );
    paint_time_axis(&plot, window, cx);
    for (s, change) in series.iter().zip(&changes) {
        let style = if s.thin { Line::Thin } else { Line::Plain };
        paint_series(&plot, change, s.color, style, window);
    }

    // Hover: Fadenkreuz auf der nächstgelegenen Messung, Tooltip mit den echten Werten
    let mouse = window.mouse_position();
    let (mx, my) = (f32::from(mouse.x), f32::from(mouse.y));
    if !plot.hovered(mx, my) {
        return;
    }
    let Some(hit) = nearest(&plot, all().map(|p| p.0), mx) else {
        return;
    };
    let hx = plot.sx(hit);
    paint_crosshair(&plot, hit, window);

    let mut rows = Vec::new();
    for (s, change) in series.iter().zip(&changes) {
        let Ok(i) = change.binary_search_by_key(&hit, |p| p.0) else {
            continue;
        };
        ringed_dot(window, hx, plot.sy(change[i].1), 3.5, s.color);
        let label = shape(window, &s.label, 12.0, theme::TEXT_SECONDARY, false);
        let value = shape(
            window,
            &with_unit(&fmt_num(s.points[i].1, s.decimals), s.unit),
            12.0,
            theme::TEXT,
            true,
        );
        rows.push((s.color, label, value));
    }

    let date = shape(window, &date_label(hit), 11.0, theme::TEXT_SECONDARY, false);
    let label_w = rows
        .iter()
        .map(|r| f32::from(r.1.width))
        .fold(0.0, f32::max);
    let value_w = rows
        .iter()
        .map(|r| f32::from(r.2.width))
        .fold(0.0, f32::max);
    let (swatch, row_h) = (16.0, 18.0);
    let tw = f32::from(date.width).max(swatch + label_w + 16.0 + value_w) + 20.0;
    let th = 30.0 + rows.len() as f32 * row_h + 6.0;
    let mut tx = hx + 12.0;
    if tx + tw > plot.x0 + plot.w {
        tx = hx - 12.0 - tw;
    }
    let ty = (my - th / 2.0).clamp(plot.y0, (plot.y0 + plot.h - th).max(plot.y0));
    paint_tooltip_box(tx, ty, tw, th, window);
    draw(&date, tx + 10.0, ty + 8.0, 11.0, window, cx);
    for (i, (color, label, value)) in rows.iter().enumerate() {
        let y = ty + 30.0 + i as f32 * row_h;
        dot(window, tx + 14.0, y + 8.0, 4.0, *color);
        draw(label, tx + 10.0 + swatch, y, 12.0, window, cx);
        draw(
            value,
            tx + tw - 10.0 - f32::from(value.width),
            y,
            12.0,
            window,
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn y_scale_uses_round_numbers() {
        let (lo, hi, step) = y_scale(72.4, 79.1, 4);
        assert_eq!((lo, hi, step), (72.0, 80.0, 2.0));
        let (lo, hi, _) = y_scale(5.0, 5.0, 4);
        assert!(lo < 5.0 && hi > 5.0);
    }

    #[test]
    fn percent_change_is_relative_to_first_value() {
        let change = percent_change(&[(0, 80.0), (1, 84.0), (2, 76.0)]);
        assert_eq!(change[0], (0, 0.0));
        assert!((change[1].1 - 5.0).abs() < 1e-9);
        assert!((change[2].1 + 5.0).abs() < 1e-9);
        assert!(percent_change(&[]).is_empty());
        assert_eq!(fmt_percent(5.0, 0), "+5 %");
        assert_eq!(fmt_percent(-2.0, 0), "−2 %");
        assert_eq!(fmt_percent(0.0, 0), "0 %");
    }

    #[test]
    fn time_ticks_adapt_to_span() {
        let t = |y, m, d| ts_of(NaiveDate::from_ymd_opt(y, m, d).unwrap());
        let months = time_ticks(t(2024, 6, 26), t(2025, 1, 24), Lang::De);
        assert_eq!(months.first().unwrap().1, "Jul");
        assert_eq!(months.last().unwrap().1, "Jan 25");
        let quarters = time_ticks(t(2024, 6, 26), t(2026, 9, 21), Lang::De);
        assert_eq!(quarters.first().unwrap().1, "Jul 24");
        assert_eq!(quarters.len(), 9);
        let weeks = time_ticks(t(2026, 8, 1), t(2026, 9, 21), Lang::De);
        assert_eq!(weeks.first().unwrap().1, "03.08.");
        let weeks = time_ticks(t(2026, 8, 1), t(2026, 9, 21), Lang::En);
        assert_eq!(weeks.first().unwrap().1, "03 Aug");
        let months = time_ticks(t(2025, 9, 1), t(2026, 6, 1), Lang::En);
        assert_eq!(months[1].1, "Oct");
    }
}
