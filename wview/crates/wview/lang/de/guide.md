# wview

wview zeigt Dateien. Jedes Format ist ein Plugin: eine DLL im Ordner `plugins` neben `wview.exe`.
Ohne Datei zeigt das Fenster die Plugins und was jedes von ihnen öffnet.

## Dateien öffnen

- **Öffnen** (Ctrl+O) oder eine auf das Fenster gezogene Datei öffnet die Datei in einem Tab;
  `wview DATEI...` öffnet Dateien von der Befehlszeile.
- Eine Datei eines Formats, das kein Plugin öffnet, bleibt geschlossen, und die Statusleiste
  sagt es.
- Ctrl+W schließt einen Tab, Ctrl+Tab geht zum nächsten.

## Text und Markdown

Markdown-Seiten (`.md`) erscheinen als Seiten, Textdateien (`.txt`, `.log`, `.ini`, `.cfg`,
`.csv`) so, wie sie sind. Ctrl+F durchsucht den Text. **Einstellungen**, **Allgemein**: *Zeilen
auf die Breite der Ansicht umbrechen* und *Breite der Seiten* ändern die Anzeige eines Textes.

## Bilder

Jedes Format, das das Windows des Computers dekodiert: BMP, PNG, JPEG, GIF, TIFF, ICO sowie HEIF,
AVIF, WebP oder das RAW von Kameras, wo ihre Erweiterungen installiert sind.

- Das Mausrad zoomt am Zeiger, Ziehen verschiebt das Bild, ein Doppelklick wechselt zwischen dem
  ganzen Bild und 100%.
- Die Leiste: **Einpassen**, **100%**, **Vergrößern**, **Verkleinern** sowie **Zurück** und
  **Weiter**, die Bilder desselben Ordners; für eine Animation **Abspielen** (oder **Pause**),
  **Vorheriges Einzelbild** und **Nächstes Einzelbild**; für eine Datei mit mehreren Seiten (TIFF,
  ICO) **Vorherige Seite** und **Nächste Seite**.
- Ein Klick auf das Bild gibt ihm die Tasten: Die Pfeile verschieben ein Bild, das größer als das
  Fenster ist, sonst nehmen Pfeil links und rechts das vorige oder nächste Bild des Ordners, wie es
  Bild auf und Bild ab immer tun; Pos1 und Ende das erste und das letzte; + und - zoomen in
  Stufen, 0 passt das Bild ein, 1 bis 4 zeigen 100% bis 400%; die Leertaste spielt eine Animation
  ab oder hält sie an, Komma und Punkt gehen zum vorigen oder nächsten Einzelbild oder zur Seite
  davor oder danach. Tab oder Escape geben die Tasten dem Fenster zurück.
- Ein Foto wird so gedreht, wie die Kamera gehalten wurde (die EXIF-Ausrichtung). Ein großes Bild
  wird dekodiert, während sein Tab schon offen ist, und in Teilen mit der Genauigkeit gezeichnet,
  die der Zoom braucht; ein Bild über der Grenze der Einstellungen wird verkleinert dekodiert, und
  die Statusleiste sagt es.

## Einstellungen

**Einstellungen**, **Plugins** zeigt die Plugins mit ihren Einstellungen: die größte zu öffnende
Textdatei; geglättete Pixel über 100%, die EXIF-Ausrichtung, eine Animation, die beim Öffnen
abspielt, und das größte vollständig dekodierte Bild (64, 256 oder 1024 Megapixel). Die
Einstellungen liegen in `wview.settings.json` neben dem Programm.

## Befehlszeile

- `wview DATEI...` öffnet die Dateien in Tabs.
- `wview --plugins` zeigt die Plugins und ihre Formate.
- `wview --probe DATEI` öffnet eine Datei und zeichnet sie ohne Fenster, mit den Zeiten.
- `wview --help` zeigt die Optionen.
