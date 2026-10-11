//! Findet die SD-Karte der Waage und liest deren Dateien.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::tanita::{self, Measurement, Profile};

/// Unterordner, den die Waage auf der Karte anlegt.
const CARD_SUBDIR: &str = "TANITA/GRAPHV1";

/// Sucht unter /Volumes nach einer eingelegten Tanita-Karte.
/// Mit der Umgebungsvariable `FETTSACK_CARD` lässt sich ein Ordner erzwingen
/// (er muss dem Ordner `TANITA/GRAPHV1` entsprechen).
pub fn find_card() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("FETTSACK_CARD") {
        let path = PathBuf::from(dir);
        return path.join("DATA").is_dir().then_some(path);
    }
    let mut volumes: Vec<PathBuf> = fs::read_dir("/Volumes")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .collect();
    volumes.sort();
    volumes
        .into_iter()
        .map(|v| v.join(CARD_SUBDIR))
        .find(|p| p.join("DATA").is_dir())
}

/// Name des Volumes, auf dem die Karte liegt (für die Anzeige).
pub fn volume_name(card: &Path) -> String {
    card.ancestors()
        .nth(2)
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| card.display().to_string())
}

#[derive(Debug, Default)]
pub struct CardData {
    pub profiles: Vec<Profile>,
    pub measurements: Vec<Measurement>,
    /// Lesbare Warnungen zu Zeilen oder Dateien, die übersprungen wurden.
    pub warnings: Vec<String>,
}

/// Die Waage speichert in Windows-Kodierung; für die reinen ASCII-Daten reicht
/// eine verlustbehaftete UTF-8-Wandlung.
fn read_text(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("{} nicht lesbar", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Liest alle acht Profil-Slots von der Karte.
pub fn read_card(card: &Path) -> Result<CardData> {
    let mut data = CardData::default();
    let mut found_any = false;

    for slot in 1..=8u8 {
        let prof_path = card.join(format!("SYSTEM/PROF{slot}.CSV"));
        if prof_path.is_file() {
            match read_text(&prof_path) {
                Ok(text) => {
                    if let Some(line) = text.lines().find(|l| !l.trim().is_empty()) {
                        match tanita::parse_profile(line, slot) {
                            Ok(p) => data.profiles.push(p),
                            Err(e) => data.warnings.push(format!("PROF{slot}.CSV: {e:#}")),
                        }
                    }
                }
                Err(e) => data.warnings.push(format!("{e:#}")),
            }
        }

        let data_path = card.join(format!("DATA/DATA{slot}.CSV"));
        if !data_path.is_file() {
            continue;
        }
        found_any = true;
        match read_text(&data_path) {
            Ok(text) => {
                let parsed = tanita::parse_data_file(&text, slot);
                data.measurements.extend(parsed.measurements);
                for (line, err) in parsed.errors {
                    data.warnings
                        .push(format!("DATA{slot}.CSV Zeile {line}: {err}"));
                }
            }
            Err(e) => data.warnings.push(format!("{e:#}")),
        }
    }

    if !found_any {
        anyhow::bail!("Keine DATA-Dateien in {} gefunden", card.display());
    }
    Ok(data)
}
