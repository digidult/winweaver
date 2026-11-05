# wview

wview mostra i file. Ogni formato è un plugin: una DLL nella cartella `plugins` accanto a
`wview.exe`. Senza un file la finestra elenca i plugin e ciò che apre ciascuno.

## Aprire i file

- **Apri** (Ctrl+O), o un file trascinato sulla finestra, apre il file in una scheda;
  `wview FILE...` apre file dalla riga di comando.
- Un file di un formato che nessun plugin apre resta chiuso, e la barra di stato lo dice.
- Ctrl+W chiude una scheda, Ctrl+Tab passa alla successiva.

## Testo e Markdown

Le pagine Markdown (`.md`) si mostrano come pagine, i file di testo (`.txt`, `.log`, `.ini`,
`.cfg`, `.csv`) così come sono. Ctrl+F cerca nel testo. **Impostazioni**, **Generale**: *Manda a
capo le righe alla larghezza della vista* e *Larghezza delle pagine* cambiano l'aspetto di un
testo.

## Immagini

Ogni formato che il Windows del computer decodifica: BMP, PNG, JPEG, GIF, TIFF, ICO, e HEIF, AVIF,
WebP o il RAW delle fotocamere dove le loro estensioni sono installate.

- La rotellina ingrandisce sotto il puntatore, un trascinamento sposta l'immagine, un doppio clic
  passa dall'immagine intera al 100% e viceversa.
- La barra: **Adatta**, **100%**, **Ingrandisci**, **Riduci**, e **Precedente** e **Successivo**,
  le immagini della stessa cartella; per un'animazione **Riproduci** (o **Pausa**), **Fotogramma
  precedente** e **Fotogramma successivo**; per un file di più pagine (TIFF, ICO) **Pagina
  precedente** e **Pagina successiva**.
- Un clic sull'immagine le dà i tasti: le frecce spostano un'immagine più grande della finestra,
  altrimenti freccia sinistra e destra prendono l'immagine precedente o successiva della cartella,
  come fanno sempre Pag su e Pag giù; Home e Fine la prima e l'ultima; + e - cambiano lo zoom a
  passi, 0 adatta l'immagine, da 1 a 4 mostrano dal 100% al 400%; la barra spaziatrice riproduce o
  mette in pausa un'animazione, la virgola e il punto vanno al fotogramma o alla pagina prima o
  dopo. Tab o Esc restituiscono i tasti alla finestra.
- Una foto viene ruotata come era tenuta la sua fotocamera (l'orientamento EXIF). Un'immagine
  grande viene decodificata con la sua scheda già aperta e disegnata a parti con il dettaglio che
  serve allo zoom; un'immagine oltre il limite delle impostazioni viene decodificata più piccola, e
  la barra di stato lo dice.

## File binari

I programmi e le librerie di Windows (`.exe`, `.dll`, `.sys` e gli altri file del Portable
Executable) si aprono come l'albero della loro struttura: le intestazioni, le sezioni, le
importazioni e le esportazioni, ogni campo con il suo valore, il suo offset e la sua dimensione.

- A sinistra l'albero: un clic o le frecce scelgono un nodo, destra e sinistra lo aprono e lo
  chiudono. A destra i byte del file, 16 per riga con il loro ASCII: i byte del nodo scelto sono
  segnati, e un clic su un byte sceglie il nodo più piccolo che lo contiene.
- **Testo** nella barra mostra l'albero come una pagina, con la ricerca della scheda (Ctrl+F);
  **Albero e byte** torna indietro.
- Il file resta sul disco: la vista legge i blocchi che mostra. Un nodo che il file non contiene
  per intero (un file corto o danneggiato) ha un segno di avviso e dice perché.
- Altri formati arrivano come descrizioni, file `.wwformat` nella cartella `plugins\formats`;
  Impostazioni, Plugin elenca i formati letti e i file che non si sono letti.

## Impostazioni

**Impostazioni**, **Plugin** elenca i plugin con le loro impostazioni: il file di testo più grande
da aprire; i pixel smussati oltre il 100%, l'orientamento EXIF, un'animazione riprodotta
all'apertura e l'immagine più grande decodificata intera (64, 256 o 1024 megapixel); i byte per
riga, il blocco di un file binario e la cache dei suoi blocchi. Le impostazioni stanno in
`wview.settings.json` accanto al programma.

## Riga di comando

- `wview FILE...` apre i file in schede.
- `wview --plugins` elenca i plugin e i loro formati.
- `wview --probe FILE` apre un file e lo disegna senza finestra, con i tempi.
- `wview --help` elenca le opzioni.
