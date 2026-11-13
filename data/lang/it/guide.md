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
- **Il catalogo** comprende le regole integrate in tre parti e i modelli di criteri (ADMX) che importate,
  che diventano anch'essi regole. **Installazione** raccoglie ciò che fanno il file di risposte e i suoi
  script; **Criteri** ogni regola che scrive solo nel Registro di sistema e ogni importazione; **Altro**
  il resto: servizi, funzionalità, app, programmi e script su un Windows in esecuzione.
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

## Il nome del computer

Nel modulo **Installazione** il nome del computer è casuale (come fa Windows per impostazione
predefinita), fisso o un modello che il file di risposte espande su ogni PC durante l'installazione. Un
modello contiene lettere latine, cifre e trattini intorno a questi segnaposto:

| Segnaposto | Che cosa dà |
|---|---|
| `{serial}`, `{serial:N}` | Il numero di serie del computer dal suo firmware, lettere e cifre in maiuscolo; `:N` tiene gli ultimi N caratteri, senza `:N` tutti (fino a 15) |
| `{mac}`, `{mac:N}` | L'indirizzo MAC della prima scheda di rete fisica, 12 cifre esadecimali in maiuscolo; `:N` tiene le ultime N cifre, senza `:N` tutte e 12 |
| `{random}`, `{random:N}` | Maiuscole e cifre casuali senza le somiglianti I, O, 0 e 1; `:N` ne dà N, senza `:N` 6 |

N va da 1 a 15. Un numero di serie o un indirizzo illeggibile (assente, o il produttore ha lasciato un
segnaposto come `Default string`) diventa caratteri casuali della stessa lunghezza; uno più corto resta
intero. Il nome con la lunghezza piena di ogni segnaposto rientra in 15 caratteri, quindi `PC-{serial}` è
troppo lungo e `PC-{serial:12}` rientra; lasciate una lettera, perché Windows rifiuta un nome di sole
cifre. `PC-{random}` dà `PC-` e sei caratteri casuali, come `PC-K7Q2MX`; con il numero di serie
`5CG1234XYZ` e l'indirizzo `00155D3E5A01`, `WS-{serial:6}` dà `WS-234XYZ` e `{mac:4}-LAB` dà `5A01-LAB`.
Sotto il campo il modulo mostra che cosa dà il modello.

## Lingue di input

Il modulo **Lingue** elenca le lingue di input nel loro ordine; la prima è quella predefinita. Ogni
lingua mostra il suo nome e un elenco di layout di tastiera: prima il layout che Windows le assegna per
impostazione predefinita, poi tutti i layout di Windows per nome. **Aggiungi** trova una lingua da una
parte del nome o del tag fra tutte le lingue che Windows conosce e la aggiunge con il layout scelto; la
stessa lingua può comparire due volte con due layout, come Ucraino (avanzato) e Ucraino. Una lingua che
Windows numera solo al primo accesso, come Russo (Ucraina), riceve il layout che Windows le assegna
allora, e fino a quell'accesso il file di risposte nomina la lingua a cui appartiene. Le lingue e i
layout sono quelli del Windows su cui gira WinWeaver, con i nomi nella sua lingua.

La regola **Regione dell'utente** (`default-user.region`) del profilo utente predefinito contiene il GeoID e il codice di due lettere del paese dei nuovi utenti. I suoi due campi si scrivono liberamente, e l'elenco accanto a ciascuno contiene tutte le regioni che Windows conosce, filtrate da ciò che si scrive: una scelta imposta GeoID e codice insieme, e il nome della regione compare dopo i campi. Il controllo avvisa quando i due valori non indicano una stessa regione dell'elenco.

## File utenti

Un file utenti (`.wwusers`) elenca gli account locali da creare su un PC con le loro password, cifrate
con una password principale nello stesso file. **Menu, Nuovo, File utenti** ne inizia uno vuoto, **File
utenti con gli account di questo PC** uno con tutti gli account normali di questo PC (gruppo, stato,
scadenza della password, cartella del profilo; senza password). La scheda mostra gli account in un elenco
e quello selezionato in un modulo: nome, nome completo, descrizione, gruppo, cartella del profilo e le
istruzioni per un account che esiste già su un PC (impostare la password del file, abilitarlo o
disabilitarlo, esattamente questo gruppo); senza di esse un account esistente resta com'è. **Sblocca**
chiede la password principale, o la imposta quando il file non ne ha; **Password** imposta quella
dell'account selezionato; **Blocca** le dimentica. Senza la password principale le password non si
possono più leggere. **Verifica** elenca i problemi, **Leggi questo PC** aggiunge gli account di questo
PC che mancano al file. **Crea su questo PC** e **Applica le password** cambiano questo PC dopo una
domanda e richiedono WinWeaver eseguito come amministratore; il risultato sta sotto il modulo, una riga
per account. **Script** salva `Create-<nome>.ps1` accanto al file utenti: chiede da sé la password
principale e fa lo stesso su un altro PC da un prompt dei comandi aperto come amministratore
(`-PasswordsOnly` imposta solo le password). Nel modulo **Account** di un profilo, **Da un file utenti**
carica i suoi account, con le loro password se date la password principale. Un cambio del gruppo nel modulo spunta esattamente questo gruppo, così l'account passa tra Administrators e Users; l'account con cui gira WinWeaver resta in Administrators.

Finché il file è sbloccato, il modulo mostra la password dell'account selezionato come punti, con
**Mostra** e **Copia** (la cronologia degli appunti di Windows conserva ciò che viene copiato). Nel
modulo **Account** di un profilo, **In un file utenti** crea un file utenti dai suoi account; le loro
password attendono una password principale prima del primo salvataggio.

## La finestra

- **Schede**: ogni documento aperto è una scheda: un profilo, un confronto, questa guida, i segnalibri,
  il server MCP, le impostazioni. Il pulsante del menu all'estremità sinistra della riga delle schede
  contiene Nuovo, Apri, Salva, Confronta, la guida, il server MCP, i segnalibri, Impostazioni ed Esci.
- **Informazioni e Licenza** nel menu: la versione con il commit della build e la licenza (MIT), integrata
  nel programma, che resta quindi con `winweaver.exe` anche se si sposta solo quel file.
- **Barra dei comandi**: Nuovo, Apri, Salva, Verifica, Genera, Questo PC, Confronta. In una finestra
  stretta mostra solo le icone, nella riga di navigazione.
- **Riga di navigazione**: Indietro e Avanti scorrono gli elementi selezionati; il percorso mostra dove ti
  trovi e porta a qualsiasi livello; Segnalibro contrassegna la posizione; Cerca filtra l'albero mentre
  digiti e ricorda le ricerche fatte. La ricerca trova una regola da qualsiasi suo testo, in inglese e
  nella lingua dell'interfaccia, dalle sue chiavi di registro in qualsiasi forma (`HKLM\...` o
  `HKEY_LOCAL_MACHINE\...`), nomi dei valori e valori, e dai valori che le dà il profilo; le altre schede
  cercano in tutto ciò che mostrano.
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

- **Menu e schede**: ogni voce di un menu ha un'icona. Quando le schede sono più larghe della finestra,
  la riga scorre con la rotella, la scheda attiva torna in vista e un pulsante all'estremità destra
  elenca tutte le schede.
- **Solo attive** nella barra di un profilo mostra solo le regole attive con i loro gruppi aperti; gli
  strumenti MCP `list_groups`, `list_rules` e `search_rules` accettano `only_on` allo stesso modo.
- **Parametri**: i campi di una regola prendono la larghezza del pannello; un testo con un limite del file
  di risposta (i contatti del supporto) conta i suoi caratteri.
- **Lavoro in background**: una lettura di questo PC, un'importazione di modelli, una modifica di questo PC
  e una ricerca nel registro compaiono in un pannello in basso a destra con il tempo trascorso e Ferma
  dove il lavoro può fermarsi; il pannello si comprime in un indicatore.

## Questo PC

**Questo PC** nella barra dei comandi legge le impostazioni di questo computer tramite Windows, senza
modificare nulla:

**Il profilo.** Il modulo Profilo, primo nell'albero, modifica il nome, l'autore e il commento del profilo; la prima parte della riga di navigazione mostra il profilo con i suoi moduli in breve. Un modello si apre con la lingua della finestra e la regione del tuo Windows, e le lingue di input di questo Windows (English International se non si possono leggere). **Menu, Questo PC** funziona da ogni scheda: la lettura in un nuovo profilo non richiede un profilo aperto.

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

**Confronta, Modifiche non salvate** apre una scheda delle differenze del profilo dal suo file, o dalla
preimpostazione da cui viene: a sinistra il profilo com'è ora, a destra com'è stato salvato.

## Apertura di altri file

Apri accetta profili (`.wwprofile`), confronti (`.wwdiff`), file di segnalibri (`.wwmarks`), pagine
Markdown, file di risposte con un profilo incorporato (il profilo che incorporano), vecchi profili in
JSON e report di controllo (`.json`).

Un file di registro (`.reg`) si apre in una scheda propria, come descrive la sezione sul registro.

## Modelli di criteri

**Impostazioni, Modelli di criteri** importa i modelli ADMX di questo Windows, di una cartella o di un
file di catalogo; i loro criteri diventano regole in Criteri e le importazioni sono conservate in
`imports/`. Vengono letti e caricati solo le importazioni selezionate; le altre non occupano memoria. Un
profilo può contenere opzioni di un'importazione non caricata: restano invariate nel profilo, sono
elencate in **Opzioni sconosciute** alla fine dell'albero e sono escluse dalla generazione, con un avviso
all'apertura del profilo e alla generazione del file di risposte. **Carica cataloghi** in quel gruppo trova
le importazioni che le contengono e carica solo quelle. Un'impostazione (Generale, Finestra) nasconde il
gruppo.

Il menu **Cataloghi** nella barra di un profilo elenca ogni file di catalogo in `imports/`, caricato o no: uno caricato ha un segno di spunta e il colore di accento, e un clic apre la sua scheda; un altro si carica con un clic e apre anch'esso la sua scheda. L'elenco viene riletto ogni volta che il menu si apre. Per impostazione predefinita un clic su un catalogo del menu lo carica o lo scarica; in Impostazioni, Generale, l'altra scelta lascia nel menu solo i cataloghi caricati e un clic apre la scheda di uno.

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
modelli ne avvia uno vuoto tutto vostro. Un criterio si modifica per intero, come una regola del catalogo
integrato: titolo e descrizione, il suo gruppo, quando viene eseguito (un passaggio del file di risposte, lo
script dell'installazione, il profilo utente predefinito, il primo accesso, dopo OOBE), il suo livello, i
testi di **Dettagli**, i suoi parametri e le sue azioni di ogni tipo (valori del registro, impostazioni e
comandi del file di risposte, righe dello script, servizi, funzionalità, funzionalità su richiesta, app,
programmi), e in **Traduzioni** ogni testo in un'altra lingua, con quello inglese mostrato in un campo vuoto.
Un gruppo riceve titolo, descrizione e posto: un criterio del file di risposte sta sotto Installazione, uno
che scrive solo il registro sotto Criteri, il resto sotto Altro. **Applica** controlla tutto il file come si
controlla il catalogo integrato, e nulla cambia se il controllo fallisce. Salva (Ctrl+S) scrive il file e lo
ricarica. **Rimuovi il gruppo** rimuove il gruppo selezionato, dopo una domanda, con tutti i gruppi e i
criteri che contiene; il gruppo superiore del file resta. Un catalogo con script, comandi o programmi propri
si importa solo dopo una domanda: importate un catalogo così solo da una fonte di cui vi fidate. Il campo
**Id** con **Rinomina** dà a un criterio, a un gruppo o a tutto il catalogo un nuovo id in tutto il catalogo;
un nuovo id del catalogo rinomina il suo file e ogni id chiamato come il vecchio (`admx.custom-1.mute` diventa
`admx.office.mute`), e al salvataggio i profili aperti e i segnalibri seguono.

Il confronto si apre in una scheda: i criteri che differiscono nell'albero delle loro categorie, con
interruttori per quelli solo nel primo catalogo, solo nel secondo e i diversi, e il criterio selezionato
com'è in entrambi i cataloghi, fianco a fianco. **Salva il rapporto** scrive il rapporto come file
Markdown, **Come pagina** lo mostra come pagina. **Copia in**, nella barra di una scheda di catalogo,
copia il criterio selezionato o il gruppo selezionato con i gruppi sotto di esso in un catalogo aperto nel
suo editor (scegliete quale quando ce ne sono diversi) o in un nuovo catalogo, con gli stessi id e i testi
delle altre lingue: un catalogo con i soli criteri necessari, da passare come file.

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
Una chiave che il file elimina e poi scrive di nuovo (così si salva un elenco che sostituisce i valori
della sua chiave) compare una volta nell'albero, segnata ↻ come ricreata, con i valori che riceve.

Anche un file di Criteri di gruppo (`Registry.pol`) si apre in una scheda propria: i file del computer e
degli utenti di questo Windows (**Menu, Criteri di gruppo**; la barra di un profilo, di un catalogo e del
registro) o qualsiasi file `.pol`. Le chiavi sono un albero a sinistra, le voci della chiave selezionata a
destra con ciò che fa ciascuna. **Nuova chiave**, **Nuovo valore**, **Modifica** (o un doppio clic) e
**Rimuovi** cambiano le voci in una finestra sopra la scheda, come i valori di un file del registro;
**Radice** sceglie se un file fuori dai Criteri di gruppo vale per il computer o per l'utente (i file di
questo Windows lo prendono dalla loro cartella); **Aggiungi un file** prende le voci di un altro file di
Criteri di gruppo o le modifiche di un file del registro di quella radice; **In .reg** salva il file come
file del registro. Salva riscrive il formato dei Criteri di gruppo; i file di questo Windows si salvano
come copie altrove, perché li scrivono i Criteri di gruppo stessi. Le modifiche non salvate tornano da una
bozza dopo un riavvio.

## Il server MCP

**Menu, Server MCP** apre la scheda del server tramite cui gli agenti di IA leggono i profili e, quando la modalità lo consente, li modificano. La scheda modifica un file di configurazione, `mcp/default.wwmcp` accanto al programma, e lo salva a ogni modifica: la porta e il token, la modalità (read legge soltanto, edit modifica i profili in memoria, files li salva anche e scrive il file di risposte) e il registro dettagliato. In **Strumenti** ogni strumento si può disattivare da solo o con il suo gruppo, e la scheda di WinWeaver contiene le sue opzioni: la lettura di questo PC, la lettura o la modifica del registro, la modifica dei segnalibri e dei cataloghi e il profilo con cui parte il server. **Chiama uno strumento** esegue qualsiasi strumento per nome con i suoi argomenti, come farebbe un agente, e mostra la risposta. **Apri** prende un altro file `.wwmcp` e **Salva con nome** ne fa una copia (un file di una versione precedente di WinWeaver viene scritto nel nuovo formato all'apertura); `winweaver --mcp FILE` serve senza finestra come dice il file. Il giornale elenca ogni chiamata; **Mostra i dettagli** apre alla sua destra un pannello con gli argomenti e la risposta della chiamata scelta e pulsanti verso le regole, i profili, i cataloghi e le chiavi di registro che nomina. Il registro del server è `logs/mcp-<date>.jsonl`, un oggetto JSON per riga, e **Apri il registro** lo mostra; il registro dettagliato, spento per impostazione predefinita, conserva ogni richiesta e risposta intere, senza password, chiavi e token.

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
| `--build PROFILE` | Verifica un profilo e genera il suo file di risposte senza la finestra: `autounattend.xml` in `output`, con accanto gli script che incorpora; PROFILE è un file `.wwprofile`, una preimpostazione (`Office`, `Strict`, `Laptop`, `Home`) o il nome di un profilo di `profiles/`; un errore della verifica ferma la generazione |
| `--import-admx SOURCE` | Importa modelli di criteri come Impostazioni, Modelli di criteri: `system` (i modelli di questo Windows), una cartella di file ADMX, un file `.admx` con gli ADML delle cartelle delle sue lingue, un archivio `.zip` o un file `.wwcatalog`; il file di catalogo va in `imports/`, e la finestra e il server MCP lo caricano al loro prossimo avvio (da eseguire a finestra chiusa); un catalogo le cui regole eseguono codice proprio richiede `--code allow` |
| `--export-catalog SOURCE` | Scrive un file di catalogo (`.wwcatalog`) come Esporta in Impostazioni, Modelli di criteri: di un'importazione per il suo id (il nome del suo file in `imports/`), o di una sorgente di `--import-admx` letta senza importarla; `<id>.wwcatalog` in `output` |
| `--read-pc` | Senza `--mcp`: legge questo PC in un nuovo profilo con il nome del computer, come Questo PC, Leggi questo PC, e lo salva in `profiles/`; con un file `.xml` in `--out` scrive invece il file di risposte di quel profilo; le funzionalità e le capacità di Windows vengono lette solo quando il programma è eseguito come amministratore |
| `--base PROFILE` | Con `--read-pc`: il profilo da cui parte la lettura, indicato come per `--build`; per impostazione predefinita la preimpostazione Office |
| `--diff A B` | Scrive le differenze dal profilo A al profilo B come file di differenze, `A-vs-B.wwdiff` in `profiles/`; A e B si indicano come per `--build` |
| `--script KIND FILE` | Scrive senza la finestra lo script autonomo di un profilo (`.wwprofile`) o di una differenza (`.wwdiff`): `apply`, `revert` (ritorno ai valori predefiniti di Windows) o `audit`; accanto a uno script di applicazione o di ritorno va `Undo-Apply.ps1` |
| `--reg KIND FILE` | Scrive il file di registro dell'applicazione (`apply`) o del ritorno ai valori predefiniti (`revert`) di un profilo o di una differenza |
| `--only IDS` | Con `--script` o `--reg`: solo queste regole o gruppi, i loro id separati da virgole |
| `--out FILE` | Con `--build`, `--export-catalog`, `--read-pc`, `--diff`, `--script` o `--reg`: il file da scrivere, quando il valore termina con l'estensione che il comando scrive (`.xml`, `.wwcatalog`, `.wwprofile`, `.wwdiff`, `.ps1`, `.reg`); qualsiasi altro valore è una cartella, creata se manca, per il file con il suo nome abituale; `Undo-Apply.ps1` e gli script del file di risposte vanno accanto al file; senza `--out` i file vanno in `output` accanto al programma (`profiles/` per `--read-pc` e `--diff`) |
| `--help` | Stampa l'elenco dei parametri |
| `--smoke-window` | Apre una piccola finestra, disegna due fotogrammi e stampa con che cosa sono stati disegnati (anche con `--renderer`): un controllo della grafica |
| `--elevated-apply PLAN DIGEST` | La finestra avvia così se stessa per applicare un piano confermato come amministratore; non va avviato a mano |

Un comando che scrive file nomina ogni file scritto e termina con il codice 0; un uso errato dei parametri
termina con il codice 2, un errore con 1, e il motivo viene stampato.

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

`winweaver.settings.json` (lingua, tema, file recenti, le schede dell'ultima sessione), `profiles/` (i tuoi
profili), `output/` (il file di risposte e gli script), `drafts/` (le modifiche non salvate, ripristinate
all'avvio successivo), `mcp/` (le configurazioni del server MCP), `logs/` (cosa ha fatto il programma), `bookmarks.wwmarks`. Il programma non
scrive nulla altrove.

Le impostazioni, i segnalibri, le bozze e i file di differenze conservano il percorso di un file di
questa cartella relativo a essa, così l'intera cartella si può spostare o copiare su un altro PC; un file
altrove mantiene il percorso completo.

## Non ancora disponibile

Per ora nulla del piano: ciò che serve dopo al programma verrà dall'uso.
