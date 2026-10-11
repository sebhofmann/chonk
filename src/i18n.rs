//! Übersetzungen der Oberfläche (Deutsch und Englisch).
//!
//! Feste Texte stehen als Felder in [`Text`], damit der Compiler prüft, dass
//! jede Sprache jeden Text hat. Texte mit Platzhaltern sind Methoden.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    De,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::De, Lang::En];

    /// Kürzel für Umschalter und Einstellungen.
    pub fn code(self) -> &'static str {
        match self {
            Lang::De => "de",
            Lang::En => "en",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        let code = code.to_ascii_lowercase();
        if code.starts_with("de") {
            Some(Lang::De)
        } else if code.starts_with("en") {
            Some(Lang::En)
        } else {
            None
        }
    }

    /// Sprache des Betriebssystems; alles außer Deutsch wird Englisch.
    pub fn system() -> Lang {
        sys_locale::get_locale()
            .and_then(|l| Lang::from_code(&l))
            .unwrap_or(Lang::En)
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::En,
        _ => Lang::De,
    }
}

/// Texte der aktuell gewählten Sprache.
pub fn t() -> &'static Text {
    text(lang())
}

pub fn text(lang: Lang) -> &'static Text {
    match lang {
        Lang::De => &DE,
        Lang::En => &EN,
    }
}

pub struct Text {
    pub lang: Lang,
    pub quit: &'static str,
    // Kopf
    pub archive_subtitle: &'static str,
    pub no_card: &'static str,
    pub import_now: &'static str,
    pub no_card_found: &'static str,
    pub rename: &'static str,
    // Ansichten und Zeiträume
    pub overview: &'static str,
    pub measurements: &'static str,
    pub days_30: &'static str,
    pub days_90: &'static str,
    pub months_6: &'static str,
    pub year_1: &'static str,
    pub all: &'static str,
    // Kennzahlen
    pub weight: &'static str,
    pub body_fat: &'static str,
    pub muscle_mass: &'static str,
    pub body_water: &'static str,
    pub bmi: &'static str,
    pub visceral_fat: &'static str,
    pub metabolic_age: &'static str,
    pub calorie_need: &'static str,
    pub bone_mass: &'static str,
    pub years: &'static str,
    // Einordnungen
    pub underweight: &'static str,
    pub normal_weight: &'static str,
    pub overweight: &'static str,
    pub obese: &'static str,
    pub low: &'static str,
    pub healthy_range: &'static str,
    pub elevated: &'static str,
    pub highly_elevated: &'static str,
    pub age_matches: &'static str,
    // Übersicht
    pub current_weight: &'static str,
    pub lowest: &'static str,
    pub highest: &'static str,
    pub average: &'static str,
    pub only_measurement: &'static str,
    pub no_data_title: &'static str,
    pub no_data_hint: &'static str,
    pub body_segments: &'static str,
    pub fat_in_percent: &'static str,
    pub muscle_in_kg: &'static str,
    pub right_arm: &'static str,
    pub left_arm: &'static str,
    pub trunk: &'static str,
    pub right_leg: &'static str,
    pub left_leg: &'static str,
    pub composition: &'static str,
    pub fat_mass: &'static str,
    pub fat_free_mass: &'static str,
    // Tabelle
    pub col_date: &'static str,
    pub col_time: &'static str,
    pub col_weight: &'static str,
    pub col_fat: &'static str,
    pub col_muscle: &'static str,
    pub col_water: &'static str,
    pub col_bone: &'static str,
    pub col_metabolic_age: &'static str,
    pub col_calories: &'static str,
    /// Monatskürzel für Achsenbeschriftungen.
    pub months: [&'static str; 12],
}

pub static DE: Text = Text {
    lang: Lang::De,
    quit: "Chonk beenden",
    archive_subtitle: "Körperanalyse-Archiv",
    no_card: "Keine SD-Karte eingelegt",
    import_now: "Jetzt importieren",
    no_card_found: "Keine SD-Karte gefunden.",
    rename: "umbenennen",
    overview: "Übersicht",
    measurements: "Messungen",
    days_30: "30 Tage",
    days_90: "90 Tage",
    months_6: "6 Monate",
    year_1: "1 Jahr",
    all: "Alles",
    weight: "Gewicht",
    body_fat: "Körperfett",
    muscle_mass: "Muskelmasse",
    body_water: "Körperwasser",
    bmi: "BMI",
    visceral_fat: "Viszeralfett",
    metabolic_age: "Stoffwechselalter",
    calorie_need: "Kalorienbedarf",
    bone_mass: "Knochenmasse",
    years: "Jahre",
    underweight: "Untergewicht",
    normal_weight: "Normalgewicht",
    overweight: "Übergewicht",
    obese: "Adipositas",
    low: "niedrig",
    healthy_range: "gesunder Bereich",
    elevated: "erhöht",
    highly_elevated: "stark erhöht",
    age_matches: "entspricht dem tatsächlichen Alter",
    current_weight: "Aktuelles Gewicht",
    lowest: "Tiefstwert",
    highest: "Höchstwert",
    average: "Durchschnitt",
    only_measurement: "einzige Messung im Zeitraum",
    no_data_title: "Noch keine Messungen archiviert",
    no_data_hint: "Lege die SD-Karte der Waage ein. Die Daten werden automatisch importiert.",
    body_segments: "Körpersegmente",
    fat_in_percent: "Fettanteil in %",
    muscle_in_kg: "Muskelmasse in kg",
    right_arm: "Rechter Arm",
    left_arm: "Linker Arm",
    trunk: "Rumpf",
    right_leg: "Rechtes Bein",
    left_leg: "Linkes Bein",
    composition: "Zusammensetzung",
    fat_mass: "Fettmasse",
    fat_free_mass: "Fettfreie Masse",
    col_date: "Datum",
    col_time: "Uhrzeit",
    col_weight: "Gewicht kg",
    col_fat: "Fett %",
    col_muscle: "Muskeln kg",
    col_water: "Wasser %",
    col_bone: "Knochen kg",
    col_metabolic_age: "Stoffw.-Alter",
    col_calories: "Bedarf kcal",
    months: [
        "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
    ],
};

pub static EN: Text = Text {
    lang: Lang::En,
    quit: "Quit Chonk",
    archive_subtitle: "Body composition archive",
    no_card: "No SD card inserted",
    import_now: "Import now",
    no_card_found: "No SD card found.",
    rename: "rename",
    overview: "Overview",
    measurements: "Measurements",
    days_30: "30 days",
    days_90: "90 days",
    months_6: "6 months",
    year_1: "1 year",
    all: "All",
    weight: "Weight",
    body_fat: "Body fat",
    muscle_mass: "Muscle mass",
    body_water: "Body water",
    bmi: "BMI",
    visceral_fat: "Visceral fat",
    metabolic_age: "Metabolic age",
    calorie_need: "Calorie need",
    bone_mass: "Bone mass",
    years: "years",
    underweight: "Underweight",
    normal_weight: "Normal weight",
    overweight: "Overweight",
    obese: "Obese",
    low: "low",
    healthy_range: "healthy range",
    elevated: "elevated",
    highly_elevated: "highly elevated",
    age_matches: "matches your actual age",
    current_weight: "Current weight",
    lowest: "Lowest",
    highest: "Highest",
    average: "Average",
    only_measurement: "only measurement in this period",
    no_data_title: "No measurements archived yet",
    no_data_hint: "Insert the scale's SD card. The data is imported automatically.",
    body_segments: "Body segments",
    fat_in_percent: "Body fat in %",
    muscle_in_kg: "Muscle mass in kg",
    right_arm: "Right arm",
    left_arm: "Left arm",
    trunk: "Trunk",
    right_leg: "Right leg",
    left_leg: "Left leg",
    composition: "Composition",
    fat_mass: "Fat mass",
    fat_free_mass: "Fat-free mass",
    col_date: "Date",
    col_time: "Time",
    col_weight: "Weight kg",
    col_fat: "Fat %",
    col_muscle: "Muscle kg",
    col_water: "Water %",
    col_bone: "Bone kg",
    col_metabolic_age: "Metab. age",
    col_calories: "Need kcal",
    months: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
};

// Texte mit Platzhaltern. `match self.lang` hält beide Sprachen nebeneinander.
impl Text {
    fn pick(&self, de: String, en: String) -> String {
        match self.lang {
            Lang::De => de,
            Lang::En => en,
        }
    }

    pub fn born(&self, date: &str) -> String {
        self.pick(format!("geboren {date}"), format!("born {date}"))
    }

    pub fn card_connected(&self, volume: &str) -> String {
        self.pick(
            format!("SD-Karte „{volume}“ verbunden"),
            format!("SD card “{volume}” connected"),
        )
    }

    pub fn import_result(&self, seen: usize, new: usize) -> String {
        match (self.lang, new) {
            (Lang::De, 0) => format!("{seen} Messungen gelesen, alle bereits archiviert"),
            (Lang::De, n) => format!("{seen} Messungen gelesen, {n} neu archiviert"),
            (Lang::En, 0) => format!("{seen} measurements read, all already archived"),
            (Lang::En, n) => format!("{seen} measurements read, {n} newly archived"),
        }
    }

    pub fn lines_skipped(&self, n: usize) -> String {
        match (self.lang, n) {
            (Lang::De, 1) => "1 Zeile übersprungen".into(),
            (Lang::De, n) => format!("{n} Zeilen übersprungen"),
            (Lang::En, 1) => "1 line skipped".into(),
            (Lang::En, n) => format!("{n} lines skipped"),
        }
    }

    pub fn import_failed(&self, error: &str) -> String {
        self.pick(
            format!("Import fehlgeschlagen: {error}"),
            format!("Import failed: {error}"),
        )
    }

    pub fn archive_unreadable(&self, error: &str) -> String {
        self.pick(
            format!("Archiv nicht lesbar: {error}"),
            format!("Could not read the archive: {error}"),
        )
    }

    pub fn not_saved(&self, error: &str) -> String {
        self.pick(
            format!("Nicht gespeichert: {error}"),
            format!("Could not save: {error}"),
        )
    }

    pub fn profile(&self, slot: u8) -> String {
        self.pick(format!("Profil {slot}"), format!("Profile {slot}"))
    }

    pub fn name_for_profile(&self, slot: u8) -> String {
        self.pick(
            format!("Name für Profil {slot}"),
            format!("Name for profile {slot}"),
        )
    }

    pub fn archive_path(&self, path: &str) -> String {
        self.pick(format!("Archiv: {path}"), format!("Archive: {path}"))
    }

    pub fn measurement_count(&self, n: usize) -> String {
        match (self.lang, n) {
            (Lang::De, 1) => "1 Messung".into(),
            (Lang::De, n) => format!("{n} Messungen"),
            (Lang::En, 1) => "1 measurement".into(),
            (Lang::En, n) => format!("{n} measurements"),
        }
    }

    pub fn last_import(&self, when: &str) -> String {
        self.pick(
            format!("letzter Import {when}"),
            format!("last import {when}"),
        )
    }

    pub fn years_younger(&self, years: &str) -> String {
        self.pick(
            format!("{years} Jahre jünger als tatsächlich"),
            format!("{years} years younger than your actual age"),
        )
    }

    pub fn years_older(&self, years: &str) -> String {
        self.pick(
            format!("{years} Jahre älter als tatsächlich"),
            format!("{years} years older than your actual age"),
        )
    }

    pub fn no_data_in_range(&self, range: &str) -> String {
        self.pick(
            format!("Keine Messungen im Zeitraum „{range}“"),
            format!("No measurements in the period “{range}”"),
        )
    }

    pub fn last_measurement_hint(&self, date: &str) -> String {
        self.pick(
            format!("Die letzte Messung ist vom {date}. Wähle oben einen längeren Zeitraum."),
            format!("The last measurement is from {date}. Pick a longer period above."),
        )
    }

    pub fn measured_on(&self, when: &str) -> String {
        self.pick(format!("gemessen am {when}"), format!("measured on {when}"))
    }

    pub fn measurement_from(&self, date: &str) -> String {
        self.pick(
            format!("Messung vom {date}"),
            format!("Measurement from {date}"),
        )
    }

    pub fn unchanged_since(&self, date: &str) -> String {
        self.pick(
            format!("unverändert seit {date}"),
            format!("unchanged since {date}"),
        )
    }

    pub fn change_since(&self, change: &str, date: &str) -> String {
        self.pick(
            format!("{change} seit {date}"),
            format!("{change} since {date}"),
        )
    }

    pub fn in_unit(&self, unit: &str) -> String {
        format!("in {unit}")
    }

    pub fn of_body_weight(&self, weight: &str) -> String {
        self.pick(
            format!("von {weight} kg Körpergewicht"),
            format!("of {weight} kg body weight"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_locale_codes() {
        assert_eq!(Lang::from_code("de-DE"), Some(Lang::De));
        assert_eq!(Lang::from_code("de_AT.UTF-8"), Some(Lang::De));
        assert_eq!(Lang::from_code("en-US"), Some(Lang::En));
        assert_eq!(Lang::from_code("fr-FR"), None);
        for lang in Lang::ALL {
            assert_eq!(Lang::from_code(lang.code()), Some(lang));
            assert_eq!(text(lang).lang, lang);
        }
    }

    #[test]
    fn plural_forms() {
        assert_eq!(text(Lang::En).measurement_count(1), "1 measurement");
        assert_eq!(text(Lang::En).measurement_count(59), "59 measurements");
        assert_eq!(text(Lang::De).measurement_count(1), "1 Messung");
        assert_eq!(
            text(Lang::De).import_result(59, 0),
            "59 Messungen gelesen, alle bereits archiviert"
        );
    }
}
