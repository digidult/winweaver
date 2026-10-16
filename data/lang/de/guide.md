# WinWeaver-Handbuch

WinWeaver bereitet unbeaufsichtigte Installationen von Windows 11 Pro für kleine Arbeitsgruppen vor und hält
die installierten PCs konfiguriert. Ein Profil legt fest, welche Regeln des Katalogs eingeschaltet sind, mit
welchen Parametern, und was die Installationsformulare enthalten; daraus erstellt das Programm
`autounattend.xml`.

Diese Seite wird beim ersten Start geöffnet und später über das Menü oder mit F1. Sie ist eine
Markdown-Datei; die Seiten der Dokumentation werden genauso geöffnet, jede in einer eigenen Registerkarte.

## Erste Schritte

1. Öffnen Sie eine Vorlage: **Menü, Neu, Profil aus der Vorlage Office**. Office enthält die Standardwerte
   des Katalogs; Strict fügt Einschränkungen hinzu, Laptop eignet sich für tragbare PCs, Home ist eine
   Positivliste für einen PC zu Hause.
2. Gehen Sie die Struktur links durch. Ein Kontrollkästchen schaltet eine Regel ein oder aus; das
   Kontrollkästchen einer Gruppe schaltet alle ihre Regeln auf einmal ein oder aus; eine abgeblendete Regel
   wartet auf eine Regel, die sie benötigt. Der rechte Bereich zeigt die Parameter der ausgewählten Regel
   und ihre Beschreibung.
3. Füllen Sie die Formulare **Installation**, **Konten** und **Sprachen** unter dem ersten Stammknoten aus.
4. Prüfen Sie das Profil mit F7. Die Meldungen erscheinen unter der Navigationszeile; ein Klick auf eine
   Meldung wählt die Regel oder das Formular aus, um das es geht.
5. Erstellen Sie die Antwortdatei mit F9: `autounattend.xml` wird im Ordner `output` neben der ausführbaren
   Datei abgelegt, daneben zur Ansicht die Skripts, die sie einbettet. Kopieren Sie die Datei in das
   Stammverzeichnis des USB-Installationssticks.
6. Speichern Sie das Profil mit Ctrl+S. Profile sind `.wwprofile`-Dateien im Ordner `profiles` neben der
   ausführbaren Datei oder an einem beliebigen Ort, den Sie mit „Speichern unter“ wählen.

## Das Fenster

- **Registerkarten**: Jedes geöffnete Dokument ist eine Registerkarte: ein Profil, ein Vergleich, dieses
  Handbuch, die Lesezeichen, der MCP-Server, die Einstellungen. Die Menüschaltfläche am linken Ende der
  Registerkartenleiste enthält Neu, Öffnen, Speichern, Vergleichen, das Handbuch, den MCP-Server, die
  Lesezeichen, Einstellungen und Beenden.
- **Befehlsleiste**: Neu, Öffnen, Speichern, Prüfen, Erstellen, Dieser PC, Vergleichen. In einem schmalen
  Fenster zeigt sie nur Symbole, und zwar in der Navigationszeile.
- **Navigationszeile**: Zurück und Vorwärts bewegen sich durch die Elemente, die Sie ausgewählt haben; die
  Pfadleiste zeigt, wo Sie sich befinden, und springt zu jeder Ebene; Lesezeichen markiert die Position;
  Suchen filtert die Struktur während der Eingabe und merkt sich, wonach Sie gesucht haben.
- **Struktur und Detailbereich**: links die Struktur, rechts das ausgewählte Element. Die Beschreibung wird
  mit Ctrl+D eingeklappt. Die Regeln einer ausgewählten Gruppe zeigen am Ende ihrer Zeilen einen Schalter.
- **Statusleiste**: der Zustand des Dokuments, die Katalogversion, der MCP-Server und die letzte Meldung.

## Dieser PC

**Dieser PC** in der Befehlsleiste liest die Einstellungen dieses Computers über Windows aus, ohne etwas zu
ändern:

- **Diesen PC in ein neues Profil einlesen**: Jede Regel, die sich auf einem laufenden Windows beurteilen
  lässt, wird eingeschaltet (wirksam), ausgeschaltet (teilweise oder nicht wirksam) oder behält ihren
  Zustand (nicht lesbar oder wie auf einem frisch installierten Windows); die Formulare übernehmen die
  Edition, die Zeitzone und die Sprachen.
- **Auswahl in dieses Profil einlesen** und **Auswahl auf diesem PC prüfen** wirken auf die ausgewählte
  Regel oder Gruppe.
- **Anwendungsskript speichern**, **Skript zum Zurücksetzen auf die Standardwerte speichern** oder
  **Prüfskript speichern** schreibt ein PowerShell-Skript für diesen oder einen anderen PC. Führen Sie ein
  Anwendungs- oder Rücksetzskript als Administrator aus, zuerst auf einem Test-PC; es legt eine Sicherung
  an, die `Undo-Apply.ps1` daneben wiederherstellt. Der Bericht eines Prüfskripts wird mit
  **Prüfbericht öffnen** geöffnet und wird zu einem Profil.

Regeln, die nur während der Installation oder bei der ersten Anmeldung eines Benutzers wirken, werden nicht
eingelesen.

## Profile vergleichen

**Menü, Mit einer Datei vergleichen** oder **Mit einer Vorlage vergleichen** öffnet eine Registerkarte mit
dem Vergleich: jeden Unterschied zwischen dem aktiven Profil und dem anderen, in drei Spalten. „Rechte Seite
anwenden“ setzt diese Werte in dem Profil, aus dem der Vergleich erstellt wurde; „Als Vergleichsdatei
speichern“ bewahrt die Unterschiede als `.wwdiff`-Datei auf, die sich später auf jedes Profil anwenden
lässt.

## Andere Dateien öffnen

Öffnen akzeptiert Profile (`.wwprofile`), Vergleiche (`.wwdiff`), Lesezeichendateien (`.wwmarks`),
Markdown-Seiten, von WinWeaver oder WinKickOff erstellte Antwortdateien (das darin eingebettete Profil),
WinKickOff-Profile und Prüfberichte (`.json`).

## Der MCP-Server

**Menü, MCP-Server** öffnet die Registerkarte des Servers, über den KI-Agents Profile lesen und, wenn der
Modus es zulässt, ändern. Der Server startet im Modus „Lesen“; „Bearbeiten“ erlaubt ihm, Profile im
Arbeitsspeicher zu ändern, „Dateien“ erlaubt ihm, sie zu speichern und die Antwortdatei zu schreiben. Das
Einlesen dieses PCs erfordert eine eigene Berechtigung, die bei jedem Start ausgeschaltet ist.

## Tastatur

| Tasten | Aktion |
|---|---|
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | Neu, Öffnen, Speichern, Speichern unter |
| F7, F9 | Profil prüfen, Antwortdatei erstellen |
| Ctrl+F, Esc | Suchen, Suche leeren und schließen |
| Alt+Left, Alt+Right | Zurück, Vorwärts |
| Ctrl+B | Lesezeichen für die Position setzen |
| Ctrl+D | Beschreibung einklappen oder anzeigen |
| Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab | Registerkarte schließen, nächste Registerkarte, vorherige Registerkarte |
| Esc | Bestätigung einer Änderung dieses PCs abbrechen |
| Ctrl+, | Einstellungen |
| F1 | Dieses Handbuch |

## Dateien neben der ausführbaren Datei

`settings.json` (Sprache, Design, zuletzt verwendete Dateien, die Registerkarten der letzten Sitzung),
`profiles/` (Ihre Profile), `output/` (die Antwortdatei und die Skripts), `drafts/` (nicht gespeicherte
Änderungen, die beim nächsten Start wiederhergestellt werden), `logs/` (was das Programm getan hat),
`bookmarks.wwmarks`. An anderer Stelle wird nichts geschrieben.

## Noch nicht verfügbar

Das direkte Anwenden von Einstellungen auf diesen PC aus dem Programm (verwenden Sie bis dahin das
Anwendungsskript) und der Import von Richtlinienvorlagen (ADMX) folgen in den nächsten Versionen.
