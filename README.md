# Fettsack

Desktop-App (Rust + [gpui](https://www.gpui.rs)), die die Messungen einer
Tanita-Körperanalysewaage (getestet mit der BC-601) von der SD-Karte liest,
in einer lokalen SQLite-Datenbank archiviert und mit Diagrammen darstellt.

## Starten

```sh
cargo run --release
```

Die App sucht unter `/Volumes/*/TANITA/GRAPHV1` nach der Karte. Ist sie
eingelegt (beim Start oder später), werden neue Messungen automatisch
importiert. Bereits archivierte Messungen werden übersprungen, die Karte wird
nur gelesen.

## Daten

| Was | Wo |
|---|---|
| Archiv | `~/.fettsack/fettsack.db` (SQLite) |
| Messungen | Tabelle `measurements`, eindeutig je Profil und Zeitpunkt, inklusive Originalzeile in `raw` |
| Profile | Tabelle `profiles` (Slot 1–8 der Waage) |
| Importprotokoll | Tabelle `imports` |

Umgebungsvariablen zum Testen: `FETTSACK_HOME` (anderer Datenordner) und
`FETTSACK_CARD` (Pfad zu einem `TANITA/GRAPHV1`-Ordner statt der Karte).

## Dateiformat der Waage

`DATA/DATAn.CSV` enthält pro Messung eine Zeile aus Schlüssel-Wert-Paaren,
`SYSTEM/PROFn.CSV` das Profil. `n` ist der Profil-Slot.

| Schlüssel | Bedeutung |
|---|---|
| `MO` | Modell |
| `DT`, `Ti` | Datum (`TT/MM/JJJJ`) und Uhrzeit |
| `Bt`, `GE`, `AG`, `Hm`, `AL` | Körpertyp, Geschlecht, Alter, Größe in cm, Aktivitätslevel |
| `Wk`, `MI` | Gewicht in kg, BMI |
| `FW` | Körperfett gesamt in % |
| `Fr`, `Fl`, `FR`, `FL`, `FT` | Fett in % für rechten/linken Arm, rechtes/linkes Bein, Rumpf |
| `mW` | Muskelmasse gesamt in kg |
| `mr`, `ml`, `mR`, `mL`, `mT` | Muskelmasse in kg, gleiche Segmente |
| `bW` | Knochenmasse in kg |
| `IF` | Viszeralfett-Stufe |
| `rD`, `rA` | täglicher Kalorienbedarf in kcal, Stoffwechselalter |
| `ww` | Körperwasser in % |
| `CS` | Prüfsumme: Summe aller Bytes vor `CS` modulo 256, hexadezimal |

Zeilen mit falscher Prüfsumme werden beim Import übersprungen.

## Hinweis zum Bauen

gpui wird mit dem Feature `runtime_shaders` gebaut. Dadurch genügen die
Xcode Command Line Tools, ein vollständiges Xcode ist nicht nötig.
