# WinWeaver-Handbuch

WinWeaver bereitet unbeaufsichtigte Installationen von Windows 10 und 11 vor und hält die installierten PCs
konfiguriert. Ein Profil sagt, welche Regeln des Katalogs an sind, mit welchen Parametern, und was die
Installationsformulare enthalten; daraus erstellt das Programm die Antwortdatei `autounattend.xml` mit
ihren Skripten, vergleicht Profile, liest die Einstellungen eines laufenden PCs und wendet sie auf ihn an.
Der Katalog ist für kleine Büros ohne Domäne geschrieben: Sicherheit und Updates zuerst.

Diese Seite wird beim ersten Start geöffnet und später über das Menü oder mit F1. Sie ist eine
Markdown-Datei; die Seiten der Dokumentation werden genauso geöffnet, jede in einer eigenen Registerkarte.

## Wie das Programm aufgebaut ist

- **Profile** (`.wwprofile`) enthalten Ihre Auswahl; die Vorlagen Office, Strict, Laptop und Home sind
  Ausgangspunkte.
- **Der Katalog** besteht aus den integrierten Regeln in drei Teilen und den Richtlinienvorlagen (ADMX),
  die Sie importieren und die ebenfalls zu Regeln werden. **Installation** enthält, was die Antwortdatei
  und ihre Skripts erledigen; **Richtlinien** jede Regel, die nur in die Registrierung schreibt, und jeden
  Import; **Sonstiges** den Rest: Dienste, Features, Apps, Programme und Skripts in einem laufenden
  Windows.
- **Registerkarten**: Jedes Dokument öffnet sich in einer eigenen Registerkarte: Profile, Vergleiche,
  Kataloge, Lesezeichen, der MCP-Server, Seiten wie diese.
- **Dieser PC** liest das laufende Windows, prüft Regeln darauf, wendet sie mit einer Sicherung an und
  erstellt Skripte für andere PCs.
- **Der Ordner des Programms** hält alles neben `winweaver.exe` (siehe das Ende dieser Seite); nichts
  wird installiert.

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
7. Testen Sie die Antwortdatei in einer virtuellen Maschine vor jedem echten PC: installieren, dann diesen
   PC mit **Dieser PC** lesen und mit dem Profil vergleichen.
8. Halten Sie die installierten PCs auf Linie: **Dieser PC** wendet die ausgewählten Regeln als
   Administrator mit einer Sicherung an oder speichert eigenständige Skripte zum Anwenden, Zurücksetzen,
   Prüfen und Rückgängigmachen auf anderen PCs.

## Der Name des Computers

Im Formular **Installation** ist der Computername zufällig (wie bei Windows standardmäßig), fest oder ein
Muster, das die Antwortdatei auf jedem PC während des Setups erweitert. Ein Muster enthält lateinische
Buchstaben, Ziffern und Bindestriche um diese Platzhalter:

| Platzhalter | Was er ergibt |
|---|---|
| `{serial}`, `{serial:N}` | Die Seriennummer des Computers aus seiner Firmware, Buchstaben und Ziffern in Großbuchstaben; `:N` behält die letzten N Zeichen, ohne `:N` alle (bis 15) |
| `{mac}`, `{mac:N}` | Die MAC-Adresse des ersten physischen Netzwerkadapters, 12 hexadezimale Ziffern in Großbuchstaben; `:N` behält die letzten N Ziffern, ohne `:N` alle 12 |
| `{random}`, `{random:N}` | Zufällige Großbuchstaben und Ziffern ohne die verwechselbaren I, O, 0 und 1; `:N` ergibt N davon, ohne `:N` 6 |

N reicht von 1 bis 15. Eine Seriennummer oder Adresse, die nicht lesbar ist (keine vorhanden, oder der
Hersteller hat einen Platzhalter wie `Default string` hinterlassen), wird zu zufälligen Zeichen derselben
Länge; eine kürzere bleibt ganz. Der Name mit der vollen Länge jedes Platzhalters passt in 15 Zeichen,
daher ist `PC-{serial}` zu lang und `PC-{serial:12}` passt; lassen Sie einen Buchstaben darin, denn
Windows lehnt einen Namen nur aus Ziffern ab. `PC-{random}` ergibt `PC-` und sechs zufällige Zeichen,
etwa `PC-K7Q2MX`; mit der Seriennummer `5CG1234XYZ` und der Adresse `00155D3E5A01` ergibt `WS-{serial:6}`
den Namen `WS-234XYZ` und `{mac:4}-LAB` den Namen `5A01-LAB`. Unter dem Feld zeigt das Formular, was das
Muster ergibt.

## Eingabesprachen

Das Formular **Sprachen** listet die Eingabesprachen in ihrer Reihenfolge; die erste ist die
Standardsprache. Jede Sprache zeigt ihren Namen und eine Liste der Tastaturlayouts: zuerst das Layout,
das Windows der Sprache standardmäßig gibt, dann alle Layouts von Windows nach ihren Namen.
**Hinzufügen** findet eine Sprache über einen Teil ihres Namens oder Tags unter allen Sprachen, die
Windows kennt, und fügt sie mit dem gewählten Layout hinzu; dieselbe Sprache kann zweimal mit zwei
Layouts vorkommen, etwa Ukrainisch (erweitert) und Ukrainisch. Eine Sprache, die Windows erst bei der
ersten Anmeldung nummeriert, etwa Russisch (Ukraine), erhält das Layout, das Windows ihr dann gibt, und
bis zu dieser Anmeldung nennt die Antwortdatei die Sprache, zu der sie gehört. Die Sprachen und Layouts
stammen aus dem Windows, auf dem WinWeaver läuft, mit ihren Namen in seiner Sprache.

Die Regel **Benutzerregion** (`default-user.region`) im Standardbenutzerprofil hält die GeoID und den Zweibuchstabencode des Landes neuer Benutzer. Beide Felder werden frei getippt, und die Liste neben jedem hält alle Regionen, die Windows kennt, gefiltert nach dem Getippten: eine Auswahl setzt GeoID und Code zugleich, und der Name der Region erscheint nach den Feldern. Die Prüfung warnt, wenn die beiden Werte nicht eine Region der Liste benennen.

## Benutzerdateien

Eine Benutzerdatei (`.wwusers`) listet die lokalen Konten, die auf einem PC anzulegen sind, mit ihren
Kennwörtern, die unter einem Masterkennwort in derselben Datei verschlüsselt sind. **Menü, Neu,
Benutzerdatei** beginnt eine leere, **Benutzerdatei mit den Konten dieses PCs** eine mit allen normalen
Konten dieses PCs (Gruppe, Zustand, Kennwortablauf, Profilordner; kein Kennwort). Die Registerkarte zeigt
die Konten in einer Liste und das gewählte in einem Formular: Name, vollständiger Name, Beschreibung,
Gruppe, Profilordner und die Anweisungen für ein Konto, das auf einem PC bereits besteht (Kennwort aus
der Datei setzen, aktivieren oder deaktivieren, genau diese Gruppe); ohne sie bleibt ein bestehendes
Konto, wie es ist. **Entsperren** fragt das Masterkennwort oder legt es fest, wenn die Datei keines hat;
**Kennwort** setzt das Kennwort des gewählten Kontos; **Sperren** vergisst sie. Ohne das Masterkennwort
lassen sich die Kennwörter nicht mehr lesen. **Prüfen** listet die Probleme, **Diesen PC lesen** fügt die
Konten dieses PCs hinzu, die der Datei fehlen. **Auf diesem PC anlegen** und **Kennwörter anwenden**
ändern diesen PC nach einer Rückfrage und brauchen WinWeaver als Administrator; das Ergebnis steht unter
dem Formular, eine Zeile je Konto. **Skript** speichert `Create-<Name>.ps1` neben der Benutzerdatei: es
fragt das Masterkennwort selbst und tut dasselbe auf einem anderen PC aus einer als Administrator
geöffneten Eingabeaufforderung (`-PasswordsOnly` setzt nur die Kennwörter). Im Formular **Konten** eines
Profils lädt **Aus einer Benutzerdatei** deren Konten, mit ihren Kennwörtern, wenn Sie das Masterkennwort
eingeben. Ein Wechsel der Gruppe im Formular markiert genau diese Gruppe, sodass das Konto zwischen Administratoren und Benutzern wechselt; das Konto, unter dem WinWeaver läuft, bleibt bei den Administratoren.

Solange die Datei entsperrt ist, zeigt das Formular das Kennwort des gewählten Kontos als Punkte, mit
**Anzeigen** und **Kopieren** (der Zwischenablageverlauf von Windows behält das Kopierte). Im Formular
**Konten** eines Profils erstellt **In eine Benutzerdatei** eine Benutzerdatei aus seinen Konten; ihre
Kennwörter warten vor dem ersten Speichern auf ein Masterkennwort.

## Das Fenster

- **Registerkarten**: Jedes geöffnete Dokument ist eine Registerkarte: ein Profil, ein Vergleich, dieses
  Handbuch, die Lesezeichen, der MCP-Server, die Einstellungen. Die Menüschaltfläche am linken Ende der
  Registerkartenleiste enthält Neu, Öffnen, Speichern, Vergleichen, das Handbuch, den MCP-Server, die
  Lesezeichen, Einstellungen und Beenden.
- **Info und Lizenz** im Menü: die Version mit dem Commit des Builds und die Lizenz (MIT), in das Programm
  eingebaut, sodass sie bei `winweaver.exe` bleibt, auch wenn nur diese Datei verschoben wird.
- **Befehlsleiste**: Neu, Öffnen, Speichern, Prüfen, Erstellen, Dieser PC, Vergleichen. In einem schmalen
  Fenster zeigt sie nur Symbole, und zwar in der Navigationszeile.
- **Navigationszeile**: Zurück und Vorwärts bewegen sich durch die Elemente, die Sie ausgewählt haben; die
  Pfadleiste zeigt, wo Sie sich befinden, und springt zu jeder Ebene; Lesezeichen markiert die Position;
  Suchen filtert die Struktur während der Eingabe und merkt sich, wonach Sie gesucht haben.
- **Struktur und Detailbereich**: links die Struktur, rechts das ausgewählte Element. Die Beschreibung wird
  mit Ctrl+D eingeklappt. Die Regeln einer ausgewählten Gruppe zeigen am Ende ihrer Zeilen einen Schalter.
- **Links**: jede Regel, Gruppe, jeder Katalog, jede Profildatei und jeder Registrierungsschlüssel, den
  eine Ansicht nennt, führt dorthin: die Regeln, die eine Regel braucht, die sie brauchen und mit denen
  sie in Konflikt steht; die in den Meldungen genannten Regeln; die Gruppen über und in einer Gruppe;
  dieselbe Richtlinie in anderen Katalogen; die Regeln, die einen Registrierungsschlüssel schreiben, in
  der Registerkarte der Registrierung und in einer Registrierungsdatei. Ein Link in einem Profil öffnet
  dessen Baum bis zum Element; aus einer anderen Registerkarte zeigt er die Regel in der ersten
  Profilregisterkarte.
- **Statusleiste**: der Zustand des Dokuments, die Katalogversion, der MCP-Server und die letzte Meldung.
- **Ordner öffnen**: nach einem Erstellen, einem gespeicherten Skript, einer Registrierungsdatei oder
  einem Profil zeigt eine Schaltfläche am Ende der Statusleiste die Datei in ihrem Ordner im Explorer;
  Einstellungen, Dateien öffnet die Ordner des Programms.

- **Menüs und Tabs**: jeder Menüpunkt hat ein Symbol. Sind die Tabs breiter als das Fenster, rollt die
  Zeile mit dem Rad, der aktive Tab kommt ins Bild, und ein Knopf am rechten Ende listet alle Tabs.
- **Nur aktive** in der Leiste eines Profils zeigt nur die aktiven Regeln mit geöffneten Gruppen; die
  MCP-Werkzeuge `list_groups`, `list_rules` und `search_rules` nehmen ebenso `only_on`.
- **Parameter**: die Felder einer Regel nehmen die Breite des Felds ein; ein Text mit einer Grenze der
  Antwortdatei (die Supportkontakte) zählt seine Zeichen dagegen.
- **Arbeit im Hintergrund**: das Lesen dieses PCs, ein Import von Richtlinienvorlagen, eine Änderung
  dieses PCs und eine Suche in der Registrierung erscheinen in einem Feld rechts unten mit der
  verstrichenen Zeit und Stopp, wo die Arbeit anhalten kann; das Feld klappt zu einem Kreisel ein.

## Dieser PC

**Dieser PC** in der Befehlsleiste liest die Einstellungen dieses Computers über Windows aus, ohne etwas zu
ändern:

**Das Profil.** Das Formular Profil, als erstes im Baum, bearbeitet Name, Autor und Kommentar des Profils; der erste Teil der Navigationszeile zeigt das Profil mit seinen Formularen kurz. Eine Vorlage öffnet sich mit der Sprache des Fensters und der Region Ihres Windows und den Eingabesprachen dieses Windows (English International, wenn sie nicht lesbar sind). **Menü, Dieser PC** wirkt von jedem Tab: Das Lesen in ein neues Profil braucht kein offenes Profil.

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
- **Registrierungsdatei speichern** oder **Registrierungsdatei zum Zurücksetzen auf die Standardwerte
  speichern** schreibt die Registrierungswerte der Auswahl als `.reg`-Datei für regedit oder `reg
  import`; was eine Registrierungsdatei nicht aufnehmen kann (Skriptschritte, Apps, Funktionen), bleibt
  darin als Kommentar. Die Werte des Standardbenutzerprofils gehen nach `HKEY_USERS\UnattendDefault`, und
  der Kommentar am Anfang der Datei sagt, wie seine Struktur dort geladen wird.

Regeln, die nur während der Installation oder bei der ersten Anmeldung eines Benutzers wirken, werden nicht
eingelesen.

Die Features und Funktionen von Windows werden über DISM gelesen, wenn WinWeaver als Administrator läuft;
das verlängert das Lesen um einige Sekunden. Ohne diese Rechte bleiben sie nicht lesbar.
Das Lesen läuft im Hintergrund: Das Fenster arbeitet weiter, und ein Kreisel in der Statusleiste zeigt es an.

Beim Anwenden als Administrator entfernt eine Regel, die Apps oder Funktionen von Windows entfernt, diese
auch direkt im Fenster: Funktionen über DISM, Apps für alle Benutzer und für später angelegte Konten. Die
Sicherung nennt sie, aber „Rückgängig machen“ bringt sie nicht zurück.

Auch die Skriptschritte der Regeln werden nativ angewendet, ohne PowerShell: .NET Framework 3.5 und die
Treiber vom Installationsmedium, die Starttypen von Diensten, NetBIOS, die Wechseldatenträger im
Navigationsbereich, „Kennwort läuft nie ab“ für die Konten des Profils, das Deaktivieren der integrierten
Konten Administrator und Gast (nie des Kontos, mit dem Sie arbeiten) und das Löschen der Kopien der
Antwortdatei. Rückgängig nimmt ausgeführte Programme, geänderte Konten und gelöschte Dateien nicht
zurück; die Bestätigung sagt, wie viele solche Schritte ein Plan hat.

## Profile vergleichen

**Menü, Mit einer Datei vergleichen** oder **Mit einer Vorlage vergleichen** öffnet eine Registerkarte mit
dem Vergleich: jeden Unterschied zwischen dem aktiven Profil und dem anderen, in drei Spalten. „Rechte Seite
anwenden“ setzt diese Werte in dem Profil, aus dem der Vergleich erstellt wurde; „Als Vergleichsdatei
speichern“ bewahrt die Unterschiede als `.wwdiff`-Datei auf, die sich später auf jedes Profil anwenden
lässt.

**Vergleichen, Ungespeicherte Änderungen** öffnet eine Registerkarte mit den Unterschieden des Profils zu
seiner Datei oder zu der Vorlage, aus der es stammt: links das Profil, wie es jetzt ist, rechts, wie es
gespeichert wurde.

## Andere Dateien öffnen

Öffnen akzeptiert Profile (`.wwprofile`), Vergleiche (`.wwdiff`), Lesezeichendateien (`.wwmarks`),
Markdown-Seiten, Antwortdateien mit eingebettetem Profil (das darin eingebettete Profil), ältere Profile
in JSON und Prüfberichte (`.json`).

Eine Registrierungsdatei (`.reg`) öffnet sich in einer eigenen Registerkarte, wie der Abschnitt über die
Registrierung beschreibt.

## Richtlinienvorlagen

**Einstellungen, Richtlinienvorlagen** importiert die ADMX-Vorlagen dieses Windows, eines Ordners oder
einer Katalogdatei; ihre Richtlinien werden zu Regeln unter Richtlinien, und die Importe liegen in
`imports/`. Nur die markierten Importe werden gelesen und geladen; die anderen belegen keinen Speicher.
Ein Profil kann Optionen eines Imports enthalten, der nicht geladen ist: Sie bleiben unverändert im
Profil, stehen unter **Unbekannte Optionen** am Ende der Struktur und werden beim Erstellen ausgelassen,
mit einer Warnung beim Öffnen des Profils und beim Erstellen der Antwortdatei. **Kataloge laden** in
dieser Gruppe findet die Importe, die sie enthalten, und lädt nur diese. Eine Einstellung (Allgemein,
Fenster) blendet die Gruppe aus.

Das Menü **Kataloge** in der Leiste eines Profils führt jede Katalogdatei in `imports/` auf, geladen oder nicht: eine geladene trägt ein Häkchen und die Akzentfarbe, ein Klick öffnet ihre Registerkarte; eine andere wird mit einem Klick geladen und öffnet ebenfalls ihre Registerkarte. Die Liste wird bei jedem Öffnen des Menüs neu gelesen. Standardmäßig lädt oder entlädt ein Klick auf einen Katalog des Menüs ihn; in Einstellungen, Allgemein, lässt die andere Wahl nur die geladenen Kataloge im Menü, und ein Klick öffnet den Tab eines Katalogs.

Ein Zip-Archiv mit Vorlagen, wie Chrome, Edge und Firefox sie ausliefern, oder mit einer Katalogdatei wird
mit **Archiv importieren...** importiert und ohne Entpacken gelesen.

Eine Richtlinie, die zwei Importe enthalten, erscheint in beiden Strukturen. **Vergleichen** in der Leiste
eines Katalog-Tabs stellt den Katalog einem anderen geladenen Import oder einer geöffneten Katalogdatei
gegenüber: die Richtlinien, die nur in einem davon stehen, und die, deren Titel, Beschreibung, Werte oder
Parameter sich unterscheiden. Das Entfernen eines Imports fragt, wie jedes Entfernen, zuerst nach.

Der Detailbereich einer Option verlinkt die Katalog-Tabs, die sie enthalten, und dieselbe Richtlinie in
den Strukturen anderer Importe; ein Lesezeichen der Option ebenso. Unter Einstellungen, Allgemein wird
gewählt, welche Teile der Beschreibung zuerst geöffnet sind.

**Bearbeiten** in der Leiste eines Katalog-Tabs öffnet den Katalog in einem Editor, und **Neuer Katalog** im
Menü der Vorlagen beginnt einen leeren eigenen. Eine Richtlinie bekommt ihren Titel, ihre Beschreibung, den
Computer oder das Standardbenutzerprofil und die Registrierungswerte, die sie schreibt oder entfernt;
**Übernehmen** prüft die ganze Datei, und nichts ändert sich, wenn die Prüfung fehlschlägt. Speichern
(Ctrl+S) schreibt die Datei und lädt sie neu. Eine gewählte Gruppe entfernt **Gruppe entfernen** nach einer Frage mit allen Gruppen und Richtlinien darin; die oberste Gruppe der Datei bleibt.

Der Vergleich öffnet sich in einem Tab: die abweichenden Richtlinien im Baum ihrer Kategorien, mit
Schaltern für die nur im ersten Katalog, nur im zweiten und die unterschiedlichen, und die gewählte
Richtlinie, wie sie in beiden Katalogen ist, nebeneinander. **Bericht speichern** schreibt den Bericht
als Markdown-Datei, **Als Seite** zeigt ihn als Seite. **Kopieren nach** in der Leiste eines Katalog-Tabs
kopiert die gewählte Richtlinie oder die gewählte Gruppe mit den Gruppen darunter in einen im Editor
offenen Katalog (bei mehreren wählen Sie welchen) oder in einen neuen Katalog, mit denselben Ids und den
Texten anderer Sprachen: ein Katalog nur mit den nötigen Richtlinien, als Datei weiterzugeben.

## Die Registrierung

**Menü, Registrierung** öffnet die Registrierung dieses PCs in einem Tab: die Schlüssel links, die Werte
des gewählten Schlüssels rechts mit ihren Typen (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD,
REG_QWORD, REG_BINARY), der Pfad in der Zeile darüber. Ein Doppelklick bearbeitet einen Wert; Neuer
Schlüssel, Neuer Wert, Bearbeiten und Löschen stehen in der Leiste, und ein Löschen fragt zuerst.
Schlüssel des Computers (HKEY_LOCAL_MACHINE) ändern sich nur, wenn WinWeaver als Administrator läuft. Die
Schaltfläche neben einem Schlüssel in **Was es schreibt** öffnet ihn hier. **Lesezeichen** hält den
Schlüssel und den Wert mit einem Kommentar fest, und das Lesezeichen nennt die Regeln, die ihn schreiben.
**Suchen** in der Leiste sucht unter dem gewählten Schlüssel wie die Suche von regedit: in den Namen von
Schlüsseln, den Namen von Werten und den Daten, auf Wunsch nur nach der ganzen Zeichenfolge. Die Suche
läuft im Hintergrund, **Beenden** bricht sie ab, und ein Klick auf eine Fundstelle zeigt sie.

**Berechtigungen** in der Leiste zeigt den Besitzer und die Zugriffsliste des gewählten Schlüssels:
geerbte Einträge werden gezeigt, die eigenen Einträge des Schlüssels ändern sich (zulassen oder
verweigern, Vollzugriff oder Lesen, wofür sie gelten), ein Konto oder eine Gruppe dieses PCs bekommt
einen neuen Eintrag, und die Vererbung vom übergeordneten Schlüssel lässt sich abschalten. Übernehmen
fragt zuerst; ein Schlüssel des Computers ändert sich nur, wenn WinWeaver als Administrator läuft.

Eine Registrierungsdatei (`.reg`) öffnet sich in einer eigenen Registerkarte, als Struktur statt als
Text: links die Schlüssel und Kommentare der Datei in ihrer Reihenfolge, rechts der gewählte Schlüssel
mit seinen Werten in den Typen von regedit und den Kommentaren dazwischen, ein Link zur Registerkarte der
Registrierung und die Regeln der geladenen Kataloge, die den Schlüssel oder den gewählten Wert schreiben.
**Neuer Schlüssel**, **Neuer Wert**, **Bearbeiten** und **Löschen** sind in der Leiste, und ein Löschen
fragt zuerst. **Speichern** schreibt jede Zeile zurück: die Kommentare und die Zeilen, die das Programm
nicht liest, bleiben an ihrem Platz. Eine über **Dieser PC** gespeicherte Datei nennt ihre Regeln in
Kommentaren, und der Pfeil eines solchen Kommentars zeigt die Regel im Profil.

Eine Gruppenrichtliniendatei (`Registry.pol`) öffnet sich ebenfalls in einem eigenen Tab: die Dateien des
Computers und der Benutzer dieses Windows (**Menü, Gruppenrichtlinie**; die Leiste eines Profils, eines
Katalogs und der Registrierung) oder jede `.pol`-Datei. Die Schlüssel sind links ein Baum, die Einträge
des gewählten Schlüssels rechts mit dem, was jeder tut. **Neuer Schlüssel**, **Neuer Wert**,
**Bearbeiten** (oder ein Doppelklick) und **Entfernen** ändern die Einträge in einem Fenster über dem
Tab, wie die Werte einer Registrierungsdatei; **Stamm** bestimmt, ob eine Datei außerhalb der
Gruppenrichtlinie für den Computer oder den Benutzer gilt (die Dateien dieses Windows nehmen es aus
ihrem Ordner); **Datei hinzufügen** übernimmt die Einträge einer anderen Gruppenrichtliniendatei oder
die Änderungen einer Registrierungsdatei dieses Stamms; **Als .reg** speichert die Datei als
Registrierungsdatei. Speichern schreibt das Format der Gruppenrichtlinie zurück; die Dateien dieses
Windows werden als Kopien anderswo gespeichert, da die Gruppenrichtlinie selbst sie schreibt.
Ungespeicherte Änderungen kommen nach einem Neustart aus einem Entwurf zurück.

## Der MCP-Server

**Menü, MCP-Server** öffnet den Tab des Servers, über den KI-Agenten Profile lesen und, wenn der Modus es erlaubt, ändern. Der Tab bearbeitet eine Konfigurationsdatei, `mcp/default.wwmcp` neben dem Programm, und speichert sie bei jeder Änderung: Port und Token, den Modus (read liest nur, edit ändert Profile im Speicher, files speichert sie auch und schreibt die Antwortdatei), die Erlaubnis, diesen PC zu lesen, die Registrierung zu lesen oder zu ändern und Lesezeichen und Kataloge zu ändern, das Profil, mit dem der Server beginnt, und das ausführliche Protokoll. **Öffnen** nimmt eine andere `.wwmcp`-Datei, **Speichern unter** legt eine Kopie an; `winweaver --mcp FILE` dient ohne Fenster so, wie die Datei es sagt. Das Protokoll des laufenden Servers listet jeden Aufruf; **Details einblenden** öffnet rechts davon einen Bereich mit den Argumenten und der Antwort des gewählten Aufrufs und Schaltflächen zu den Regeln, Profilen, Katalogen und Registrierungsschlüsseln, die er nennt. Das Protokoll des Servers ist `logs/mcp-<Datum>.jsonl`, ein JSON-Objekt pro Zeile; das ausführliche Protokoll, standardmäßig aus, hält jede Anfrage und Antwort vollständig fest, ohne Passwörter, Schlüssel und Token.

## Der Renderer

Das Fenster zeichnet mit OpenGL oder mit DirectX 12. **Automatisch** (Einstellungen, Allgemein, Fenster)
versucht zuerst OpenGL, das weniger Speicher braucht, und wechselt zu DirectX 12, wenn OpenGL nicht
startet: in einer virtuellen Maschine ohne 3D-Beschleunigung, in einer frischen Installation ohne Treiber
der Grafikkarte, in manchen Remotesitzungen. DirectX 12 zeichnet auch dort, mit dem Software-Rasterizer
von Windows. Zeichnet das Fenster mit OpenGL fehlerhaft (ein alter Treiber), wählen Sie DirectX 12. Eine
Änderung gilt ab dem nächsten Start; `winweaver.exe --renderer directx` (oder `opengl`, `auto`) wählt nur
für einen Start. Das Protokoll in `logs/` nennt, womit das Fenster zeichnet.

## Befehlszeile

`winweaver.exe` ohne Parameter öffnet das Fenster. Ein Wert folgt seinem Parameter nach einem Leerzeichen
oder `=`: `--renderer=directx`.

| Parameter | Was sie tun |
|---|---|
| `--renderer NAME` | Öffnet das Fenster und versucht zuerst den Renderer NAME, nur für diesen Start: `auto`, `opengl` oder `directx` |
| `--mcp stdio` | Bedient MCP über die Standardein- und -ausgabe, ohne Fenster; so startet es ein MCP-Client |
| `--mcp http --token TOKEN` | Bedient MCP auf `http://127.0.0.1:8765/mcp` ohne Fenster; das Token hat mindestens 32 Zeichen |
| `--port N` | Mit `--mcp http`: ein anderer Port |
| `--mode NAME` | Mit `--mcp`: `read` (Standard) liest nur, `edit` ändert Profile im Speicher, `files` speichert sie auch und schreibt die Antwortdatei |
| `--read-pc` | Mit `--mcp`: erlaubt das Lesen der Einstellungen dieses PCs |
| `--registry NAME` | Mit `--mcp`: `off` (Standard), `read` lässt die Tools die Registrierung lesen, `write` sie auch ändern |
| `--edit-library` | Mit `--mcp`: die Tools dürfen Lesezeichen und Katalogdateien ändern; lesen dürfen sie sie immer |
| `--profile NAME` | Mit `--mcp`: eine Vorlage oder ein Profil aus `profiles/` wird das aktuelle |
| `--mcp FILE` | Bedient MCP ohne Fenster so, wie eine Konfigurationsdatei es sagt: ein Pfad zu einer `.wwmcp`-Datei oder ein Name aus dem Ordner `mcp` neben dem Programm |
| `--config FILE` | Mit `--mcp stdio` oder `--mcp http`: die Einstellungen einer Konfigurationsdatei, auch das Token; die anderen Optionen ändern sie für diesen Start |
| `--detailed-log` | Mit `--mcp`: das Protokoll hält jede Anfrage und Antwort vollständig fest, ohne Passwörter, Schlüssel und Token |
| `--version` | Gibt die Version aus |
| `--check-catalog` | Lädt den integrierten Katalog mit seinen Übersetzungen und gibt ihre Probleme aus |
| `--build PROFILE` | Prüft ohne Fenster ein Profil und erstellt seine Antwortdatei: `autounattend.xml` in `output`, daneben die Skripts, die sie einbettet; PROFILE ist eine `.wwprofile`-Datei, eine Vorlage (`Office`, `Strict`, `Laptop`, `Home`) oder der Name eines Profils aus `profiles/`; ein Fehler der Prüfung stoppt die Erstellung |
| `--import-admx SOURCE` | Importiert Richtlinienvorlagen wie Einstellungen, Richtlinienvorlagen: `system` (die Vorlagen dieses Windows), einen Ordner mit ADMX-Dateien, eine `.admx`-Datei mit den ADML ihrer Sprachordner, ein `.zip`-Archiv oder eine `.wwcatalog`-Datei; die Katalogdatei kommt nach `imports/`, und das Fenster und der MCP-Server laden sie bei ihrem nächsten Start (bei geschlossenem Fenster ausführen) |
| `--export-catalog SOURCE` | Schreibt eine Katalogdatei (`.wwcatalog`) wie Exportieren in Einstellungen, Richtlinienvorlagen: eines Imports nach seiner ID (dem Namen seiner Datei in `imports/`) oder einer Quelle von `--import-admx`, gelesen ohne Import; `<id>.wwcatalog` in `output` |
| `--read-pc` | Ohne `--mcp`: liest diesen PC in ein neues Profil mit dem Namen des Computers, wie Dieser PC, Diesen PC lesen, und speichert es in `profiles/`; mit einer `.xml`-Datei in `--out` schreibt es stattdessen die Antwortdatei dieses Profils; die Features und Funktionen von Windows werden nur gelesen, wenn das Programm als Administrator läuft |
| `--base PROFILE` | Mit `--read-pc`: das Profil, von dem das Lesen ausgeht, angegeben wie bei `--build`; standardmäßig die Vorlage Office |
| `--diff A B` | Schreibt die Unterschiede vom Profil A zum Profil B als Unterschiedsdatei, `A-vs-B.wwdiff` in `profiles/`; A und B werden wie bei `--build` angegeben |
| `--script KIND FILE` | Schreibt ohne Fenster das eigenständige Skript eines Profils (`.wwprofile`) oder eines Unterschieds (`.wwdiff`): `apply`, `revert` (zurück zu den Standardwerten von Windows) oder `audit`; neben ein Anwendungs- oder Rückkehrskript kommt `Undo-Apply.ps1` |
| `--reg KIND FILE` | Schreibt die Registrierungsdatei der Anwendung (`apply`) oder der Rückkehr zu den Standardwerten (`revert`) eines Profils oder eines Unterschieds |
| `--only IDS` | Mit `--script` oder `--reg`: nur diese Regeln oder Gruppen, ihre IDs durch Kommas getrennt |
| `--out FILE` | Mit `--build`, `--export-catalog`, `--read-pc`, `--diff`, `--script` oder `--reg`: die zu schreibende Datei, wenn der Wert mit der Endung endet, die der Befehl schreibt (`.xml`, `.wwcatalog`, `.wwprofile`, `.wwdiff`, `.ps1`, `.reg`); jeder andere Wert ist ein Ordner, bei Bedarf angelegt, für die Datei mit ihrem üblichen Namen; `Undo-Apply.ps1` und die Skripts der Antwortdatei kommen neben die Datei; ohne `--out` gehen die Dateien nach `output` neben dem Programm (`profiles/` für `--read-pc` und `--diff`) |
| `--help` | Gibt die Liste der Parameter aus |
| `--smoke-window` | Öffnet ein kleines Fenster, zeichnet zwei Bilder und gibt aus, womit sie gezeichnet wurden (auch mit `--renderer`): eine Prüfung der Grafik |
| `--elevated-apply PLAN DIGEST` | So startet das Fenster sich selbst, um einen bestätigten Plan als Administrator anzuwenden; nicht zum Starten von Hand |

Ein Befehl, der Dateien schreibt, nennt jede geschriebene Datei und endet mit dem Code 0; eine falsche
Verwendung der Parameter endet mit dem Code 2, ein Fehler mit 1, und der Grund wird ausgegeben.

Der Modus und die Erlaubnis, diesen PC zu lesen, werden bei jedem Start angegeben und nie gespeichert. Die
Meldungen gehen an die Konsole, aus der das Programm gestartet wurde, und nach `logs/`.

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

`winweaver.settings.json` (Sprache, Design, zuletzt verwendete Dateien, die Registerkarten der letzten Sitzung),
`profiles/` (Ihre Profile), `output/` (die Antwortdatei und die Skripts), `drafts/` (nicht gespeicherte
Änderungen, die beim nächsten Start wiederhergestellt werden), `mcp/` (die Konfigurationen des MCP-Servers), `logs/` (was das Programm getan hat),
`bookmarks.wwmarks`. An anderer Stelle wird nichts geschrieben.

Einstellungen, Lesezeichen, Entwürfe und Unterschiedsdateien speichern den Pfad einer Datei in diesem
Ordner relativ zu ihm, sodass der ganze Ordner verschoben oder auf einen anderen PC kopiert werden kann;
eine Datei an anderer Stelle behält ihren vollständigen Pfad.

## Noch nicht verfügbar

Aus dem Plan fehlt derzeit nichts: was das Programm als Nächstes braucht, zeigt die Praxis.
