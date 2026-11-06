# wview

wview affiche des fichiers. Chaque format est un module : une DLL du dossier `plugins` à côté de
`wview.exe`. Sans fichier, la fenêtre liste les modules et ce que chacun ouvre.

## Ouvrir des fichiers

- **Ouvrir** (Ctrl+O), ou un fichier déposé sur la fenêtre, ouvre le fichier dans un onglet ;
  `wview FICHIER...` ouvre des fichiers depuis la ligne de commande.
- Un fichier d'une extension qu'aucun module ne connaît, ou sans extension, s'ouvre selon son
  contenu : un texte, une image, un programme.
- Un fichier d'un format qu'aucun module n'ouvre reste fermé, et la barre d'état le dit.
- Ctrl+W ferme un onglet, Ctrl+Tab passe au suivant.
- Le menu contient les éléments de l'onglet actif et un sous-menu de chaque module qui a ses propres
  éléments.

## Texte et Markdown

Les pages Markdown (`.md`) s'affichent comme des pages, les fichiers texte (`.txt`, `.log`,
`.ini`, `.cfg`, `.csv`) tels quels. Ctrl+F cherche dans le texte. **Paramètres**, **Général** :
*Renvoyer les lignes à la largeur de la vue* et *Largeur des pages* changent l'affichage d'un
texte.

Les fichiers XML (`.xml`, `.manifest`) s'affichent aussi comme textes, et un texte UTF-16 avec sa
marque se lit comme un texte UTF-8.

Un fichier plus grand que **Plus grand fichier lu en entier** des paramètres (64 Mo au départ)
s'ouvre par parties : ses lignes sont comptées pendant la lecture, avec leurs numéros à gauche ; la
molette, les flèches, Page préc. et Page suiv., Début, Fin et la barre de droite le parcourent,
Gauche et Droite le décalent de côté. La recherche liste les lignes qui contiennent tous les mots,
et un clic sur l'une y mène.

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
- **Comparer avec une structure** compare le fichier avec un `.wwbin` enregistré de lui (d'une
  autre version du fichier, par exemple) : l'arbre des modifications, chacune avec sa valeur avant
  et après, les nœuds appariés par leurs noms et leurs clés, si bien qu'une nouvelle section n'en
  déplace aucune autre ; **Tous les nœuds** ou **Les modifications seules**. Un nœud choisi reçoit
  un commentaire et un signet ; **Enregistrer la comparaison** les écrit avec les deux fichiers
  dans un `.wwbindiff`, qui ouvre la comparaison dans un onglet à lui. Un `.wwbin` s'ouvre aussi
  seul, comme son arbre.
- **Contenu** dans la barre, quand le nœud choisi contient un fichier d'un type qu'un autre module
  affiche (le manifeste d'un programme, ses icônes, ses bitmaps et ses pages HTML, une image ou un
  texte parmi ses données, un autre programme à l'intérieur) : la partie droite l'affiche avec ce
  module, ses propres boutons au-dessus, et suit la sélection ; **Arbre et octets** revient. Dans le
  menu : **Ouvrir le contenu dans un onglet**, **Enregistrer le nœud comme fichier** (une icône
  comme fichier ICO, sinon ses octets) et **Lire tous les nœuds**.
- Un groupe d'icônes (`GROUP_ICON`) s'affiche comme une icône de toutes les tailles, les tailles
  comme des pages, et s'enregistre comme un fichier ICO.
- D'autres formats viennent comme descriptions, des fichiers `.wwformat` dans le dossier
  `plugins\formats` ; Paramètres, Modules liste les formats lus et les fichiers qui ne se sont pas
  lus.
- **Structures binaires**, **Dossier des descriptions** dans le menu ouvre le dossier
  `plugins\formats` dans l'Explorateur et le crée s'il n'existe pas.

## L'ensemble de Mandelbrot

L'exemple d'un module dans un autre langage que Rust : C, par l'interface stable de wview. Un
fichier `.mandel`, quelques lignes avec le centre, le zoom, les itérations, la palette et les
couleurs lissées, s'ouvre comme l'image de l'ensemble ; l'archive contient
`examples\seahorse.mandel`.

- Au-dessus de l'image : **Iterations**, **Palette** (classic, fire, ice, grey), **Smooth
  colours**, **Zoom in**, **Zoom out**, **Reset** et **Save**, qui réécrit la vue dans son fichier
  (les textes du module sont en anglais).
- La molette zoome sous le pointeur, un glisser déplace l'image, Maj et un glisser tracent un
  rectangle dans lequel le relâchement zoome, un double clic centre et agrandit, le bouton droit
  réduit.
- Après un clic sur l'image : les flèches la déplacent, + et - zooment, Page précédente et Page
  suivante doublent ou divisent par deux les itérations, P prend la palette suivante, Début
  réinitialise, Échap abandonne un rectangle.
- L'image vient par étapes, grossière tout de suite et plus fine pendant qu'elle se dessine ; la
  barre d'état nomme le centre, le zoom, les itérations et la durée de la dernière image.

## Paramètres

**Paramètres**, **Modules** liste les modules avec leurs paramètres : le plus grand fichier texte lu
en entier (un plus grand s'ouvre par parties); des pixels lissés au-delà de 100%, l'orientation
EXIF, une animation lue à son ouverture et la plus grande image décodée entière (64, 256 ou 1024
mégapixels) ; les octets par ligne, le bloc d'un fichier binaire et le cache de ses blocs. Les
paramètres sont gardés dans `wview.settings.json` à côté du programme.

## Ligne de commande

- `wview FICHIER...` ouvre les fichiers dans des onglets.
- `wview --plugins` liste les modules et leurs formats.
- `wview --probe FICHIER` ouvre un fichier et le dessine sans fenêtre, avec les temps.
- `wview --help` liste les options.
