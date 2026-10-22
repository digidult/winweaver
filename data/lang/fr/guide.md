# Guide de WinWeaver

WinWeaver prépare les installations sans assistance de Windows 10 et 11 et garde les PC installés
configurés. Un profil indique quelles règles du catalogue sont activées, avec quels paramètres, et ce que
contiennent les formulaires d'installation ; à partir de lui, le programme génère le fichier de réponses
`autounattend.xml` avec ses scripts, compare des profils, lit les paramètres d'un PC en marche et les lui
applique. Le catalogue est écrit pour les petits bureaux sans domaine : la sécurité et les mises à jour
d'abord.

Cette page s'ouvre au premier démarrage, puis depuis le menu ou avec F1. C'est un fichier Markdown ;
les pages de documentation s'ouvrent de la même façon, chacune dans son propre onglet.

## Comment il est organisé

- **Les profils** (`.wwprofile`) gardent vos choix ; les préréglages Office, Strict, Laptop et Home sont
  des points de départ.
- **Le catalogue** réunit les règles intégrées en trois parties (Installation, Stratégies standard,
  Autres) et les modèles de stratégie (ADMX) que vous importez, qui deviennent aussi des règles.
- **Les onglets** : chaque document s'ouvre dans son propre onglet : profils, comparaisons, catalogues,
  signets, le serveur MCP, des pages comme celle-ci.
- **Ce PC** lit le Windows en marche, y vérifie des règles, les applique avec une sauvegarde et crée des
  scripts pour d'autres PC.
- **Le dossier du programme** garde tout à côté de `winweaver.exe` (voir la fin de cette page) ; rien
  n'est installé.

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
7. Essayez le fichier de réponses dans une machine virtuelle avant tout PC réel : installez, puis lisez ce
   PC avec **Ce PC** et comparez-le au profil.
8. Gardez les PC installés en règle : **Ce PC** applique les règles choisies en administrateur avec une
   sauvegarde, ou enregistre des scripts autonomes qui appliquent, annulent, contrôlent et défont sur
   d'autres PC.

## Le nom de l'ordinateur

Dans le formulaire **Installation**, le nom de l'ordinateur est aléatoire (comme Windows le fait par
défaut), fixe ou un modèle que le fichier de réponses développe sur chaque PC pendant l'installation. Un
modèle contient des lettres latines, des chiffres et des traits d'union autour de ces espaces réservés :

| Espace réservé | Ce qu'il donne |
|---|---|
| `{serial}`, `{serial:N}` | Le numéro de série de l'ordinateur lu dans son firmware, ses lettres et chiffres en majuscules ; `:N` garde ses N derniers caractères, sans lui tous (jusqu'à 15) |
| `{mac}`, `{mac:N}` | L'adresse MAC de la première carte réseau physique, 12 chiffres hexadécimaux en majuscules ; `:N` garde ses N derniers chiffres, sans lui les 12 |
| `{random}`, `{random:N}` | Des majuscules et des chiffres au hasard sans les confusions I, O, 0 et 1 ; `:N` en donne N, sans lui 6 |

N va de 1 à 15. Un numéro de série ou une adresse illisible (absent, ou le fabricant a laissé une valeur
fictive comme `Default string`) devient des caractères aléatoires de même longueur ; un plus court reste
entier. Le nom avec la longueur complète de chaque espace réservé tient en 15 caractères : `PC-{serial}`
est trop long, `PC-{serial:12}` tient ; gardez une lettre, car Windows refuse un nom fait uniquement de
chiffres. `PC-{random}` donne `PC-` et six caractères aléatoires, comme `PC-K7Q2MX` ; avec le numéro de
série `5CG1234XYZ` et l'adresse `00155D3E5A01`, `WS-{serial:6}` donne `WS-234XYZ` et `{mac:4}-LAB` donne
`5A01-LAB`. Sous le champ, le formulaire montre ce que donne le modèle.

## Langues de saisie

Le formulaire **Langues** liste les langues de saisie dans leur ordre ; la première est celle par défaut.
Chaque langue montre son nom et une liste de dispositions de clavier : d'abord celle que Windows donne à
la langue par défaut, puis toutes les dispositions de Windows par leur nom. **Ajouter** trouve une langue
par une partie de son nom ou de sa balise parmi toutes les langues que Windows connaît et l'ajoute avec
la disposition choisie ; la même langue peut venir deux fois avec deux dispositions, comme Ukrainien
(étendu) et Ukrainien. Une langue que Windows ne numérote qu'à la première connexion, comme Russe
(Ukraine), reçoit la disposition que Windows lui donne alors, et jusqu'à cette connexion le fichier de
réponses nomme la langue à laquelle elle appartient. Les langues et dispositions sont celles du Windows
sur lequel WinWeaver s'exécute, avec leurs noms dans sa langue.

## Fichiers d'utilisateurs

Un fichier d'utilisateurs (`.wwusers`) liste les comptes locaux à créer sur un PC avec leurs mots de
passe, chiffrés sous un mot de passe principal dans le même fichier. **Menu, Nouveau, Fichier
d'utilisateurs** en commence un vide, **Fichier d'utilisateurs des comptes de ce PC** un avec tous les
comptes normaux de ce PC (groupe, état, expiration du mot de passe, dossier du profil ; sans mot de
passe). L'onglet montre les comptes dans une liste et le compte sélectionné dans un formulaire : nom, nom
complet, description, groupe, dossier du profil et les consignes pour un compte qui existe déjà sur un PC
(définir le mot de passe du fichier, l'activer ou le désactiver, exactement ce groupe) ; sans elles, un
compte existant reste tel quel. **Déverrouiller** demande le mot de passe principal ou le définit quand
le fichier n'en a pas ; **Mot de passe** définit celui du compte sélectionné ; **Verrouiller** les
oublie. Sans le mot de passe principal, les mots de passe ne peuvent plus être lus. **Vérifier** liste
les problèmes, **Lire ce PC** ajoute les comptes de ce PC absents du fichier. **Créer sur ce PC** et
**Appliquer les mots de passe** modifient ce PC après une question et demandent WinWeaver exécuté en
administrateur ; le résultat s'affiche sous le formulaire, une ligne par compte. **Script** enregistre
`Create-<nom>.ps1` à côté du fichier d'utilisateurs : il demande lui-même le mot de passe principal et
fait la même chose sur un autre PC depuis une invite de commandes ouverte en administrateur
(`-PasswordsOnly` ne définit que les mots de passe). Dans le formulaire **Comptes** d'un profil, **D'un
fichier d'utilisateurs** charge ses comptes, avec leurs mots de passe si vous donnez le mot de passe
principal.

Tant que le fichier est déverrouillé, le formulaire montre le mot de passe du compte sélectionné sous
forme de points, avec **Afficher** et **Copier** (l'historique du presse-papiers de Windows conserve ce
qui est copié). Dans le formulaire **Comptes** d'un profil, **Vers un fichier d'utilisateurs** crée un
fichier d'utilisateurs de ses comptes ; leurs mots de passe attendent un mot de passe principal avant le
premier enregistrement.

## La fenêtre

- **Onglets** : chaque document ouvert est un onglet : un profil, une comparaison, ce guide, les
  signets, le serveur MCP, les paramètres. Le bouton de menu à l'extrémité gauche de la rangée
  d'onglets contient Nouveau, Ouvrir, Enregistrer, Comparer, le guide, le serveur MCP, les signets,
  Paramètres et Quitter.
- **À propos et Licence** dans le menu : la version avec le commit de la compilation et la licence (MIT),
  intégrée au programme, qui reste donc avec `winweaver.exe` même déplacé seul.
- **Barre de commandes** : Nouveau, Ouvrir, Enregistrer, Vérifier, Générer, Ce PC, Comparer. Dans une
  fenêtre étroite, elle n'affiche que des icônes, dans la ligne de navigation.
- **Ligne de navigation** : Précédent et Suivant parcourent les éléments que vous avez sélectionnés ;
  le fil d'Ariane indique où vous êtes et permet d'atteindre n'importe quel niveau ; Signet marque la
  position ; Rechercher filtre l'arborescence au fil de la saisie et mémorise vos recherches.
- **Arborescence et volet de détails** : l'arborescence à gauche, l'élément sélectionné à droite. La
  description se réduit avec Ctrl+D. Les règles d'un groupe sélectionné affichent un commutateur en fin
  de ligne.
- **Liens** : chaque règle, groupe, catalogue, fichier de profil et clé de registre qu'une vue nomme y
  mène : les règles dont une règle a besoin, celles qui ont besoin d'elle et celles avec lesquelles elle
  est en conflit ; les règles nommées dans les messages ; les groupes au-dessus et à l'intérieur d'un
  groupe ; la même stratégie dans d'autres catalogues ; les règles qui écrivent une clé de registre, dans
  l'onglet du registre et dans un fichier de registre. Un lien dans un profil ouvre son arbre jusqu'à
  l'élément ; depuis un autre onglet, il montre la règle dans le premier onglet de profil.
- **Barre d'état** : l'état du document, la version du catalogue, le serveur MCP et le dernier message.
- **Ouvrir le dossier** : après une génération ou l'enregistrement d'un script, d'un fichier de registre
  ou d'un profil, un bouton au bout de la barre d'état montre le fichier dans son dossier dans
  l'Explorateur ; Paramètres, Fichiers ouvre les dossiers du programme.

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
- **Enregistrer un fichier de registre** ou **un fichier de registre de retour aux valeurs par défaut**
  écrit les valeurs du registre de la sélection dans un fichier `.reg` pour regedit ou `reg import` ; ce
  qu'un fichier de registre ne peut pas contenir (étapes de scripts, applications, fonctionnalités) y
  reste en commentaires. Les valeurs du profil utilisateur par défaut vont dans
  `HKEY_USERS\UnattendDefault`, et le commentaire en tête du fichier explique comment y charger sa ruche.

Les règles qui n'agissent que pendant l'installation ou à la première connexion d'un utilisateur ne
sont pas lues.

Les fonctionnalités et capacités de Windows sont lues via DISM lorsque WinWeaver s'exécute en tant
qu'administrateur, ce qui ajoute quelques secondes à la lecture ; sans ces droits, elles restent
illisibles.
La lecture se fait en arrière-plan : la fenêtre continue de fonctionner, et un indicateur dans la barre d'état
la signale.

Appliquée en tant qu'administrateur, une règle qui supprime des applications ou des capacités de Windows les
supprime aussi depuis la fenêtre : les capacités via DISM, les applications pour tous les utilisateurs et
pour les comptes créés plus tard. La sauvegarde les nomme, mais l'annulation ne les rétablit pas.

Les étapes de script des règles s'appliquent aussi en natif, sans PowerShell : .NET Framework 3.5 et les
pilotes du support d'installation, les types de démarrage des services, NetBIOS, les lecteurs amovibles
dans le volet de navigation, « le mot de passe n'expire jamais » pour les comptes du profil, la
désactivation des comptes intégrés Administrateur et Invité (jamais le compte utilisé) et la suppression
des copies du fichier de réponses. Annuler ne reprend pas les programmes exécutés, les comptes modifiés
ni les fichiers supprimés ; la confirmation indique combien d'étapes de ce type compte un plan.

## Comparer des profils

**Menu, Comparer avec un fichier** ou **Comparer avec un préréglage** ouvre un onglet de comparaison :
chaque différence entre le profil actif et l'autre, sur trois colonnes. « Appliquer le côté droit »
définit ces valeurs dans le profil à partir duquel la comparaison a été faite ; « Enregistrer en tant
que fichier de différences » conserve les différences dans un fichier `.wwdiff` qui peut ensuite
s'appliquer à n'importe quel profil.

**Comparer, Modifications non enregistrées** ouvre un onglet des différences du profil avec son fichier,
ou avec le préréglage dont il vient : à gauche le profil tel qu'il est maintenant, à droite tel qu'il a
été enregistré.

## Ouvrir d'autres fichiers

Ouvrir accepte les profils (`.wwprofile`), les fichiers de différences (`.wwdiff`), les fichiers de
signets (`.wwmarks`), les pages Markdown, les fichiers de réponses générés par WinWeaver ou WinKickOff
(le profil qu'ils contiennent), les profils WinKickOff et les rapports d'audit (`.json`).

Un fichier de registre (`.reg`) s'ouvre dans un onglet à lui, comme le décrit la section sur le registre.

## Modèles de stratégie

**Paramètres, Modèles de stratégie** importe les modèles ADMX de ce Windows, d'un dossier ou d'un fichier
de catalogue ; leurs stratégies deviennent des règles sous Stratégies standard, et les importations sont
gardées dans `imports/`. Seules les importations cochées sont lues et chargées ; les autres n'occupent pas
de mémoire. Un profil peut contenir des options d'une importation non chargée : elles restent inchangées
dans le profil, sont listées sous **Options inconnues** à la fin de l'arborescence et sont exclues de la
génération, avec un avertissement à l'ouverture du profil et à la génération du fichier de réponses.
**Charger les catalogues** dans ce groupe trouve les importations qui les contiennent et ne charge que
celles-ci. Un paramètre (Général, Fenêtre) masque le groupe.

Une archive zip de modèles, comme Chrome, Edge et Firefox les distribuent, ou d'un fichier de catalogue
s'importe avec **Importer une archive...** et se lit sans être décompressée.

Une stratégie que contiennent deux importations apparaît dans les deux arborescences. **Comparer** dans la
barre d'un onglet de catalogue le met en regard d'une autre importation chargée ou d'un fichier de
catalogue ouvert : les stratégies présentes dans un seul des deux et celles dont le titre, la description,
les valeurs ou les paramètres diffèrent. Supprimer une importation, comme toute suppression, demande
confirmation.

Le panneau de détails d'une option renvoie aux onglets des catalogues qui la contiennent et à la même
stratégie dans les arborescences d'autres importations ; un signet de l'option aussi. Paramètres, Général
choisit quelles parties de la description sont ouvertes d'abord.

**Modifier** dans la barre d'un onglet de catalogue ouvre le catalogue dans un éditeur, et **Nouveau
catalogue** dans le menu des modèles en commence un vide à vous. Une stratégie reçoit son titre, sa
description, l'ordinateur ou le profil utilisateur par défaut et les valeurs du registre qu'elle écrit ou
supprime ; **Appliquer** contrôle tout le fichier, et rien ne change si le contrôle échoue. Enregistrer
(Ctrl+S) écrit le fichier et le recharge.

## Le registre

**Menu, Registre** ouvre le registre de ce PC dans un onglet : les clés à gauche, les valeurs de la clé
choisie à droite avec leurs types (REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ, REG_DWORD, REG_QWORD,
REG_BINARY), le chemin dans la ligne du dessus. Un double clic modifie une valeur ; Nouvelle clé,
Nouvelle valeur, Modifier et Supprimer sont dans la barre, et une suppression demande d'abord
confirmation. Les clés de la machine (HKEY_LOCAL_MACHINE) ne changent que si WinWeaver tourne en
administrateur. Le bouton à côté d'une clé dans **Ce qu'elle écrit** l'ouvre ici. **Signet** garde la clé
et la valeur avec un commentaire, et le signet liste les règles qui l'écrivent. **Rechercher** dans la
barre cherche sous la clé choisie comme la recherche de regedit : dans les noms des clés, les noms des
valeurs et les données, la chaîne entière uniquement si on le demande. La recherche se fait en
arrière-plan, **Arrêter** l'interrompt, et un clic sur un endroit trouvé l'affiche.

**Autorisations** dans la barre montre le propriétaire et la liste d'accès de la clé sélectionnée : les
entrées héritées sont affichées, les entrées propres de la clé changent (autoriser ou refuser, contrôle
total ou lecture, ce à quoi elles s'appliquent), un compte ou un groupe de ce PC reçoit une nouvelle
entrée, et l'héritage de la clé parente peut être désactivé. Appliquer demande d'abord ; une clé de
l'ordinateur ne change qu'avec WinWeaver lancé en administrateur.

Un fichier de registre (`.reg`) s'ouvre dans un onglet à lui, comme une structure et non comme du texte :
à gauche les clés et les commentaires du fichier dans leur ordre, à droite la clé sélectionnée avec ses
valeurs dans les types de regedit et les commentaires entre elles, un lien vers l'onglet du registre et
les règles des catalogues chargés qui écrivent la clé ou la valeur sélectionnée. **Nouvelle clé**,
**Nouvelle valeur**, **Modifier** et **Supprimer** sont dans la barre, et une suppression demande
d'abord. **Enregistrer** réécrit chaque ligne : les commentaires et les lignes que le programme ne lit
pas restent à leur place. Un fichier enregistré depuis **Ce PC** nomme ses règles dans des commentaires,
et la flèche d'un tel commentaire montre la règle dans le profil.

## Le serveur MCP

**Menu, Serveur MCP** ouvre l'onglet du serveur par lequel les agents d'IA lisent les profils et, si le mode le permet, les modifient. L'onglet édite un fichier de configuration, `mcp/default.wwmcp` à côté du programme, et l'enregistre à chaque modification : le port et le jeton, le mode (read lit seulement, edit modifie les profils en mémoire, files les enregistre aussi et écrit le fichier de réponses), les autorisations de lire ce PC, de lire ou modifier le registre et de modifier les signets et les catalogues, le profil avec lequel le serveur démarre et le journal détaillé. **Ouvrir** prend un autre fichier `.wwmcp` et **Enregistrer sous** en fait une copie ; `winweaver --mcp FILE` sert sans la fenêtre comme le dit le fichier. Le journal du serveur en marche liste chaque appel ; **Afficher les détails** ouvre à sa droite un panneau avec les arguments et la réponse de l'appel choisi et des boutons vers les règles, profils, catalogues et clés du registre qu'il cite. Le journal du serveur est `logs/mcp-<date>.jsonl`, un objet JSON par ligne ; le journal détaillé, désactivé par défaut, garde chaque requête et réponse en entier, sans mots de passe, clés ni jetons.

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
| `--registry NAME` | Avec `--mcp` : `off` (par défaut), `read` laisse les outils lire le registre, `write` aussi le modifier |
| `--edit-library` | Avec `--mcp` : les outils peuvent modifier les signets et les fichiers de catalogue ; les lire est toujours permis |
| `--profile NAME` | Avec `--mcp` : un préréglage ou un profil de `profiles/` devient le profil courant |
| `--mcp FILE` | Sert MCP sans la fenêtre comme le dit un fichier de configuration : un chemin vers un fichier `.wwmcp` ou un nom du dossier `mcp` à côté du programme |
| `--config FILE` | Avec `--mcp stdio` ou `--mcp http` : les paramètres d'un fichier de configuration, le jeton aussi ; les autres options les changent pour ce démarrage |
| `--detailed-log` | Avec `--mcp` : le journal garde chaque requête et réponse en entier, sans mots de passe, clés ni jetons |
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

`winweaver.settings.json` (langue, thème, fichiers récents, onglets de la dernière session), `profiles/` (vos
profils), `output/` (le fichier de réponses et les scripts), `drafts/` (modifications non enregistrées,
restaurées au démarrage suivant), `mcp/` (les configurations du serveur MCP), `logs/` (ce qu'a fait le programme), `bookmarks.wwmarks`. Rien n'est
écrit ailleurs.

Les paramètres, les signets, les brouillons et les fichiers de différences gardent le chemin d'un fichier
de ce dossier relatif à celui-ci : le dossier entier peut être déplacé ou copié sur un autre PC ; un
fichier situé ailleurs garde son chemin complet.

## Pas encore disponible

Rien du plan pour l'instant : ce dont le programme a besoin ensuite viendra de son usage.
