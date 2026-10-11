//! Lokales Archiv in SQLite (`~/.fettsack/fettsack.db`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate, NaiveDateTime};
use rusqlite::{Connection, params};

use crate::tanita::{Measurement, Profile};

const DT_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS profiles (
    slot            INTEGER PRIMARY KEY,
    model           TEXT NOT NULL DEFAULT '',
    birth_date      TEXT,
    gender          INTEGER,
    height_cm       REAL,
    activity_level  INTEGER,
    body_type       INTEGER,
    updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS measurements (
    id                   INTEGER PRIMARY KEY,
    profile              INTEGER NOT NULL,
    measured_at          TEXT NOT NULL,
    model                TEXT NOT NULL DEFAULT '',
    body_type            INTEGER,
    gender               INTEGER,
    age                  INTEGER,
    height_cm            REAL,
    activity_level       INTEGER,
    weight_kg            REAL NOT NULL,
    bmi                  REAL,
    fat_pct              REAL,
    fat_right_arm_pct    REAL,
    fat_left_arm_pct     REAL,
    fat_right_leg_pct    REAL,
    fat_left_leg_pct     REAL,
    fat_trunk_pct        REAL,
    muscle_kg            REAL,
    muscle_right_arm_kg  REAL,
    muscle_left_arm_kg   REAL,
    muscle_right_leg_kg  REAL,
    muscle_left_leg_kg   REAL,
    muscle_trunk_kg      REAL,
    bone_kg              REAL,
    visceral_fat         REAL,
    dci_kcal             REAL,
    metabolic_age        REAL,
    water_pct            REAL,
    raw                  TEXT NOT NULL,
    imported_at          TEXT NOT NULL,
    UNIQUE (profile, measured_at)
);

CREATE TABLE IF NOT EXISTS imports (
    id           INTEGER PRIMARY KEY,
    imported_at  TEXT NOT NULL,
    source       TEXT NOT NULL,
    rows_seen    INTEGER NOT NULL,
    rows_new     INTEGER NOT NULL
);
"#;

pub struct Db {
    conn: Connection,
    path: PathBuf,
}

/// Ergebnis eines Imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportStats {
    pub seen: usize,
    pub new: usize,
}

/// Datenordner der App: `~/.fettsack` (oder `FETTSACK_HOME`).
pub fn data_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("FETTSACK_HOME") {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Weder HOME noch USERPROFILE ist gesetzt")?;
    Ok(PathBuf::from(home).join(".fettsack"))
}

impl Db {
    pub fn open_default() -> Result<Self> {
        let dir = data_dir()?;
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("{} konnte nicht angelegt werden", dir.display()))?;
        Self::open(&dir.join("fettsack.db"))
    }

    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("{} konnte nicht geöffnet werden", path.display()))?;
        conn.execute_batch(SCHEMA)?;
        // Migration: Archive aus älteren Versionen kennen noch keine Profilnamen.
        let has_name = conn
            .prepare("SELECT 1 FROM pragma_table_info('profiles') WHERE name = 'name'")?
            .exists([])?;
        if !has_name {
            conn.execute_batch("ALTER TABLE profiles ADD COLUMN name TEXT")?;
        }
        Ok(Self {
            conn,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Archiviert Profile und Messungen. Bereits bekannte Messungen
    /// (gleiches Profil, gleicher Zeitpunkt) werden übersprungen.
    pub fn import(
        &mut self,
        source: &str,
        profiles: &[Profile],
        measurements: &[Measurement],
    ) -> Result<ImportStats> {
        let now = Local::now().naive_local().format(DT_FORMAT).to_string();
        let tx = self.conn.transaction()?;
        let mut new = 0;
        {
            let mut prof = tx.prepare(
                "INSERT INTO profiles (slot, model, birth_date, gender, height_cm, activity_level, body_type, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(slot) DO UPDATE SET
                    model = excluded.model, birth_date = excluded.birth_date, gender = excluded.gender,
                    height_cm = excluded.height_cm, activity_level = excluded.activity_level,
                    body_type = excluded.body_type, updated_at = excluded.updated_at",
            )?;
            for p in profiles {
                prof.execute(params![
                    p.slot,
                    p.model,
                    p.birth_date.map(|d| d.to_string()),
                    p.gender,
                    p.height_cm,
                    p.activity_level,
                    p.body_type,
                    now
                ])?;
            }

            let mut ins = tx.prepare(
                "INSERT OR IGNORE INTO measurements (
                    profile, measured_at, model, body_type, gender, age, height_cm, activity_level,
                    weight_kg, bmi, fat_pct, fat_right_arm_pct, fat_left_arm_pct, fat_right_leg_pct,
                    fat_left_leg_pct, fat_trunk_pct, muscle_kg, muscle_right_arm_kg, muscle_left_arm_kg,
                    muscle_right_leg_kg, muscle_left_leg_kg, muscle_trunk_kg, bone_kg, visceral_fat,
                    dci_kcal, metabolic_age, water_pct, raw, imported_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                         ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)",
            )?;
            for m in measurements {
                new += ins.execute(params![
                    m.profile,
                    m.measured_at.format(DT_FORMAT).to_string(),
                    m.model,
                    m.body_type,
                    m.gender,
                    m.age,
                    m.height_cm,
                    m.activity_level,
                    m.weight_kg,
                    m.bmi,
                    m.fat_pct,
                    m.fat_right_arm_pct,
                    m.fat_left_arm_pct,
                    m.fat_right_leg_pct,
                    m.fat_left_leg_pct,
                    m.fat_trunk_pct,
                    m.muscle_kg,
                    m.muscle_right_arm_kg,
                    m.muscle_left_arm_kg,
                    m.muscle_right_leg_kg,
                    m.muscle_left_leg_kg,
                    m.muscle_trunk_kg,
                    m.bone_kg,
                    m.visceral_fat,
                    m.dci_kcal,
                    m.metabolic_age,
                    m.water_pct,
                    m.raw,
                    now
                ])?;
            }
            tx.execute(
                "INSERT INTO imports (imported_at, source, rows_seen, rows_new) VALUES (?1, ?2, ?3, ?4)",
                params![now, source, measurements.len() as i64, new as i64],
            )?;
        }
        tx.commit()?;
        Ok(ImportStats {
            seen: measurements.len(),
            new,
        })
    }

    /// Alle Messungen eines Profils, älteste zuerst.
    pub fn measurements(&self, profile: u8) -> Result<Vec<Measurement>> {
        let mut stmt = self.conn.prepare(
            "SELECT profile, measured_at, model, body_type, gender, age, height_cm, activity_level,
                    weight_kg, bmi, fat_pct, fat_right_arm_pct, fat_left_arm_pct, fat_right_leg_pct,
                    fat_left_leg_pct, fat_trunk_pct, muscle_kg, muscle_right_arm_kg, muscle_left_arm_kg,
                    muscle_right_leg_kg, muscle_left_leg_kg, muscle_trunk_kg, bone_kg, visceral_fat,
                    dci_kcal, metabolic_age, water_pct, raw
             FROM measurements WHERE profile = ?1 ORDER BY measured_at",
        )?;
        let rows = stmt.query_map([profile], |r| {
            let measured_at: String = r.get(1)?;
            let measured_at =
                NaiveDateTime::parse_from_str(&measured_at, DT_FORMAT).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
            Ok(Measurement {
                profile: r.get(0)?,
                measured_at,
                model: r.get(2)?,
                body_type: r.get(3)?,
                gender: r.get(4)?,
                age: r.get(5)?,
                height_cm: r.get(6)?,
                activity_level: r.get(7)?,
                weight_kg: r.get(8)?,
                bmi: r.get(9)?,
                fat_pct: r.get(10)?,
                fat_right_arm_pct: r.get(11)?,
                fat_left_arm_pct: r.get(12)?,
                fat_right_leg_pct: r.get(13)?,
                fat_left_leg_pct: r.get(14)?,
                fat_trunk_pct: r.get(15)?,
                muscle_kg: r.get(16)?,
                muscle_right_arm_kg: r.get(17)?,
                muscle_left_arm_kg: r.get(18)?,
                muscle_right_leg_kg: r.get(19)?,
                muscle_left_leg_kg: r.get(20)?,
                muscle_trunk_kg: r.get(21)?,
                bone_kg: r.get(22)?,
                visceral_fat: r.get(23)?,
                dci_kcal: r.get(24)?,
                metabolic_age: r.get(25)?,
                water_pct: r.get(26)?,
                raw: r.get(27)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Profil-Slots, für die Messungen vorliegen, mit Anzahl.
    pub fn profile_slots(&self) -> Result<Vec<(u8, usize)>> {
        let mut stmt = self.conn.prepare(
            "SELECT profile, COUNT(*) FROM measurements GROUP BY profile ORDER BY profile",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as usize)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn profile(&self, slot: u8) -> Result<Option<Profile>> {
        let mut stmt = self.conn.prepare(
            "SELECT slot, model, birth_date, gender, height_cm, activity_level, body_type
             FROM profiles WHERE slot = ?1",
        )?;
        let mut rows = stmt.query_map([slot], |r| {
            let birth: Option<String> = r.get(2)?;
            Ok(Profile {
                slot: r.get(0)?,
                model: r.get(1)?,
                birth_date: birth.and_then(|b| NaiveDate::parse_from_str(&b, "%Y-%m-%d").ok()),
                gender: r.get(3)?,
                height_cm: r.get(4)?,
                activity_level: r.get(5)?,
                body_type: r.get(6)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    /// Vom Nutzer vergebene Namen je Profil-Slot.
    pub fn profile_names(&self) -> Result<HashMap<u8, String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT slot, name FROM profiles WHERE name IS NOT NULL AND name <> ''")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<HashMap<_, _>>>()?)
    }

    /// Setzt oder löscht (leerer Text) den Namen eines Profils. Der Name bleibt
    /// bei späteren Importen erhalten.
    pub fn set_profile_name(&self, slot: u8, name: &str) -> Result<()> {
        let name = name.trim();
        let name = (!name.is_empty()).then_some(name);
        let now = Local::now().naive_local().format(DT_FORMAT).to_string();
        self.conn.execute(
            "INSERT INTO profiles (slot, name, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(slot) DO UPDATE SET name = excluded.name",
            params![slot, name, now],
        )?;
        Ok(())
    }

    /// Zeitpunkt des letzten Imports.
    pub fn last_import(&self) -> Result<Option<NaiveDateTime>> {
        let ts: Option<String> =
            self.conn
                .query_row("SELECT MAX(imported_at) FROM imports", [], |r| r.get(0))?;
        Ok(ts.and_then(|t| NaiveDateTime::parse_from_str(&t, DT_FORMAT).ok()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tanita::parse_measurement;

    const LINE: &str = r#"{0,16,~0,2,~1,2,~2,3,~3,4,MO,"BC-601",DT,"01/02/2024",Ti,"08:00:00",Bt,0,GE,1,AG,40,Hm,180.0,AL,1,Wk,80.0,MI,24.7,FW,20.0,Fr,18.0,Fl,18.0,FR,17.0,FL,17.0,FT,21.0,mW,60.0,mr,3.5,ml,3.5,mR,10.0,mL,10.0,mT,33.0,bW,3.2,IF,6,rD,2800,rA,38,ww,57.0,CS,BC"#;

    #[test]
    fn import_is_idempotent_and_round_trips() {
        let mut db = Db::open(Path::new(":memory:")).unwrap();
        let m = parse_measurement(LINE, 1).unwrap();
        let first = db.import("test", &[], std::slice::from_ref(&m)).unwrap();
        assert_eq!(first, ImportStats { seen: 1, new: 1 });
        let second = db.import("test", &[], std::slice::from_ref(&m)).unwrap();
        assert_eq!(second, ImportStats { seen: 1, new: 0 });

        let stored = db.measurements(1).unwrap();
        assert_eq!(stored, vec![m]);
        assert_eq!(db.profile_slots().unwrap(), vec![(1, 1)]);
        assert!(db.last_import().unwrap().is_some());
    }

    #[test]
    fn profile_names_survive_a_reimport() {
        let mut db = Db::open(Path::new(":memory:")).unwrap();
        let profile = crate::tanita::Profile {
            slot: 2,
            model: "BC-601".into(),
            birth_date: None,
            gender: Some(1),
            height_cm: Some(180.0),
            activity_level: Some(1),
            body_type: Some(0),
        };
        db.set_profile_name(2, "  Alex ").unwrap();
        db.import("test", std::slice::from_ref(&profile), &[])
            .unwrap();
        assert_eq!(
            db.profile_names().unwrap().get(&2).map(String::as_str),
            Some("Alex")
        );
        assert_eq!(db.profile(2).unwrap(), Some(profile));

        db.set_profile_name(2, "").unwrap();
        assert!(db.profile_names().unwrap().is_empty());
    }

    #[test]
    fn migrates_an_archive_without_names() {
        let path = std::env::temp_dir().join(format!("fettsack-mig-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE profiles (slot INTEGER PRIMARY KEY, model TEXT NOT NULL DEFAULT '',
                 birth_date TEXT, gender INTEGER, height_cm REAL, activity_level INTEGER,
                 body_type INTEGER, updated_at TEXT NOT NULL);
                 INSERT INTO profiles (slot, updated_at) VALUES (1, '2024-01-01 00:00:00');",
            )
            .unwrap();
        }
        let db = Db::open(&path).unwrap();
        db.set_profile_name(1, "Alex").unwrap();
        assert_eq!(db.profile_names().unwrap().len(), 1);
        drop(db);
        std::fs::remove_file(&path).unwrap();
    }
}
