# Chonk

Desktop-App (Rust + [gpui](https://www.gpui.rs)), die die Messungen einer
Tanita-Körperanalysewaage (getestet mit der BC-601) von der SD-Karte liest,
in einer lokalen SQLite-Datenbank archiviert und mit Diagrammen darstellt.
Läuft unter macOS, Linux und Windows.

![Übersicht der App mit Gewichtsverlauf, Kennzahlen und Profilumschalter](docs/screenshot.png)

*Der Screenshot zeigt erfundene Beispieldaten.*

## Installieren

Fertige Pakete hängen an den [Releases](../../releases):

| System | Paket | Hinweis |
|---|---|---|
| macOS | `Chonk-…-macos-universal.zip` | Die App ist nicht notarisiert. Beim ersten Start per Rechtsklick „Öffnen" wählen oder `xattr -dr com.apple.quarantine Chonk.app` ausführen. |
| Linux | `chonk-…-linux-x86_64.tar.gz` | Braucht Vulkan sowie Wayland oder X11. |
| Windows | `chonk-…-windows-x86_64.zip` | `chonk.exe` direkt starten. |

## Selbst bauen

```sh
cargo run --release
```

Unter Debian/Ubuntu vorher `script/linux-deps.sh` ausführen.

## Bedienung

Die App sucht auf allen Wechseldatenträgern nach dem Ordner `TANITA/GRAPHV1`
(macOS: `/Volumes`, Linux: `/run/media/$USER`, `/media`, `/mnt`, Windows: alle
Laufwerksbuchstaben). Ist die Karte
eingelegt (beim Start oder später), werden neue Messungen automatisch
importiert. Bereits archivierte Messungen werden übersprungen, die Karte wird
nur gelesen.

Die Oberfläche gibt es auf Deutsch und Englisch. Beim ersten Start richtet sie
sich nach der Systemsprache, umschalten lässt sie sich unten rechts.

Die Waage kennt acht Profile, aber keine Namen. Ein Klick auf den Profilnamen
unter dem Titel vergibt einen eigenen Namen (Enter speichert, Esc bricht ab).

## Daten

| Was | Wo |
|---|---|
| Archiv | `~/.chonk/chonk.db` (SQLite), unter Windows `%USERPROFILE%\.chonk` |
| Messungen | Tabelle `measurements`, eindeutig je Profil und Zeitpunkt, inklusive Originalzeile in `raw` |
| Profile | Tabelle `profiles` (Slot 1–8 der Waage, eigener Name in `name`) |
| Einstellungen | Tabelle `settings` (zum Beispiel die Sprache) |
| Importprotokoll | Tabelle `imports` |

Die App hieß früher „Fettsack". Ein vorhandenes Archiv unter `~/.fettsack`
wird beim ersten Start automatisch nach `~/.chonk` umgezogen.

Umgebungsvariablen zum Testen: `CHONK_HOME` (anderer Datenordner) und
`CHONK_CARD` (Pfad zu einem `TANITA/GRAPHV1`-Ordner statt der Karte).

Eine Karte mit erfundenen Messwerten für zwei Profile erzeugt
`script/fake-card.py`:

```sh
python3 script/fake-card.py /tmp/demo/TANITA/GRAPHV1
CHONK_HOME=/tmp/demo/home CHONK_CARD=/tmp/demo/TANITA/GRAPHV1 cargo run
```

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

## Entwicklung

- gpui wird mit dem Feature `runtime_shaders` gebaut. Unter macOS genügen
  dadurch die Xcode Command Line Tools, ein vollständiges Xcode ist nicht nötig.
- Übersetzungen stehen in `src/i18n.rs`. Feste Texte sind Felder einer
  Struktur, der Compiler meldet also fehlende Übersetzungen.
- Der Workflow `CI` prüft jeden Push unter Linux, macOS und Windows
  (Formatierung, Clippy, Tests). Abhängigkeiten werden zwischen den Läufen
  gecacht, reine Doku-Änderungen lösen keinen Lauf aus.
- Der Workflow `Release` startet, sobald auf GitHub ein Release veröffentlicht
  wird (Releases → „Draft a new release" → Tag wie `v0.1.0` wählen →
  „Publish release"). Er baut die Pakete und hängt sie an das Release.
