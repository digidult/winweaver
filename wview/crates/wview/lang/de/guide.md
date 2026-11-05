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

## Binärdateien

Programme und Bibliotheken von Windows (`.exe`, `.dll`, `.sys` und die anderen Dateien des
Portable Executable) öffnen als Baum ihrer Struktur: die Header, die Abschnitte, die Importe und
die Exporte, jedes Feld mit seinem Wert, seinem Offset und seiner Größe.

- Links der Baum: ein Klick oder die Pfeile wählen einen Knoten, rechts und links öffnen und
  schließen ihn. Rechts die Bytes der Datei, 16 in einer Zeile mit ihrem ASCII: die Bytes des
  gewählten Knotens sind markiert, und ein Klick auf ein Byte wählt den kleinsten Knoten, der es
  hält.
- **Text** in der Leiste zeigt den Baum als Seite, mit der Suche des Tabs (Strg+F); **Baum und
  Bytes** geht zurück.
- Die Datei bleibt auf der Festplatte: die Ansicht liest die Blöcke, die sie zeigt. Ein Knoten,
  den die Datei nicht ganz hält (eine kurze oder beschädigte Datei), hat ein Warnzeichen und sagt
  warum.
- Neben den Headern: die Ressourcen (ihre Typen, Namen und Sprachen, eine Ebene wird beim Öffnen
  gelesen), die Ausnahmen von x64, die Zertifikate, die Relocations, das Debug-Verzeichnis mit dem
  Pfad der PDB, TLS, die Ladekonfiguration, die verzögerten Importe und der Header von .NET mit
  den Streams seiner Metadaten.
- **Struktur speichern** schreibt den Baum als `.wwbin`-Datei, standardmäßig neben die Datei: jedes
  Feld mit seinem Wert, jeder Bereich roher Bytes mit seinem SHA-256, die Datei mit ihrer Größe und
  ihrem SHA-256, und keines ihrer Bytes.
- **Mit einer Struktur vergleichen** vergleicht die Datei mit einer gespeicherten `.wwbin` (etwa
  einer anderen Version der Datei): der Baum der Änderungen, jede mit ihrem Wert davor und danach,
  die Knoten nach ihren Namen und Schlüsseln zugeordnet, sodass ein neuer Abschnitt keinen anderen
  verschiebt; **Alle Knoten** oder **Nur Änderungen**. Ein gewählter Knoten nimmt einen Kommentar
  und ein Lesezeichen; **Vergleich speichern** schreibt sie mit beiden Dateien in eine
  `.wwbindiff`, die den Vergleich in einem eigenen Tab öffnet. Eine `.wwbin` öffnet auch allein,
  als ihr Baum.
- Weitere Formate kommen als Beschreibungen, `.wwformat`-Dateien im Ordner `plugins\formats`;
  Einstellungen, Plugins zeigt die gelesenen Formate und die Dateien, die nicht gelesen wurden.

## Einstellungen

**Einstellungen**, **Plugins** zeigt die Plugins mit ihren Einstellungen: die größte zu öffnende
Textdatei; geglättete Pixel über 100%, die EXIF-Ausrichtung, eine Animation, die beim Öffnen
abspielt, und das größte vollständig dekodierte Bild (64, 256 oder 1024 Megapixel); die Bytes in
einer Zeile, der Block einer Binärdatei und der Cache ihrer Blöcke. Die Einstellungen liegen in
`wview.settings.json` neben dem Programm.

## Befehlszeile

- `wview DATEI...` öffnet die Dateien in Tabs.
- `wview --plugins` zeigt die Plugins und ihre Formate.
- `wview --probe DATEI` öffnet eine Datei und zeichnet sie ohne Fenster, mit den Zeiten.
- `wview --help` zeigt die Optionen.
