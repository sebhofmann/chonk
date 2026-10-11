//! Findet die SD-Karte der Waage und liest deren Dateien.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::tanita::{self, Measurement, Profile};

/// Sucht einen Verzeichniseintrag ohne Rücksicht auf Groß-/Kleinschreibung.
/// FAT-Karten erscheinen je nach System und Mount-Option in anderer Schreibweise.
fn child(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name))
        })
}

/// Liefert `<volume>/TANITA/GRAPHV1`, falls das Volume eine Tanita-Karte ist.
fn card_dir(volume: &Path) -> Option<PathBuf> {
    let card = child(&child(volume, "TANITA")?, "GRAPHV1")?;
    child(&card, "DATA")?.is_dir().then_some(card)
}

/// Orte, an denen das Betriebssystem Wechseldatenträger einhängt.
fn volumes() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if cfg!(target_os = "windows") {
        return (b'A'..=b'Z')
            .map(|l| PathBuf::from(format!("{}:\\", l as char)))
            .collect();
    } else if cfg!(target_os = "macos") {
        roots.push("/Volumes".into());
    } else {
        if let Ok(user) = std::env::var("USER") {
            roots.push(format!("/run/media/{user}").into());
            roots.push(format!("/media/{user}").into());
        }
        roots.push("/media".into());
        roots.push("/mnt".into());
    }
    let mut volumes: Vec<PathBuf> = roots
        .iter()
        .filter_map(|r| fs::read_dir(r).ok())
        .flat_map(|entries| entries.flatten().map(|e| e.path()))
        .collect();
    volumes.sort();
    volumes
}

/// Sucht eine eingelegte Tanita-Karte.
/// Mit der Umgebungsvariable `FETTSACK_CARD` lässt sich ein Ordner erzwingen
/// (er muss dem Ordner `TANITA/GRAPHV1` entsprechen).
pub fn find_card() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("FETTSACK_CARD") {
        let path = PathBuf::from(dir);
        return child(&path, "DATA")?.is_dir().then_some(path);
    }
    volumes().iter().find_map(|v| card_dir(v))
}

/// Name des Volumes, auf dem die Karte liegt (für die Anzeige).
pub fn volume_name(card: &Path) -> String {
    let Some(volume) = card.ancestors().nth(2) else {
        return card.display().to_string();
    };
    match volume.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        // Unter Windows ist das Volume ein Laufwerk wie `E:\` ohne Dateinamen.
        None => volume
            .display()
            .to_string()
            .trim_end_matches('\\')
            .to_string(),
    }
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
        let prof_path = child(card, "SYSTEM").and_then(|d| child(&d, &format!("PROF{slot}.CSV")));
        if let Some(prof_path) = prof_path.filter(|p| p.is_file()) {
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

        let data_path = child(card, "DATA").and_then(|d| child(&d, &format!("DATA{slot}.CSV")));
        let Some(data_path) = data_path.filter(|p| p.is_file()) else {
            continue;
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = r#"{0,16,~0,2,~1,2,~2,3,~3,4,MO,"BC-601",DT,"01/02/2024",Ti,"08:00:00",Bt,0,GE,1,AG,40,Hm,180.0,AL,1,Wk,80.0,MI,24.7,FW,20.0,Fr,18.0,Fl,18.0,FR,17.0,FL,17.0,FT,21.0,mW,60.0,mr,3.5,ml,3.5,mR,10.0,mL,10.0,mT,33.0,bW,3.2,IF,6,rD,2800,rA,38,ww,57.0,CS,@@"#;

    fn with_checksum(line: &str) -> String {
        let body = line.trim_end_matches("@@");
        let sum = body[..body.len() - 3]
            .bytes()
            .fold(0u8, |a, b| a.wrapping_add(b));
        format!("{body}{sum:02X}")
    }

    #[test]
    fn finds_and_reads_a_card_regardless_of_case() {
        let root = std::env::temp_dir().join(format!("fettsack-card-{}", std::process::id()));
        let card = root.join("tanita").join("GraphV1");
        fs::create_dir_all(card.join("data")).unwrap();
        fs::write(
            card.join("data").join("data2.csv"),
            format!("{}\r\n", with_checksum(LINE)),
        )
        .unwrap();

        let found = card_dir(&root).expect("Karte nicht erkannt");
        let data = read_card(&found).unwrap();
        assert_eq!(data.measurements.len(), 1);
        assert_eq!(data.measurements[0].profile, 2);
        assert!(data.warnings.is_empty());
        assert_eq!(
            volume_name(&found),
            root.file_name().unwrap().to_string_lossy()
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
