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
  le immagini della stessa cartella.
- Un clic sull'immagine le dà i tasti: freccia sinistra e destra o Pag su e Pag giù prendono
  l'immagine precedente o successiva della cartella, Home e Fine la prima e l'ultima, + e -
  cambiano lo zoom, 0 adatta l'immagine, 1 mostra il 100%. Tab o Esc restituiscono i tasti alla
  finestra.

## Impostazioni

**Impostazioni**, **Plugin** elenca i plugin con le loro impostazioni: il file di testo più grande
da aprire; i pixel smussati oltre il 100% e il lato maggiore di un'immagine decodificata. Le
impostazioni stanno in `wview.settings.json` accanto al programma.

## Riga di comando

- `wview FILE...` apre i file in schede.
- `wview --plugins` elenca i plugin e i loro formati.
- `wview --probe FILE` apre un file e lo disegna senza finestra, con i tempi.
- `wview --help` elenca le opzioni.
