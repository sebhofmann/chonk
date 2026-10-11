//! Selbst gezeichnete Diagramme: Zeitreihen-Liniendiagramm und Sparkline.

use chrono::{DateTime, Datelike, Duration, NaiveDate};
use gpui::{
    App, BorderStyle, Bounds, Hsla, IntoElement, PathBuilder, Pixels, ShapedLine, SharedString,
    Styled, TextRun, Window, canvas, fill, point, px, quad, rgb, size,
};

use super::theme;
use crate::model::{fmt_num, with_unit};

const DAY: i64 = 86_400;

pub struct ChartSpec {
    /// (Unix-Sekunden, Wert), aufsteigend sortiert.
    pub points: Vec<(i64, f64)>,
    pub color: u32,
    pub unit: &'static str,
    pub decimals: usize,
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

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
];

fn date_of(ts: i64) -> NaiveDate {
    DateTime::from_timestamp(ts, 0)
        .map(|d| d.date_naive())
        .unwrap_or_default()
}

fn ts_of(date: NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp()
}

/// Achsenmarken für die Zeitachse, je nach Spanne Tage, Wochen, Monate oder Jahre.
fn time_ticks(t0: i64, t1: i64) -> Vec<(i64, String)> {
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
            ticks.push((ts_of(d), d.format("%d.%m.").to_string()));
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
        loop {
            let Some(d) = NaiveDate::from_ymd_opt(y, m, 1) else {
                break;
            };
            if d > end {
                break;
            }
            if d >= start && (m - 1) % month_step == 0 {
                let label = if month_step == 12 {
                    format!("{y}")
                } else if m == 1 || month_step == 3 {
                    format!("{} {:02}", MONTHS[(m - 1) as usize], y % 100)
                } else {
                    MONTHS[(m - 1) as usize].to_string()
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

fn paint_line_chart(spec: &ChartSpec, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
    let pts = &spec.points;
    if pts.is_empty() {
        return;
    }
    let (bx, by, bw, bh) = rect(bounds);
    let (left, right, top, bottom) = (40.0f32, 14.0f32, 10.0f32, 26.0f32);
    let (px0, py0) = (bx + left, by + top);
    let (pw, ph) = (bw - left - right, bh - top - bottom);
    if pw < 40.0 || ph < 40.0 {
        return;
    }

    // Skalen
    let vmin = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let vmax = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let (lo, hi, step) = y_scale(vmin, vmax, 4);
    let (mut t0, mut t1) = (pts[0].0, pts[pts.len() - 1].0);
    if t1 - t0 < DAY {
        t0 -= DAY;
        t1 += DAY;
    }
    let sx = |t: i64| px0 + pw * ((t - t0) as f64 / (t1 - t0) as f64) as f32;
    let sy = |v: f64| py0 + ph * (1.0 - ((v - lo) / (hi - lo)) as f32);

    // Gitterlinien und y-Beschriftung
    let tick_decimals = if step.fract().abs() < 1e-9 { 0 } else { 1 };
    let mut v = lo;
    while v <= hi + step * 0.01 {
        let y = sy(v).round();
        let is_base = (v - lo).abs() < step * 0.01;
        window.paint_quad(fill(
            Bounds::new(point(px(px0), px(y)), size(px(pw), px(1.))),
            rgb(if is_base { theme::AXIS } else { theme::GRID }),
        ));
        let label = shape(window, &fmt_num(v, tick_decimals), 11.0, theme::TEXT_MUTED, false);
        let lw = f32::from(label.width);
        draw(&label, px0 - 8.0 - lw, y - 7.0, 11.0, window, cx);
        v += step;
    }

    // x-Beschriftung, überlappende Marken werden ausgelassen
    let mut last_right = f32::NEG_INFINITY;
    for (ts, text) in time_ticks(t0, t1) {
        let x = sx(ts);
        let label = shape(window, &text, 11.0, theme::TEXT_MUTED, false);
        let lw = f32::from(label.width);
        let lx = (x - lw / 2.0).clamp(px0, px0 + pw - lw);
        if lx < last_right + 14.0 {
            continue;
        }
        window.paint_quad(fill(
            Bounds::new(point(px(x.round()), px(py0 + ph)), size(px(1.), px(4.))),
            rgb(theme::AXIS),
        ));
        draw(&label, lx, py0 + ph + 8.0, 11.0, window, cx);
        last_right = lx + lw;
    }

    // Messpausen: lange Lücken unterbrechen die Linie, statt Werte vorzutäuschen.
    let gap = ((t1 - t0) as f64 * 0.12).max(45.0 * DAY as f64) as i64;
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
        let a = pair[0][pair[0].len() - 1];
        let b = pair[1][0];
        let mut bridge = PathBuilder::stroke(px(1.)).dash_array(&[px(3.), px(4.)]);
        bridge.move_to(point(px(sx(a.0)), px(sy(a.1))));
        bridge.line_to(point(px(sx(b.0)), px(sy(b.1))));
        if let Ok(path) = bridge.build() {
            window.paint_path(path, alpha(spec.color, 0.55));
        }
    }

    for run in &runs {
        if run.len() == 1 {
            dot(window, sx(run[0].0), sy(run[0].1), 3.0, spec.color);
            continue;
        }
        let mut area = PathBuilder::fill();
        area.move_to(point(px(sx(run[0].0)), px(py0 + ph)));
        for p in run.iter() {
            area.line_to(point(px(sx(p.0)), px(sy(p.1))));
        }
        area.line_to(point(px(sx(run[run.len() - 1].0)), px(py0 + ph)));
        area.close();
        if let Ok(path) = area.build() {
            window.paint_path(path, alpha(spec.color, 0.10));
        }

        let mut line = PathBuilder::stroke(px(2.));
        line.move_to(point(px(sx(run[0].0)), px(sy(run[0].1))));
        for p in run.iter().skip(1) {
            line.line_to(point(px(sx(p.0)), px(sy(p.1))));
        }
        if let Ok(path) = line.build() {
            window.paint_path(path, rgb(spec.color));
        }
    }

    // Letzter Wert als markierter Endpunkt
    let last = pts[pts.len() - 1];
    ringed_dot(window, sx(last.0), sy(last.1), 4.0, spec.color);

    // Hover: Fadenkreuz rastet auf die nächstgelegene Messung ein
    let mouse = window.mouse_position();
    let (mx, my) = (f32::from(mouse.x), f32::from(mouse.y));
    if mx < px0 - 10.0 || mx > px0 + pw + 10.0 || my < py0 || my > py0 + ph + bottom {
        return;
    }
    let Some(hit) = pts.iter().min_by(|a, b| {
        (sx(a.0) - mx)
            .abs()
            .partial_cmp(&(sx(b.0) - mx).abs())
            .unwrap()
    }) else {
        return;
    };
    let (hx, hy) = (sx(hit.0), sy(hit.1));
    window.paint_quad(fill(
        Bounds::new(point(px(hx.round()), px(py0)), size(px(1.), px(ph))),
        rgb(theme::TEXT_MUTED),
    ));
    ringed_dot(window, hx, hy, 4.0, spec.color);

    let value = shape(
        window,
        &with_unit(&fmt_num(hit.1, spec.decimals), spec.unit),
        13.0,
        theme::TEXT,
        true,
    );
    let when = DateTime::from_timestamp(hit.0, 0)
        .map(|d| d.format("%d.%m.%Y, %H:%M").to_string())
        .unwrap_or_default();
    let date = shape(window, &when, 11.0, theme::TEXT_SECONDARY, false);
    let tw = f32::from(value.width).max(f32::from(date.width)) + 20.0;
    let th = 48.0;
    let mut tx = hx + 12.0;
    if tx + tw > px0 + pw {
        tx = hx - 12.0 - tw;
    }
    let ty = (hy - th - 10.0).clamp(py0, py0 + ph - th);
    window.paint_quad(quad(
        Bounds::new(point(px(tx), px(ty)), size(px(tw), px(th))),
        px(6.),
        rgb(theme::SURFACE_RAISED),
        px(1.),
        rgb(theme::AXIS),
        BorderStyle::default(),
    ));
    draw(&value, tx + 10.0, ty + 8.0, 13.0, window, cx);
    draw(&date, tx + 10.0, ty + 27.0, 11.0, window, cx);
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
    fn time_ticks_adapt_to_span() {
        let t = |y, m, d| ts_of(NaiveDate::from_ymd_opt(y, m, d).unwrap());
        let months = time_ticks(t(2024, 6, 26), t(2025, 1, 24));
        assert_eq!(months.first().unwrap().1, "Jul");
        assert_eq!(months.last().unwrap().1, "Jan 25");
        let quarters = time_ticks(t(2024, 6, 26), t(2026, 9, 21));
        assert_eq!(quarters.first().unwrap().1, "Jul 24");
        assert_eq!(quarters.len(), 9);
        let weeks = time_ticks(t(2026, 8, 1), t(2026, 9, 21));
        assert_eq!(weeks.first().unwrap().1, "03.08.");
    }
}
