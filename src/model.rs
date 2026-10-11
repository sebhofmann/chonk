//! Kennzahlen, Zeiträume, Einordnungen und deutsche Zahlenformatierung.

use chrono::{Duration, NaiveDateTime};

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
        match self {
            Metric::Weight => "Gewicht",
            Metric::Fat => "Körperfett",
            Metric::Muscle => "Muskelmasse",
            Metric::Water => "Körperwasser",
            Metric::Bmi => "BMI",
            Metric::Visceral => "Viszeralfett",
            Metric::MetabolicAge => "Stoffwechselalter",
            Metric::Dci => "Kalorienbedarf",
            Metric::Bone => "Knochenmasse",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Metric::Weight | Metric::Muscle | Metric::Bone => "kg",
            Metric::Fat | Metric::Water => "%",
            Metric::Bmi | Metric::Visceral => "",
            Metric::MetabolicAge => "Jahre",
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
        match self {
            Range::Days30 => "30 Tage",
            Range::Days90 => "90 Tage",
            Range::Months6 => "6 Monate",
            Range::Year1 => "1 Jahr",
            Range::All => "Alles",
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
        ("Untergewicht", Status::Warning)
    } else if bmi < 25.0 {
        ("Normalgewicht", Status::Good)
    } else if bmi < 30.0 {
        ("Übergewicht", Status::Warning)
    } else {
        ("Adipositas", Status::Serious)
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
        ("niedrig", Status::Warning)
    } else if fat < healthy {
        ("gesunder Bereich", Status::Good)
    } else if fat < over {
        ("erhöht", Status::Warning)
    } else {
        ("stark erhöht", Status::Serious)
    }
}

pub fn visceral_status(level: f64) -> (&'static str, Status) {
    if level <= 12.0 {
        ("gesunder Bereich", Status::Good)
    } else {
        ("erhöht", Status::Warning)
    }
}

/// Zahl mit deutschem Dezimalkomma und Tausenderpunkt.
pub fn fmt_num(v: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, v.abs());
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s.as_str(), None),
    };
    let mut grouped = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push('.');
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
        out.push(',');
        out.push_str(frac);
    }
    out
}

pub fn fmt_date(dt: NaiveDateTime) -> String {
    dt.format("%d.%m.%Y").to_string()
}

pub fn fmt_date_time(dt: NaiveDateTime) -> String {
    dt.format("%d.%m.%Y, %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_german_numbers() {
        assert_eq!(fmt_num(73.44, 1), "73,4");
        assert_eq!(fmt_num(2931.0, 0), "2.931");
        assert_eq!(fmt_num(-6.2, 1), "−6,2");
        assert_eq!(fmt_num(-0.04, 1), "0,0");
        assert_eq!(fmt_num(1234567.0, 0), "1.234.567");
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
