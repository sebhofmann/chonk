//! Kennzahlen, Zeiträume, Einordnungen und deutsche Zahlenformatierung.

use chrono::{Duration, NaiveDateTime};

use crate::i18n::{Lang, lang, t};
use crate::tanita::Measurement;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Metric {
    Weight,
    Fat,
    Muscle,
    Water,
    Bmi,
    Visceral,
    MetabolicAge,
    Dci,
    Bone,
}

/// In welche Richtung eine Veränderung als Verbesserung gilt.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Better {
    Lower,
    Higher,
    Neutral,
}

impl Metric {
    pub fn label(self) -> &'static str {
        let t = t();
        match self {
            Metric::Weight => t.weight,
            Metric::Fat => t.body_fat,
            Metric::Muscle => t.muscle_mass,
            Metric::Water => t.body_water,
            Metric::Bmi => t.bmi,
            Metric::Visceral => t.visceral_fat,
            Metric::MetabolicAge => t.metabolic_age,
            Metric::Dci => t.calorie_need,
            Metric::Bone => t.bone_mass,
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Metric::Weight | Metric::Muscle | Metric::Bone => "kg",
            Metric::Fat | Metric::Water => "%",
            Metric::Bmi | Metric::Visceral => "",
            Metric::MetabolicAge => t().years,
            Metric::Dci => "kcal",
        }
    }

    pub fn decimals(self) -> usize {
        match self {
            Metric::Visceral | Metric::MetabolicAge | Metric::Dci => 0,
            _ => 1,
        }
    }

    pub fn better(self) -> Better {
        match self {
            Metric::Fat | Metric::Visceral | Metric::MetabolicAge => Better::Lower,
            Metric::Muscle => Better::Higher,
            _ => Better::Neutral,
        }
    }

    pub fn value(self, m: &Measurement) -> Option<f64> {
        match self {
            Metric::Weight => Some(m.weight_kg),
            Metric::Fat => m.fat_pct,
            Metric::Muscle => m.muscle_kg,
            Metric::Water => m.water_pct,
            Metric::Bmi => m.bmi,
            Metric::Visceral => m.visceral_fat,
            Metric::MetabolicAge => m.metabolic_age,
            Metric::Dci => m.dci_kcal,
            Metric::Bone => m.bone_kg,
        }
    }

    /// Zeitreihe (Unix-Sekunden, Wert) für Diagramme.
    pub fn series(self, data: &[Measurement]) -> Vec<(i64, f64)> {
        data.iter()
            .filter_map(|m| Some((timestamp(m.measured_at), self.value(m)?)))
            .collect()
    }

    pub fn format(self, v: f64) -> String {
        fmt_num(v, self.decimals())
    }

    pub fn format_with_unit(self, v: f64) -> String {
        with_unit(&self.format(v), self.unit())
    }
}

pub fn with_unit(value: &str, unit: &str) -> String {
    if unit.is_empty() {
        value.to_string()
    } else {
        format!("{value} {unit}")
    }
}

pub fn timestamp(dt: NaiveDateTime) -> i64 {
    dt.and_utc().timestamp()
}

/// Zeitraum-Filter, gerechnet ab heute.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Range {
    Days30,
    Days90,
    Months6,
    Year1,
    All,
}

impl Range {
    pub const ALL: [Range; 5] = [
        Range::Days30,
        Range::Days90,
        Range::Months6,
        Range::Year1,
        Range::All,
    ];

    pub fn label(self) -> &'static str {
        let t = t();
        match self {
            Range::Days30 => t.days_30,
            Range::Days90 => t.days_90,
            Range::Months6 => t.months_6,
            Range::Year1 => t.year_1,
            Range::All => t.all,
        }
    }

    fn days(self) -> Option<i64> {
        match self {
            Range::Days30 => Some(30),
            Range::Days90 => Some(90),
            Range::Months6 => Some(183),
            Range::Year1 => Some(365),
            Range::All => None,
        }
    }

    /// Schneidet die (aufsteigend sortierten) Messungen auf den Zeitraum zu.
    pub fn filter(self, data: &[Measurement], now: NaiveDateTime) -> &[Measurement] {
        match self.days() {
            None => data,
            Some(days) => {
                let from = now - Duration::days(days);
                let start = data.partition_point(|m| m.measured_at < from);
                &data[start..]
            }
        }
    }
}

/// Bewertung eines Werts; wird immer zusammen mit einem Text angezeigt.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Good,
    Warning,
    Serious,
}

pub fn bmi_status(bmi: f64) -> (&'static str, Status) {
    if bmi < 18.5 {
        (t().underweight, Status::Warning)
    } else if bmi < 25.0 {
        (t().normal_weight, Status::Good)
    } else if bmi < 30.0 {
        (t().overweight, Status::Warning)
    } else {
        (t().obese, Status::Serious)
    }
}

/// Einordnung des Körperfettanteils nach den Tanita-Referenzbereichen
/// (Gallagher et al.), abhängig von Geschlecht und Alter.
pub fn fat_status(fat: f64, gender: Option<i64>, age: Option<i64>) -> (&'static str, Status) {
    let age = age.unwrap_or(30);
    let female = gender == Some(2);
    let (low, healthy, over) = match (female, age) {
        (false, ..=39) => (8.0, 20.0, 25.0),
        (false, 40..=59) => (11.0, 22.0, 28.0),
        (false, _) => (13.0, 25.0, 30.0),
        (true, ..=39) => (21.0, 33.0, 39.0),
        (true, 40..=59) => (23.0, 34.0, 40.0),
        (true, _) => (24.0, 36.0, 42.0),
    };
    if fat < low {
        (t().low, Status::Warning)
    } else if fat < healthy {
        (t().healthy_range, Status::Good)
    } else if fat < over {
        (t().elevated, Status::Warning)
    } else {
        (t().highly_elevated, Status::Serious)
    }
}

pub fn visceral_status(level: f64) -> (&'static str, Status) {
    if level <= 12.0 {
        (t().healthy_range, Status::Good)
    } else {
        (t().elevated, Status::Warning)
    }
}

/// Zahl in der Schreibweise der aktuellen Sprache.
pub fn fmt_num(v: f64, decimals: usize) -> String {
    fmt_num_in(v, decimals, lang())
}

/// Deutsch: Dezimalkomma und Tausenderpunkt. Englisch: umgekehrt.
pub fn fmt_num_in(v: f64, decimals: usize, lang: Lang) -> String {
    let (decimal_sep, group_sep) = match lang {
        Lang::De => (',', '.'),
        Lang::En => ('.', ','),
    };
    let s = format!("{:.*}", decimals, v.abs());
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s.as_str(), None),
    };
    let mut grouped = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i).is_multiple_of(3) {
            grouped.push(group_sep);
        }
        grouped.push(ch);
    }
    let is_zero = s.chars().all(|c| c == '0' || c == '.');
    let mut out = String::new();
    if v < 0.0 && !is_zero {
        out.push('−');
    }
    out.push_str(&grouped);
    if let Some(frac) = frac {
        out.push(decimal_sep);
        out.push_str(frac);
    }
    out
}

fn date_format(lang: Lang) -> &'static str {
    match lang {
        Lang::De => "%d.%m.%Y",
        Lang::En => "%d %b %Y",
    }
}

pub fn fmt_date(dt: NaiveDateTime) -> String {
    dt.format(date_format(lang())).to_string()
}

pub fn fmt_day(date: chrono::NaiveDate) -> String {
    date.format(date_format(lang())).to_string()
}

pub fn fmt_date_time(dt: NaiveDateTime) -> String {
    format!("{}, {}", fmt_date(dt), dt.format("%H:%M"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers_per_language() {
        assert_eq!(fmt_num_in(73.44, 1, Lang::De), "73,4");
        assert_eq!(fmt_num_in(2931.0, 0, Lang::De), "2.931");
        assert_eq!(fmt_num_in(-6.2, 1, Lang::De), "−6,2");
        assert_eq!(fmt_num_in(-0.04, 1, Lang::De), "0,0");
        assert_eq!(fmt_num_in(1234567.0, 0, Lang::De), "1.234.567");
        assert_eq!(fmt_num_in(73.44, 1, Lang::En), "73.4");
        assert_eq!(fmt_num_in(2931.0, 0, Lang::En), "2,931");
    }

    #[test]
    fn classifies_values() {
        assert_eq!(bmi_status(23.4).1, Status::Good);
        assert_eq!(bmi_status(25.4).1, Status::Warning);
        assert_eq!(fat_status(19.9, Some(1), Some(33)).1, Status::Good);
        assert_eq!(fat_status(22.7, Some(1), Some(33)).1, Status::Warning);
        assert_eq!(fat_status(22.7, Some(2), Some(33)).1, Status::Good);
    }
}
