# Guide de WinWeaver

WinWeaver prépare des installations sans assistance de Windows 11 Pro pour les petits groupes de
travail et maintient la configuration des PC installés. Un profil indique quelles règles du catalogue
sont activées, avec quels paramètres, et ce que contiennent les formulaires d'installation ; le
programme génère `autounattend.xml` à partir de ce profil.

Cette page s'ouvre au premier démarrage, puis depuis le menu ou avec F1. C'est un fichier Markdown ;
les pages de documentation s'ouvrent de la même façon, chacune dans son propre onglet.

## Premiers pas

1. Ouvrez un préréglage : **Menu, Nouveau, Profil à partir du préréglage Office**. Office contient les
   valeurs par défaut du catalogue ; Strict ajoute des restrictions, Laptop convient aux PC portables,
   Home est une liste d'autorisation pour un PC domestique.
2. Parcourez l'arborescence à gauche. Une case à cocher active ou désactive une règle ; la case d'un
   groupe active ou désactive toutes ses règles à la fois ; une règle grisée attend une règle dont elle
   a besoin. Le volet de droite affiche les paramètres de la règle sélectionnée et sa description.
3. Remplissez les formulaires **Installation**, **Comptes** et **Langues** sous la première racine.
4. Vérifiez le profil avec F7. Les messages s'affichent sous la ligne de navigation ; un clic sur un
   message sélectionne la règle ou le formulaire concerné.
5. Générez le fichier de réponses avec F9 : `autounattend.xml` est placé dans le dossier `output` à
   côté de l'exécutable, avec les scripts qu'il intègre, pour que vous puissiez les consulter. Copiez
   le fichier à la racine de la clé USB d'installation.
6. Enregistrez le profil avec Ctrl+S. Les profils sont des fichiers `.wwprofile` dans le dossier
   `profiles` à côté de l'exécutable, ou à l'emplacement de votre choix avec Enregistrer sous.

## La fenêtre

- **Onglets** : chaque document ouvert est un onglet : un profil, une comparaison, ce guide, les
  signets, le serveur MCP, les paramètres. Le bouton de menu à l'extrémité gauche de la rangée
  d'onglets contient Nouveau, Ouvrir, Enregistrer, Comparer, le guide, le serveur MCP, les signets,
  Paramètres et Quitter.
- **Barre de commandes** : Nouveau, Ouvrir, Enregistrer, Vérifier, Générer, Ce PC, Comparer. Dans une
  fenêtre étroite, elle n'affiche que des icônes, dans la ligne de navigation.
- **Ligne de navigation** : Précédent et Suivant parcourent les éléments que vous avez sélectionnés ;
  le fil d'Ariane indique où vous êtes et permet d'atteindre n'importe quel niveau ; Signet marque la
  position ; Rechercher filtre l'arborescence au fil de la saisie et mémorise vos recherches.
- **Arborescence et volet de détails** : l'arborescence à gauche, l'élément sélectionné à droite. La
  description se réduit avec Ctrl+D. Les règles d'un groupe sélectionné affichent un commutateur en fin
  de ligne.
- **Barre d'état** : l'état du document, la version du catalogue, le serveur MCP et le dernier message.

## Ce PC

**Ce PC** dans la barre de commandes lit les paramètres de cet ordinateur par l'intermédiaire de
Windows, sans rien modifier :

- **Lire ce PC dans un nouveau profil** : chaque règle qui peut être évaluée sur un Windows en cours
  d'exécution devient activée (en vigueur), désactivée (partiellement ou pas en vigueur) ou garde son
  état (illisible, ou comme sur un Windows neuf) ; les formulaires reprennent l'édition, le fuseau
  horaire et les langues.
- **Lire la sélection dans ce profil** et **Vérifier la sélection sur ce PC** portent sur la règle ou
  le groupe sélectionné.
- **Enregistrer un script d'application**, **un script de retour aux valeurs par défaut** ou **un
  script d'audit** écrit un script PowerShell pour ce PC ou pour un autre. Exécutez un script
  d'application ou de retour en tant qu'administrateur, d'abord sur un PC de test ; il enregistre une
  sauvegarde que le script `Undo-Apply.ps1` placé à côté restaure. Le rapport d'un script d'audit
  s'ouvre avec **Ouvrir un rapport d'audit** et devient un profil.

Les règles qui n'agissent que pendant l'installation ou à la première connexion d'un utilisateur ne
sont pas lues.

## Comparer des profils

**Menu, Comparer avec un fichier** ou **Comparer avec un préréglage** ouvre un onglet de comparaison :
chaque différence entre le profil actif et l'autre, sur trois colonnes. « Appliquer le côté droit »
définit ces valeurs dans le profil à partir duquel la comparaison a été faite ; « Enregistrer en tant
que fichier de différences » conserve les différences dans un fichier `.wwdiff` qui peut ensuite
s'appliquer à n'importe quel profil.

## Ouvrir d'autres fichiers

Ouvrir accepte les profils (`.wwprofile`), les fichiers de différences (`.wwdiff`), les fichiers de
signets (`.wwmarks`), les pages Markdown, les fichiers de réponses générés par WinWeaver ou WinKickOff
(le profil qu'ils contiennent), les profils WinKickOff et les rapports d'audit (`.json`).

## Modèles de stratégie

**Paramètres, Modèles de stratégie** importe les modèles ADMX de ce Windows, d'un dossier ou d'un fichier
de catalogue ; leurs stratégies deviennent des règles sous Stratégies standard, et les importations sont
gardées dans `imports/`. Seules les importations cochées sont lues et chargées ; les autres n'occupent pas
de mémoire. Un profil peut contenir des options d'une importation non chargée : elles restent inchangées
dans le profil, sont listées sous **Options inconnues** à la fin de l'arborescence et sont exclues de la
génération, avec un avertissement à l'ouverture du profil et à la génération du fichier de réponses.
**Charger les catalogues** dans ce groupe trouve les importations qui les contiennent et ne charge que
celles-ci. Un paramètre (Général, Fenêtre) masque le groupe.

## Le serveur MCP

**Menu, Serveur MCP** ouvre l'onglet du serveur par lequel les agents IA lisent les profils et, si le
mode le permet, les modifient. Le serveur démarre en mode lecture ; le mode modification lui permet de
modifier les profils en mémoire, le mode fichiers de les enregistrer et d'écrire le fichier de
réponses. La lecture de ce PC nécessite sa propre autorisation, désactivée à chaque démarrage.

## Le moteur de rendu

La fenêtre dessine avec OpenGL ou avec DirectX 12. **Automatique** (Paramètres, Général, Fenêtre) essaie
d'abord OpenGL, qui occupe moins de mémoire, et passe à DirectX 12 quand OpenGL ne démarre pas : dans une
machine virtuelle sans accélération 3D, dans une installation neuve sans le pilote de la carte graphique,
dans certaines sessions à distance. DirectX 12 dessine aussi là, avec le rastériseur logiciel de Windows.
Si la fenêtre s'affiche mal avec OpenGL (un ancien pilote), choisissez DirectX 12. Un changement prend
effet au démarrage suivant ; `winweaver.exe --renderer directx` (ou `opengl`, `auto`) choisit pour un seul
démarrage. Le journal de `logs/` indique avec quoi la fenêtre dessine.

## Ligne de commande

`winweaver.exe` sans paramètre ouvre la fenêtre. Une valeur suit son paramètre après une espace ou `=` :
`--renderer=directx`.

| Paramètres | Ce qu'ils font |
|---|---|
| `--renderer NAME` | Ouvre la fenêtre et essaie d'abord le moteur de rendu NAME, pour ce démarrage seulement : `auto`, `opengl` ou `directx` |
| `--mcp stdio` | Sert MCP sur l'entrée et la sortie standard, sans la fenêtre ; un client MCP le démarre ainsi |
| `--mcp http --token TOKEN` | Sert MCP sur `http://127.0.0.1:8765/mcp` sans la fenêtre ; le jeton a au moins 32 caractères |
| `--port N` | Avec `--mcp http` : un autre port |
| `--mode NAME` | Avec `--mcp` : `read` (par défaut) lit seulement, `edit` modifie les profils en mémoire, `files` les enregistre aussi et écrit le fichier de réponses |
| `--read-pc` | Avec `--mcp` : permet de lire les paramètres de ce PC |
| `--profile NAME` | Avec `--mcp` : un préréglage ou un profil de `profiles/` devient le profil courant |
| `--version` | Affiche la version |
| `--check-catalog` | Charge le catalogue intégré avec ses traductions et affiche leurs problèmes |
| `--help` | Affiche la liste des paramètres |
| `--smoke-window` | Ouvre une petite fenêtre, dessine deux images et affiche avec quoi elles ont été dessinées (aussi avec `--renderer`) : une vérification de l'affichage |
| `--elevated-apply PLAN DIGEST` | La fenêtre se démarre ainsi elle-même pour appliquer un plan confirmé en administrateur ; pas pour un démarrage à la main |

Le mode et l'autorisation de lire ce PC sont donnés à chaque démarrage et jamais enregistrés. Les messages
vont dans la console d'où le programme a été démarré et dans `logs/`.

## Clavier

| Touches | Action |
|---|---|
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | Nouveau, Ouvrir, Enregistrer, Enregistrer sous |
| F7, F9 | Vérifier le profil, générer le fichier de réponses |
| Ctrl+F, Esc | Rechercher, effacer et fermer la recherche |
| Alt+Left, Alt+Right | Précédent, Suivant |
| Ctrl+B | Ajouter un signet sur la position |
| Ctrl+D | Réduire ou afficher la description |
| Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab | Fermer l'onglet, onglet suivant, onglet précédent |
| Esc | Annuler la confirmation d'une modification de ce PC |
| Ctrl+, | Paramètres |
| F1 | Ce guide |

## Fichiers à côté de l'exécutable

`settings.json` (langue, thème, fichiers récents, onglets de la dernière session), `profiles/` (vos
profils), `output/` (le fichier de réponses et les scripts), `drafts/` (modifications non enregistrées,
restaurées au démarrage suivant), `logs/` (ce qu'a fait le programme), `bookmarks.wwmarks`. Rien n'est
écrit ailleurs.

## Pas encore disponible

La lecture des fonctionnalités et capacités de Windows directement depuis le programme : en attendant, le
script d'audit les lit.
