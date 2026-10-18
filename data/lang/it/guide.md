# Guida di WinWeaver

WinWeaver prepara installazioni automatiche di Windows 10 e 11 e mantiene configurati i PC installati. Un
profilo dice quali regole del catalogo sono attive, con quali parametri e che cosa contengono i moduli
dell'installazione; da esso il programma genera il file di risposte `autounattend.xml` con i suoi script,
confronta i profili, legge le impostazioni di un PC in funzione e le applica. Il catalogo è scritto per
piccoli uffici senza dominio: prima la sicurezza e gli aggiornamenti.

Questa pagina si apre al primo avvio e in seguito dal menu o con F1. È un file Markdown; le pagine
della documentazione si aprono allo stesso modo, ognuna nella propria scheda.

## Come è organizzato

- **I profili** (`.wwprofile`) conservano le vostre scelte; le preimpostazioni Office, Strict, Laptop e
  Home sono punti di partenza.
- **Il catalogo** comprende le regole integrate in tre parti (Installazione, Criteri standard, Altro) e i
  modelli di criteri (ADMX) che importate, che diventano anch'essi regole.
- **Le schede**: ogni documento si apre nella propria scheda: profili, confronti, cataloghi, segnalibri, il
  server MCP, pagine come questa.
- **Questo PC** legge il Windows in funzione, vi controlla le regole, le applica con un backup e crea
  script per altri PC.
- **La cartella del programma** tiene tutto accanto a `winweaver.exe` (vedi la fine di questa pagina);
  nulla viene installato.

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
7. Provate il file di risposte in una macchina virtuale prima di qualsiasi PC reale: installate, poi leggete
   quel PC con **Questo PC** e confrontatelo con il profilo.
8. Mantenete in regola i PC installati: **Questo PC** applica le regole scelte come amministratore con un
   backup, oppure salva script autonomi che applicano, ripristinano, controllano e annullano su altri PC.

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

Le funzionalità e le capacità di Windows vengono lette tramite DISM quando WinWeaver è eseguito come
amministratore, il che aggiunge qualche secondo alla lettura; senza questi diritti restano non leggibili.
La lettura avviene in background: la finestra continua a funzionare e un indicatore nella barra di stato la
mostra.

Applicata come amministratore, una regola che rimuove capacità di Windows le rimuove anche tramite DISM; il
backup le elenca, ma Annulla non le ripristina. Le app vengono rimosse dallo script di applicazione.

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

Un criterio presente in due importazioni compare in entrambi gli alberi. **Confronta** nella barra di una
scheda di catalogo lo affianca a un'altra importazione caricata o a un file di catalogo aperto: i criteri
presenti solo in uno dei due e quelli con titolo, descrizione, valori o parametri diversi. Rimuovere
un'importazione, come ogni rimozione, chiede prima conferma.

Il pannello dei dettagli di un'opzione collega le schede dei cataloghi che la contengono e lo stesso
criterio negli alberi di altre importazioni; anche un segnalibro dell'opzione. In Impostazioni, Generale si
sceglie quali parti della descrizione sono aperte all'inizio.

**Modifica** nella barra di una scheda di catalogo lo apre in un editor, e **Nuovo catalogo** nel menu dei
modelli ne avvia uno vuoto tutto vostro. Un criterio riceve titolo, descrizione, il computer o il profilo
utente predefinito e i valori del registro che scrive o rimuove; **Applica** controlla tutto il file, e
nulla cambia se il controllo fallisce. Salva (Ctrl+S) scrive il file e lo ricarica.

## Il registro

**Menu, Registro** apre il registro di questo PC in una scheda: le chiavi a sinistra, i valori della chiave
scelta a destra con i loro tipi (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD, REG_BINARY), il
percorso nella riga sopra. Un doppio clic modifica un valore; Nuova chiave, Nuovo valore, Modifica ed
Elimina sono nella barra, e un'eliminazione chiede prima conferma. Le chiavi del computer
(HKEY_LOCAL_MACHINE) cambiano solo se WinWeaver gira come amministratore. Il pulsante accanto a una chiave
in **Che cosa scrive** la apre qui. **Segnalibro** conserva la chiave e il valore con un commento, e il
segnalibro elenca le regole che la scrivono. I permessi delle chiavi e la ricerca nel registro arriveranno
più avanti.

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
| `--registry NAME` | Con `--mcp`: `off` (predefinito), `read` lascia leggere il registro agli strumenti, `write` anche modificarlo |
| `--edit-library` | Con `--mcp`: gli strumenti possono modificare segnalibri e file di catalogo; leggerli è sempre consentito |
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

La rimozione di app direttamente dalla finestra: nel frattempo se ne occupa lo script di applicazione.
