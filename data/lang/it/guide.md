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
- **Collegamenti**: ogni regola, gruppo, catalogo, file di profilo e chiave di registro che una vista
  nomina vi conduce: le regole di cui una regola ha bisogno, quelle che ne hanno bisogno e quelle con cui
  è in conflitto; le regole nominate nei messaggi; i gruppi sopra e dentro un gruppo; lo stesso criterio
  in altri cataloghi; le regole che scrivono una chiave di registro, nella scheda del registro e in un
  file di registro. Un collegamento in un profilo apre il suo albero fino all'elemento; da un'altra
  scheda mostra la regola nella prima scheda di profilo.
- **Barra di stato**: lo stato del documento, la versione del catalogo, il server MCP e l'ultimo messaggio.
- **Apri la cartella**: dopo una compilazione o il salvataggio di uno script, di un file di registro o di
  un profilo, un pulsante alla fine della barra di stato mostra il file nella sua cartella in Esplora
  file; Impostazioni, File apre le cartelle del programma.

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
- **Salva un file di registro** o **un file di registro di ripristino dei valori predefiniti** scrive i
  valori del registro della selezione in un file `.reg` per regedit o `reg import`; ciò che un file di
  registro non può contenere (passi di script, app, funzionalità) vi resta come commenti. I valori del
  profilo utente predefinito vanno in `HKEY_USERS\UnattendDefault`, e il commento all'inizio del file
  spiega come caricarvi il suo hive.

Le regole che agiscono solo durante l'installazione o al primo accesso di un utente non vengono lette.

Le funzionalità e le capacità di Windows vengono lette tramite DISM quando WinWeaver è eseguito come
amministratore, il che aggiunge qualche secondo alla lettura; senza questi diritti restano non leggibili.
La lettura avviene in background: la finestra continua a funzionare e un indicatore nella barra di stato la
mostra.

Applicata come amministratore, una regola che rimuove app o capacità di Windows le rimuove anche dalla
finestra: le capacità tramite DISM, le app per tutti gli utenti e per gli account creati in seguito. Il backup
le elenca, ma Annulla non le ripristina.

Anche i passaggi di script delle regole si applicano in modo nativo, senza PowerShell: .NET Framework 3.5
e i driver dal supporto di installazione, i tipi di avvio dei servizi, NetBIOS, le unità rimovibili nel
riquadro di spostamento, «la password non scade mai» per gli account del profilo, la disattivazione degli
account predefiniti Administrator e Guest (mai l'account in uso) e l'eliminazione delle copie del file di
risposte. Annulla non riporta indietro i programmi eseguiti, gli account modificati e i file eliminati;
la conferma dice quanti passaggi di questo tipo ha un piano.

## Confronto dei profili

**Menu, Confronta con un file** o **Confronta con una preimpostazione** apre una scheda di confronto:
ogni differenza tra il profilo attivo e l'altro, su tre colonne. "Applica il lato destro" imposta quei
valori nel profilo da cui è stato creato il confronto; "Salva come file di differenze" conserva le
differenze in un file `.wwdiff` applicabile in seguito a qualsiasi profilo.

## Apertura di altri file

Apri accetta profili (`.wwprofile`), confronti (`.wwdiff`), file di segnalibri (`.wwmarks`), pagine
Markdown, file di risposte generati da WinWeaver o WinKickOff (il profilo che incorporano), profili di
WinKickOff e report di controllo (`.json`).

Un file di registro (`.reg`) si apre in una scheda propria, come descrive la sezione sul registro.

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

**Menu, Registro** apre il registro di questo PC in una scheda: le chiavi a sinistra, i valori della
chiave scelta a destra con i loro tipi (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD,
REG_BINARY), il percorso nella riga sopra. Un doppio clic modifica un valore; Nuova chiave, Nuovo valore,
Modifica ed Elimina sono nella barra, e un'eliminazione chiede prima conferma. Le chiavi del computer
(HKEY_LOCAL_MACHINE) cambiano solo se WinWeaver gira come amministratore. Il pulsante accanto a una
chiave in **Che cosa scrive** la apre qui. **Segnalibro** conserva la chiave e il valore con un commento,
e il segnalibro elenca le regole che la scrivono. **Trova** nella barra cerca sotto la chiave scelta come
la ricerca di regedit: nei nomi delle chiavi, nei nomi dei valori e nei dati, solo la stringa intera se
richiesto. La ricerca avviene in background, **Arresta** la interrompe e un clic su un punto trovato lo
mostra.

**Autorizzazioni** nella barra mostra il proprietario e l'elenco di accesso della chiave selezionata: le
voci ereditate sono mostrate, le voci proprie della chiave cambiano (consenti o nega, controllo completo
o lettura, a cosa si applicano), un account o un gruppo di questo PC riceve una nuova voce e
l'ereditarietà dalla chiave superiore si può disattivare. Applica chiede prima; una chiave del computer
cambia solo con WinWeaver avviato come amministratore.

Un file di registro (`.reg`) si apre in una scheda propria, come struttura e non come testo: a sinistra
le chiavi e i commenti del file nel loro ordine, a destra la chiave selezionata con i suoi valori nei
tipi di regedit e i commenti tra di essi, un collegamento alla scheda del registro e le regole dei
cataloghi caricati che scrivono la chiave o il valore selezionato. **Nuova chiave**, **Nuovo valore**,
**Modifica** ed **Elimina** sono nella barra, e un'eliminazione chiede prima. **Salva** riscrive ogni
riga: i commenti e le righe che il programma non legge restano al loro posto. Un file salvato da **Questo
PC** nomina le sue regole nei commenti, e la freccia di un tale commento mostra la regola nel profilo.

## Il server MCP

**Menu, Server MCP** apre la scheda del server attraverso cui gli agenti di IA leggono i profili e, se la modalità lo consente, li cambiano. La scheda modifica un file di configurazione, `mcp/default.wwmcp` accanto al programma, e lo salva a ogni modifica: la porta e il token, la modalità (read legge soltanto, edit cambia i profili in memoria, files li salva anche e scrive il file di risposte), i permessi di leggere questo PC, di leggere o cambiare il registro e di cambiare i segnalibri e i cataloghi, il profilo con cui il server parte e il log dettagliato. **Apri** prende un altro file `.wwmcp` e **Salva come** ne fa una copia; `winweaver --mcp FILE` serve senza la finestra come dice il file. Il registro del server in funzione elenca ogni chiamata; **Mostra i dettagli** apre alla sua destra un pannello con gli argomenti e la risposta della chiamata scelta e pulsanti verso le regole, i profili, i cataloghi e le chiavi del registro che cita. Il log del server è `logs/mcp-<data>.jsonl`, un oggetto JSON per riga; il log dettagliato, spento per impostazione predefinita, conserva ogni richiesta e risposta per intero, senza password, chiavi e token.

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
| `--mcp FILE` | Serve MCP senza la finestra come dice un file di configurazione: un percorso a un file `.wwmcp` o un nome della cartella `mcp` accanto al programma |
| `--config FILE` | Con `--mcp stdio` o `--mcp http`: le impostazioni di un file di configurazione, anche il token; le altre opzioni le cambiano per questo avvio |
| `--detailed-log` | Con `--mcp`: il log conserva ogni richiesta e risposta per intero, senza password, chiavi e token |
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
all'avvio successivo), `mcp/` (le configurazioni del server MCP), `logs/` (cosa ha fatto il programma), `bookmarks.wwmarks`. Il programma non
scrive nulla altrove.

Le impostazioni, i segnalibri, le bozze e i file di differenze conservano il percorso di un file di
questa cartella relativo a essa, così l'intera cartella si può spostare o copiare su un altro PC; un file
altrove mantiene il percorso completo.

## Non ancora disponibile

Per ora nulla del piano: ciò che serve dopo al programma verrà dall'uso.
