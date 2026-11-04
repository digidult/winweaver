# wview

wview affiche des fichiers. Chaque format est un module : une DLL du dossier `plugins` à côté de
`wview.exe`. Sans fichier, la fenêtre liste les modules et ce que chacun ouvre.

## Ouvrir des fichiers

- **Ouvrir** (Ctrl+O), ou un fichier déposé sur la fenêtre, ouvre le fichier dans un onglet ;
  `wview FICHIER...` ouvre des fichiers depuis la ligne de commande.
- Un fichier d'un format qu'aucun module n'ouvre reste fermé, et la barre d'état le dit.
- Ctrl+W ferme un onglet, Ctrl+Tab passe au suivant.

## Texte et Markdown

Les pages Markdown (`.md`) s'affichent comme des pages, les fichiers texte (`.txt`, `.log`,
`.ini`, `.cfg`, `.csv`) tels quels. Ctrl+F cherche dans le texte. **Paramètres**, **Général** :
*Renvoyer les lignes à la largeur de la vue* et *Largeur des pages* changent l'affichage d'un
texte.

## Images

Tous les formats que décode le Windows de l'ordinateur : BMP, PNG, JPEG, GIF, TIFF, ICO, et HEIF,
AVIF, WebP ou le RAW des appareils photo là où leurs extensions sont installées.

- La molette zoome sous le pointeur, un glisser déplace l'image, un double clic passe de l'image
  entière à 100% et retour.
- La barre : **Ajuster**, **100%**, **Zoom avant**, **Zoom arrière**, et **Précédent** et
  **Suivant**, les images du même dossier.
- Un clic sur l'image lui donne les touches : flèche gauche et droite ou Page précédente et Page
  suivante prennent l'image précédente ou suivante du dossier, Début et Fin la première et la
  dernière, + et - zooment, 0 ajuste l'image, 1 affiche 100%. Tab ou Échap rendent les touches à
  la fenêtre.

## Paramètres

**Paramètres**, **Modules** liste les modules avec leurs paramètres : le plus grand fichier texte à
ouvrir ; des pixels lissés au-delà de 100% et le plus grand côté d'une image décodée. Les
paramètres sont gardés dans `wview.settings.json` à côté du programme.

## Ligne de commande

- `wview FICHIER...` ouvre les fichiers dans des onglets.
- `wview --plugins` liste les modules et leurs formats.
- `wview --probe FICHIER` ouvre un fichier et le dessine sans fenêtre, avec les temps.
- `wview --help` liste les options.
