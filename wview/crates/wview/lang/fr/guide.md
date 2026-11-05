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
  **Suivant**, les images du même dossier ; pour une animation **Lire** (ou **Pause**), **Image
  précédente de l'animation** et **Image suivante de l'animation** ; pour un fichier de plusieurs
  pages (TIFF, ICO) **Page précédente** et **Page suivante**.
- Un clic sur l'image lui donne les touches : les flèches déplacent une image plus grande que la
  fenêtre, sinon flèche gauche et droite prennent l'image précédente ou suivante du dossier, comme
  le font toujours Page précédente et Page suivante ; Début et Fin la première et la dernière ;
  + et - zooment par paliers, 0 ajuste l'image, 1 à 4 affichent 100% à 400% ; Espace lit une
  animation ou la met en pause, la virgule et le point vont à l'image ou à la page d'avant ou
  d'après. Tab ou Échap rendent les touches à la fenêtre.
- Une photo est tournée comme son appareil était tenu (l'orientation EXIF). Une grande image est
  décodée quand son onglet est déjà ouvert et dessinée par parties avec le détail dont le zoom a
  besoin ; une image au-delà de la limite des paramètres est décodée en plus petit, et la barre
  d'état le dit.

## Fichiers binaires

Les programmes et les bibliothèques de Windows (`.exe`, `.dll`, `.sys` et les autres fichiers du
Portable Executable) s'ouvrent comme l'arbre de leur structure : les en-têtes, les sections, les
importations et les exportations, chaque champ avec sa valeur, son décalage et sa taille.

- À gauche l'arbre : un clic ou les flèches choisissent un nœud, droite et gauche l'ouvrent et le
  ferment. À droite les octets du fichier, 16 par ligne avec leur ASCII : les octets du nœud choisi
  sont marqués, et un clic sur un octet choisit le plus petit nœud qui le contient.
- **Texte** dans la barre montre l'arbre comme une page, avec la recherche de l'onglet (Ctrl+F) ;
  **Arbre et octets** revient.
- Le fichier reste sur le disque : la vue lit les blocs qu'elle montre. Un nœud que le fichier ne
  contient pas en entier (un fichier court ou abîmé) porte un signe d'avertissement et dit
  pourquoi.
- En plus des en-têtes : les ressources (leurs types, noms et langues, un niveau lu à son
  ouverture), les exceptions de x64, les certificats, les relocalisations, le répertoire de
  débogage avec le chemin du PDB, TLS, la configuration de chargement, les importations différées
  et l'en-tête de .NET avec les flux de ses métadonnées.
- **Enregistrer la structure** écrit l'arbre dans un fichier `.wwbin`, à côté du fichier par
  défaut : chaque champ avec sa valeur, chaque plage d'octets bruts avec son SHA-256, le fichier
  avec sa taille et son SHA-256, et aucun de ses octets.
- D'autres formats viennent comme descriptions, des fichiers `.wwformat` dans le dossier
  `plugins\formats` ; Paramètres, Modules liste les formats lus et les fichiers qui ne se sont pas
  lus.

## Paramètres

**Paramètres**, **Modules** liste les modules avec leurs paramètres : le plus grand fichier texte à
ouvrir ; des pixels lissés au-delà de 100%, l'orientation EXIF, une animation lue à son ouverture et
la plus grande image décodée entière (64, 256 ou 1024 mégapixels) ; les octets par ligne, le bloc
d'un fichier binaire et le cache de ses blocs. Les paramètres sont gardés dans `wview.settings.json`
à côté du programme.

## Ligne de commande

- `wview FICHIER...` ouvre les fichiers dans des onglets.
- `wview --plugins` liste les modules et leurs formats.
- `wview --probe FICHIER` ouvre un fichier et le dessine sans fenêtre, avec les temps.
- `wview --help` liste les options.
