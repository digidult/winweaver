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
  **Weiter**, die Bilder desselben Ordners.
- Ein Klick auf das Bild gibt ihm die Tasten: Pfeil links und rechts oder Bild auf und Bild ab
  nehmen das vorige oder nächste Bild des Ordners, Pos1 und Ende das erste und das letzte,
  + und - zoomen, 0 passt das Bild ein, 1 zeigt 100%. Tab oder Escape geben die Tasten dem
  Fenster zurück.

## Einstellungen

**Einstellungen**, **Plugins** zeigt die Plugins mit ihren Einstellungen: die größte zu öffnende
Textdatei; geglättete Pixel über 100% und die längste Seite eines dekodierten Bildes. Die
Einstellungen liegen in `wview.settings.json` neben dem Programm.

## Befehlszeile

- `wview DATEI...` öffnet die Dateien in Tabs.
- `wview --plugins` zeigt die Plugins und ihre Formate.
- `wview --probe DATEI` öffnet eine Datei und zeichnet sie ohne Fenster, mit den Zeiten.
- `wview --help` zeigt die Optionen.
