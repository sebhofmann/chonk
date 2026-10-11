//! Parser für das CSV-Format der Tanita-Körperanalysewaagen (BC-601 & Co.).
//!
//! Jede Zeile ist eine flache Liste aus Schlüssel-Wert-Paaren:
//!
//! ```text
//! {0,16,~0,2,~1,2,~2,3,~3,4,MO,"BC-601",DT,"01/02/2024",Ti,"08:00:00",...,Wk,80.0,...,CS,BC
//! ```
//!
//! `CS` ist eine Prüfsumme: die Summe aller Bytes vor `CS` modulo 256 (hex).

use std::collections::HashMap;

use anyhow::{Context, Result, anyhow, bail};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

/// Eine einzelne Messung der Waage.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// Profil-Slot auf der Waage (1–8), ergibt sich aus dem Dateinamen DATAn.CSV.
    pub profile: u8,
    pub measured_at: NaiveDateTime,
    pub model: String,
    /// 0 = normal, 2 = Athlet
    pub body_type: Option<i64>,
    /// 1 = männlich, 2 = weiblich
    pub gender: Option<i64>,
    pub age: Option<i64>,
    pub height_cm: Option<f64>,
    pub activity_level: Option<i64>,
    pub weight_kg: f64,
    pub bmi: Option<f64>,
    pub fat_pct: Option<f64>,
    pub fat_right_arm_pct: Option<f64>,
    pub fat_left_arm_pct: Option<f64>,
    pub fat_right_leg_pct: Option<f64>,
    pub fat_left_leg_pct: Option<f64>,
    pub fat_trunk_pct: Option<f64>,
    pub muscle_kg: Option<f64>,
    pub muscle_right_arm_kg: Option<f64>,
    pub muscle_left_arm_kg: Option<f64>,
    pub muscle_right_leg_kg: Option<f64>,
    pub muscle_left_leg_kg: Option<f64>,
    pub muscle_trunk_kg: Option<f64>,
    pub bone_kg: Option<f64>,
    pub visceral_fat: Option<f64>,
    /// Täglicher Kalorienbedarf (DCI) in kcal.
    pub dci_kcal: Option<f64>,
    pub metabolic_age: Option<f64>,
    pub water_pct: Option<f64>,
    /// Die unveränderte Originalzeile, damit nichts verloren geht.
    pub raw: String,
}

/// Ein auf der Waage hinterlegtes Profil (SYSTEM/PROFn.CSV).
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub slot: u8,
    pub model: String,
    pub birth_date: Option<NaiveDate>,
    pub gender: Option<i64>,
    pub height_cm: Option<f64>,
    pub activity_level: Option<i64>,
    pub body_type: Option<i64>,
}

fn tokenize(line: &str) -> Result<HashMap<&str, &str>> {
    let line = line.trim().trim_start_matches('{');
    let tokens: Vec<&str> = line.split(',').map(|t| t.trim().trim_matches('"')).collect();
    if tokens.len() % 2 != 0 {
        bail!("ungerade Anzahl an Feldern ({})", tokens.len());
    }
    Ok(tokens.chunks(2).map(|c| (c[0], c[1])).collect())
}

/// Prüft die `CS`-Prüfsumme. Zeilen ohne `CS` gelten als gültig.
pub fn checksum_ok(line: &str) -> bool {
    let line = line.trim();
    let Some(idx) = line.rfind(",CS,") else {
        return true;
    };
    let Ok(expected) = u8::from_str_radix(line[idx + 4..].trim(), 16) else {
        return false;
    };
    // Die Summe läuft über alles vor "CS", inklusive des Kommas davor.
    let sum = line.as_bytes()[..idx + 1]
        .iter()
        .fold(0u8, |acc, b| acc.wrapping_add(*b));
    sum == expected
}

fn num(map: &HashMap<&str, &str>, key: &str) -> Option<f64> {
    map.get(key).and_then(|v| v.parse::<f64>().ok())
}

fn int(map: &HashMap<&str, &str>, key: &str) -> Option<i64> {
    map.get(key).and_then(|v| v.parse::<i64>().ok())
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%d/%m/%Y").with_context(|| format!("ungültiges Datum {s:?}"))
}

/// Parst eine Zeile aus DATAn.CSV.
pub fn parse_measurement(line: &str, profile: u8) -> Result<Measurement> {
    if !checksum_ok(line) {
        bail!("Prüfsumme stimmt nicht");
    }
    let map = tokenize(line)?;
    let date = parse_date(map.get("DT").ok_or_else(|| anyhow!("Datum (DT) fehlt"))?)?;
    let time_str = map.get("Ti").copied().unwrap_or("00:00:00");
    let time = NaiveTime::parse_from_str(time_str, "%H:%M:%S")
        .with_context(|| format!("ungültige Uhrzeit {time_str:?}"))?;
    let weight_kg = num(&map, "Wk").ok_or_else(|| anyhow!("Gewicht (Wk) fehlt"))?;

    Ok(Measurement {
        profile,
        measured_at: NaiveDateTime::new(date, time),
        model: map.get("MO").copied().unwrap_or("").to_string(),
        body_type: int(&map, "Bt"),
        gender: int(&map, "GE"),
        age: int(&map, "AG"),
        height_cm: num(&map, "Hm"),
        activity_level: int(&map, "AL"),
        weight_kg,
        bmi: num(&map, "MI"),
        fat_pct: num(&map, "FW"),
        fat_right_arm_pct: num(&map, "Fr"),
        fat_left_arm_pct: num(&map, "Fl"),
        fat_right_leg_pct: num(&map, "FR"),
        fat_left_leg_pct: num(&map, "FL"),
        fat_trunk_pct: num(&map, "FT"),
        muscle_kg: num(&map, "mW"),
        muscle_right_arm_kg: num(&map, "mr"),
        muscle_left_arm_kg: num(&map, "ml"),
        muscle_right_leg_kg: num(&map, "mR"),
        muscle_left_leg_kg: num(&map, "mL"),
        muscle_trunk_kg: num(&map, "mT"),
        bone_kg: num(&map, "bW"),
        visceral_fat: num(&map, "IF"),
        dci_kcal: num(&map, "rD"),
        metabolic_age: num(&map, "rA"),
        water_pct: num(&map, "ww"),
        raw: line.trim().to_string(),
    })
}

/// Parst die Profilzeile aus PROFn.CSV.
pub fn parse_profile(line: &str, slot: u8) -> Result<Profile> {
    if !checksum_ok(line) {
        bail!("Prüfsumme stimmt nicht");
    }
    let map = tokenize(line)?;
    Ok(Profile {
        slot,
        model: map.get("MO").copied().unwrap_or("").to_string(),
        birth_date: map.get("DB").and_then(|s| parse_date(s).ok()),
        gender: int(&map, "GE"),
        height_cm: num(&map, "Hm"),
        activity_level: int(&map, "AL"),
        body_type: int(&map, "Bt"),
    })
}

/// Ergebnis des Parsens einer ganzen DATAn.CSV.
#[derive(Debug, Default)]
pub struct ParsedFile {
    pub measurements: Vec<Measurement>,
    /// (Zeilennummer, Fehlertext) für Zeilen, die nicht lesbar waren.
    pub errors: Vec<(usize, String)>,
}

pub fn parse_data_file(content: &str, profile: u8) -> ParsedFile {
    let mut out = ParsedFile::default();
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_measurement(line, profile) {
            Ok(m) => out.measurements.push(m),
            Err(e) => out.errors.push((i + 1, format!("{e:#}"))),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = r#"{0,16,~0,2,~1,2,~2,3,~3,4,MO,"BC-601",DT,"01/02/2024",Ti,"08:00:00",Bt,0,GE,1,AG,40,Hm,180.0,AL,1,Wk,80.0,MI,24.7,FW,20.0,Fr,18.0,Fl,18.0,FR,17.0,FL,17.0,FT,21.0,mW,60.0,mr,3.5,ml,3.5,mR,10.0,mL,10.0,mT,33.0,bW,3.2,IF,6,rD,2800,rA,38,ww,57.0,CS,BC"#;

    #[test]
    fn parses_a_measurement() {
        let m = parse_measurement(LINE, 1).unwrap();
        assert_eq!(m.model, "BC-601");
        assert_eq!(m.measured_at.to_string(), "2024-02-01 08:00:00");
        assert_eq!(m.weight_kg, 80.0);
        assert_eq!(m.fat_pct, Some(20.0));
        assert_eq!(m.muscle_kg, Some(60.0));
        assert_eq!(m.muscle_trunk_kg, Some(33.0));
        assert_eq!(m.visceral_fat, Some(6.0));
        assert_eq!(m.dci_kcal, Some(2800.0));
        assert_eq!(m.metabolic_age, Some(38.0));
        assert_eq!(m.water_pct, Some(57.0));
        assert_eq!(m.age, Some(40));
    }

    #[test]
    fn rejects_a_corrupted_line() {
        let broken = LINE.replace("Wk,80.0", "Wk,08.0").replace("CS,BC", "CS,BD");
        assert!(parse_measurement(&broken, 1).is_err());
        // Zahlendreher ohne Prüfsummenänderung fällt bei einer reinen Summe nicht auf,
        // ein geänderter Wert aber schon:
        let broken = LINE.replace("Wk,80.0", "Wk,80.1");
        assert!(parse_measurement(&broken, 1).is_err());
    }

    #[test]
    fn parses_a_profile() {
        let p = parse_profile(
            r#"{0,16,~1,2,~3,4,MO,"BC-601",DB,"15/03/1984",Bt,0,GE,1,Hm,180.0,AL,1,CS,ED"#,
            1,
        )
        .unwrap();
        assert_eq!(p.birth_date, NaiveDate::from_ymd_opt(1984, 3, 15));
        assert_eq!(p.height_cm, Some(180.0));
        assert_eq!(p.gender, Some(1));
    }

    #[test]
    fn skips_empty_lines() {
        let parsed = parse_data_file(&format!("\r\n{LINE}\r\n\r\nkaputt\r\n"), 1);
        assert_eq!(parsed.measurements.len(), 1);
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0].0, 4);
    }
}
