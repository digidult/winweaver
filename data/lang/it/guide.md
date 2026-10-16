# Guida di WinWeaver

WinWeaver prepara installazioni automatiche di Windows 11 Pro per piccoli gruppi di lavoro e mantiene
configurati i PC installati. Un profilo indica quali regole del catalogo sono attivate, con quali
parametri, e cosa contengono i moduli dell'installazione; da esso il programma genera `autounattend.xml`.

Questa pagina si apre al primo avvio e in seguito dal menu o con F1. È un file Markdown; le pagine
della documentazione si aprono allo stesso modo, ognuna nella propria scheda.

## Primi passi

1. Apri una preimpostazione: **Menu, Nuovo, Profilo dalla preimpostazione Office**. Office contiene le
   impostazioni predefinite del catalogo; Strict aggiunge restrizioni, Laptop è adatta ai PC portatili,
   Home è un elenco di elementi consentiti per un PC di casa.
2. Scorri l'albero a sinistra. Una casella di controllo attiva o disattiva una regola; la casella di un
   gruppo attiva o disattiva tutte le sue regole insieme; una regola in grigio attende una regola di cui
   ha bisogno. Il riquadro a destra mostra i parametri della regola selezionata e la sua descrizione.
3. Compila i moduli **Installazione**, **Account** e **Lingue** sotto la prima radice.
4. Verifica il profilo con F7. I messaggi compaiono sotto la riga di navigazione; un clic su un messaggio
   seleziona la regola o il modulo a cui si riferisce.
5. Genera il file di risposte con F9: `autounattend.xml` viene salvato nella cartella `output` accanto
   all'eseguibile, insieme agli script che incorpora, da consultare. Copia il file nella radice della
   chiavetta USB di installazione.
6. Salva il profilo con Ctrl+S. I profili sono file `.wwprofile` nella cartella `profiles` accanto
   all'eseguibile, oppure in qualsiasi posizione scelta con Salva con nome.

## La finestra

- **Schede**: ogni documento aperto è una scheda: un profilo, un confronto, questa guida, i segnalibri,
  il server MCP, le impostazioni. Il pulsante del menu all'estremità sinistra della riga delle schede
  contiene Nuovo, Apri, Salva, Confronta, la guida, il server MCP, i segnalibri, Impostazioni ed Esci.
- **Informazioni e Licenza** nel menu: la versione con il commit della build e la licenza (MIT), integrata
  nel programma, che resta quindi con `winweaver.exe` anche se si sposta solo quel file.
- **Barra dei comandi**: Nuovo, Apri, Salva, Verifica, Genera, Questo PC, Confronta. In una finestra
  stretta mostra solo le icone, nella riga di navigazione.
- **Riga di navigazione**: Indietro e Avanti scorrono gli elementi selezionati; il percorso mostra dove
  ti trovi e porta a qualsiasi livello; Segnalibro contrassegna la posizione; Cerca filtra l'albero
  mentre digiti e ricorda le ricerche fatte.
- **Albero e riquadro dei dettagli**: l'albero a sinistra, l'elemento selezionato a destra. La
  descrizione si comprime con Ctrl+D. Le regole di un gruppo selezionato mostrano un interruttore alla
  fine delle loro righe.
- **Barra di stato**: lo stato del documento, la versione del catalogo, il server MCP e l'ultimo messaggio.

## Questo PC

**Questo PC** nella barra dei comandi legge le impostazioni di questo computer tramite Windows, senza
modificare nulla:

- **Leggi questo PC in un nuovo profilo**: ogni regola che si può valutare su un Windows in esecuzione
  diventa attivata (in vigore), disattivata (in vigore solo in parte o non in vigore) oppure mantiene
  il suo stato (non leggibile, o come in un Windows appena installato); i moduli ricevono l'edizione, il
  fuso orario e le lingue.
- **Leggi la selezione in questo profilo** e **Verifica la selezione su questo PC** agiscono sulla
  regola o sul gruppo selezionato.
- **Salva uno script di applicazione**, **uno script di ripristino dei valori predefiniti** o **uno
  script di controllo** scrive uno script PowerShell per questo o un altro PC. Esegui uno script di
  applicazione o di ripristino come amministratore, prima su un PC di prova; salva un backup che
  `Undo-Apply.ps1`, posto accanto, ripristina. Il report di uno script di controllo si apre con **Apri un
  report di controllo** e diventa un profilo.

Le regole che agiscono solo durante l'installazione o al primo accesso di un utente non vengono lette.

## Confronto dei profili

**Menu, Confronta con un file** o **Confronta con una preimpostazione** apre una scheda di confronto:
ogni differenza tra il profilo attivo e l'altro, su tre colonne. "Applica il lato destro" imposta quei
valori nel profilo da cui è stato creato il confronto; "Salva come file di differenze" conserva le
differenze in un file `.wwdiff` applicabile in seguito a qualsiasi profilo.

## Apertura di altri file

Apri accetta profili (`.wwprofile`), confronti (`.wwdiff`), file di segnalibri (`.wwmarks`), pagine
Markdown, file di risposte generati da WinWeaver o WinKickOff (il profilo che incorporano), profili di
WinKickOff e report di controllo (`.json`).

## Modelli di criteri

**Impostazioni, Modelli di criteri** importa i modelli ADMX di questo Windows, di una cartella o di un
file di catalogo; i loro criteri diventano regole in Criteri standard e le importazioni sono conservate in
`imports/`. Vengono letti e caricati solo le importazioni selezionate; le altre non occupano memoria. Un
profilo può contenere opzioni di un'importazione non caricata: restano invariate nel profilo, sono
elencate in **Opzioni sconosciute** alla fine dell'albero e sono escluse dalla generazione, con un avviso
all'apertura del profilo e alla generazione del file di risposte. **Carica cataloghi** in quel gruppo trova
le importazioni che le contengono e carica solo quelle. Un'impostazione (Generale, Finestra) nasconde il
gruppo.

Un archivio zip di modelli, come li distribuiscono Chrome, Edge e Firefox, o di un file di catalogo si
importa con **Importa un archivio...** e si legge senza estrarlo.

## Il server MCP

**Menu, Server MCP** apre la scheda del server tramite il quale gli agenti di IA leggono i profili e,
quando la modalità lo consente, li modificano. Il server si avvia in modalità lettura; modifica gli
consente di cambiare i profili in memoria, file gli consente di salvarli e di scrivere il file di
risposte. La lettura di questo PC richiede un'autorizzazione a parte, disattivata a ogni avvio.

## Il renderer

La finestra disegna con OpenGL o con DirectX 12. **Automatico** (Impostazioni, Generale, Finestra) prova
prima OpenGL, che occupa meno memoria, e passa a DirectX 12 quando OpenGL non si avvia: in una macchina
virtuale senza accelerazione 3D, in un'installazione nuova senza il driver della scheda grafica, in alcune
sessioni remote. DirectX 12 disegna anche lì, con il rasterizzatore software di Windows. Se con OpenGL la
finestra viene disegnata male (un driver vecchio), scegli DirectX 12. Una modifica vale dall'avvio
successivo; `winweaver.exe --renderer directx` (oppure `opengl`, `auto`) sceglie per un solo avvio. Il log
in `logs/` indica con che cosa disegna la finestra.

## Riga di comando

`winweaver.exe` senza parametri apre la finestra. Un valore segue il suo parametro dopo uno spazio o `=`:
`--renderer=directx`.

| Parametri | Che cosa fanno |
|---|---|
| `--renderer NAME` | Apre la finestra e prova prima il renderer NAME, solo per questo avvio: `auto`, `opengl` o `directx` |
| `--mcp stdio` | Serve MCP sull'input e sull'output standard, senza la finestra; un client MCP lo avvia così |
| `--mcp http --token TOKEN` | Serve MCP su `http://127.0.0.1:8765/mcp` senza la finestra; il token ha almeno 32 caratteri |
| `--port N` | Con `--mcp http`: un'altra porta |
| `--mode NAME` | Con `--mcp`: `read` (predefinito) legge soltanto, `edit` modifica i profili in memoria, `files` li salva anche e scrive il file di risposte |
| `--read-pc` | Con `--mcp`: consente di leggere le impostazioni di questo PC |
| `--profile NAME` | Con `--mcp`: una preimpostazione o un profilo di `profiles/` diventa quello corrente |
| `--version` | Stampa la versione |
| `--check-catalog` | Carica il catalogo integrato con le sue traduzioni e stampa i loro problemi |
| `--help` | Stampa l'elenco dei parametri |
| `--smoke-window` | Apre una piccola finestra, disegna due fotogrammi e stampa con che cosa sono stati disegnati (anche con `--renderer`): un controllo della grafica |
| `--elevated-apply PLAN DIGEST` | La finestra avvia così se stessa per applicare un piano confermato come amministratore; non va avviato a mano |

La modalità e il permesso di leggere questo PC si danno a ogni avvio e non vengono mai salvati. I messaggi
vanno nella console da cui è stato avviato il programma e in `logs/`.

## Tastiera

| Tasti | Azione |
|---|---|
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | Nuovo, Apri, Salva, Salva con nome |
| F7, F9 | Verifica il profilo, genera il file di risposte |
| Ctrl+F, Esc | Cerca, cancella e chiudi la ricerca |
| Alt+Left, Alt+Right | Indietro, Avanti |
| Ctrl+B | Aggiungi la posizione ai segnalibri |
| Ctrl+D | Comprimi o mostra la descrizione |
| Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab | Chiudi la scheda, scheda successiva, scheda precedente |
| Esc | Annulla la conferma di una modifica di questo PC |
| Ctrl+, | Impostazioni |
| F1 | Questa guida |

## File accanto all'eseguibile

`settings.json` (lingua, tema, file recenti, le schede dell'ultima sessione), `profiles/` (i tuoi
profili), `output/` (il file di risposte e gli script), `drafts/` (le modifiche non salvate, ripristinate
all'avvio successivo), `logs/` (cosa ha fatto il programma), `bookmarks.wwmarks`. Il programma non
scrive nulla altrove.

## Non ancora disponibile

La lettura delle funzionalità e delle capacità di Windows direttamente dal programma: nel frattempo le
legge lo script di controllo.
