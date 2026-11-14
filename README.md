# NightClock

**Deutsch** · [English](#english)

Eine elegante Nachtuhr mit Wecker für Windows, gedacht für den Zweitbildschirm im Vollbild.
Geschrieben in Rust mit [egui](https://github.com/emilk/egui).

**[⬇ NightClock.exe herunterladen](https://github.com/Nikiomedia/NightClock/releases/latest)** (Windows 10/11, keine Installation nötig)

![NightClock im Schrift-Stil](docs/clock.png)

### Stimmungs-Hintergründe

| Sternenhimmel | Polarlicht |
|---|---|
| ![Sternenhimmel](docs/ambient-stars.png) | ![Polarlicht](docs/ambient-aurora.png) |

| Lavalampe | Glühwürmchen | 7-Segment, zweizeilig |
|:---:|:---:|:---:|
| ![Lavalampe](docs/ambient-lava.png) | ![Glühwürmchen](docs/ambient-fireflies.png) | ![7-Segment im Hochformat](docs/segment-portrait.png) |

### Analog und Wecker

| Analog | Wecker mit Lichtwecker |
|---|---|
| ![Analog](docs/analog.png) | ![Wecker mit Lichtwecker](docs/alarm.png) |

### Einstellungen

| Darstellung | Stimmung mit Live-Vorschau |
|---|---|
| ![Einstellungen](docs/settings.png) | ![Stimmung](docs/settings-ambience.png) |

## Funktionen

- **Drei Stile:** Schrift (jede installierte Schriftart), 7-Segment (LED-Wecker) und Analog
- **Vektor-Ziffern:** gestochen scharf in jeder Größe, auch auf 4K
- **Einzeilig oder zweizeilig:** Stunden und Minuten nebeneinander oder übereinander, ideal für Hochformat-Monitore
- **Position frei wählbar:** Die Uhr lässt sich per Schieberegler waagrecht und senkrecht verschieben,
  z. B. ins obere Viertel eines Hochformat-Monitors
- **Datum nach Wunsch:** eigene Schriftart, Größe und Helligkeit
- **Helligkeit, Farbe, Leuchten, Größe, Hintergrund** und Schriftstärke bei variablen Schriften
- **Stimmungs-Hintergründe zum Einschlafen:** Sternenhimmel mit Sternschnuppen, Polarlicht,
  Glühwürmchen, Lavalampe, Plasma und ein Atem-Licht zum Mitatmen, mit Helligkeit, Tempo,
  Einfärben in der Uhrfarbe und automatischem Ausblenden
- **Nachtmodus:** dimmt in einem Zeitfenster automatisch
- **Wecker:** mehrere Weckzeiten mit Namen und Wochentagen, drei eingebaute Töne oder eigene Audiodatei,
  sanft lauter werden, Schlummern, automatisches Ausschalten
- **Lichtwecker:** Vor dem Wecken wird der Bildschirm langsam warm und hell
- **Vollbild auf dem gewünschten Bildschirm**, Bildschirm wach halten, Einbrennschutz,
  Mauszeiger ausblenden, immer im Vordergrund
- **Deutsch und Englisch**, umschaltbar oben im Einstellungsmenü
- **Läuft auch auf schwacher Grafik:** sparsame Darstellung, optionaler Sparmodus

![Einstellungen](docs/settings.png)

## Bedienung

`NightClock.exe` starten. Mit `NightClock.exe --fullscreen` startet die Uhr direkt im Vollbild.

| Taste | Funktion |
|---|---|
| F11 · F · Doppelklick | Vollbild an / aus |
| Esc | Vollbild bzw. Menü schließen |
| S · Rechtsklick | Einstellungen |
| Mausrad · ↑ ↓ | Helligkeit |
| M | Nächster Bildschirm |
| 1 · 2 · 3 | Stil wechseln |
| Z | Einzeilig / zweizeilig |
| Leertaste | Schlummern |
| Enter | Wecker ausschalten |

Die Einstellungen werden automatisch in `%APPDATA%\NightClock\settings.json` gespeichert.

**Hinweis:** Der Wecker klingelt nur, solange NightClock läuft. Solange ein Wecker aktiv ist,
verhindert die App, dass Windows in den Ruhezustand geht.

## Selbst bauen

Voraussetzung: [Rust](https://rustup.rs) mit dem MSVC-Toolchain.

```
cargo build --release
```

Die fertige Datei liegt dann unter `target\release\nightclock.exe`.

## Lizenz

[MIT](LICENSE)

---

<a name="english"></a>

# NightClock (English)

An elegant night clock with alarm for Windows, made to run fullscreen on a second monitor.
Written in Rust with [egui](https://github.com/emilk/egui).

**[⬇ Download NightClock.exe](https://github.com/Nikiomedia/NightClock/releases/latest)** (Windows 10/11, no installation required)

| Starry sky | Aurora |
|---|---|
| ![Starry sky](docs/ambient-stars.png) | ![Aurora](docs/ambient-aurora.png) |

| Lava lamp | Fireflies | 7-segment, two lines |
|:---:|:---:|:---:|
| ![Lava lamp](docs/ambient-lava.png) | ![Fireflies](docs/ambient-fireflies.png) | ![7-segment in portrait](docs/segment-portrait.png) |

| Settings (English) | Alarm with wake-up light |
|---|---|
| ![Settings in English](docs/settings-en.png) | ![Alarm](docs/alarm.png) |

## Features

- **Three styles:** text (any installed font), 7-segment (LED alarm clock) and analog
- **Vector digits:** razor-sharp at any size, including 4K
- **Single or two lines:** hours and minutes side by side or stacked, ideal for portrait monitors
- **Free positioning:** sliders move the clock horizontally and vertically,
  e.g. into the top quarter of a portrait monitor
- **Date your way:** its own font, size and brightness
- **Brightness, color, glow, size, background**, and font weight for variable fonts
- **Ambient backgrounds for falling asleep:** starry sky with shooting stars, aurora,
  fireflies, lava lamp, plasma and a breathing light to breathe along with, with brightness,
  speed, clock-color tint and automatic fade-out
- **Night mode:** dims automatically during a time window
- **Alarms:** several named alarm times with weekdays, three built-in sounds or your own audio file,
  gentle fade-in, snooze, automatic shut-off
- **Wake-up light:** before the alarm, the screen slowly turns warm and bright
- **Fullscreen on the display of your choice**, keep screen awake, burn-in protection,
  hide mouse cursor, always on top
- **German and English**, switchable at the top of the settings menu
- **Runs on weak graphics too:** lightweight rendering, optional eco mode

## Usage

Start `NightClock.exe`. `NightClock.exe --fullscreen` starts directly in fullscreen.

| Key | Action |
|---|---|
| F11 · F · Double-click | Fullscreen on / off |
| Esc | Exit fullscreen / close menu |
| S · Right-click | Settings |
| Mouse wheel · ↑ ↓ | Brightness |
| M | Next display |
| 1 · 2 · 3 | Change style |
| Z | Single / two lines |
| Space | Snooze |
| Enter | Turn off alarm |

Settings are saved automatically to `%APPDATA%\NightClock\settings.json`.

**Note:** Alarms only ring while NightClock is running. While an alarm is active,
the app keeps Windows from going to sleep.

## Building

Requires [Rust](https://rustup.rs) with the MSVC toolchain.

```
cargo build --release
```

The executable is then located at `target\release\nightclock.exe`.

## License

[MIT](LICENSE)
