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
- **Le catalogue** réunit les règles intégrées en trois parties et les modèles de stratégie (ADMX) que
  vous importez, qui deviennent aussi des règles. **Installation** regroupe ce que font le fichier de
  réponses et ses scripts ; **Stratégies**, chaque règle qui n'écrit que dans le Registre et chaque
  importation ; **Autres**, le reste : services, fonctionnalités, applications, programmes et scripts sur
  un Windows en cours d'exécution.
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

La règle **Région de l'utilisateur** (`default-user.region`) du profil utilisateur par défaut contient le GeoID et le code à deux lettres du pays des nouveaux utilisateurs. Ses deux champs se saisissent librement, et la liste à côté de chacun contient toutes les régions que Windows connaît, filtrées par la saisie : un choix définit le GeoID et le code ensemble, et le nom de la région apparaît après les champs. La vérification avertit quand les deux valeurs ne désignent pas une même région de la liste.

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
principal. Un changement du groupe dans le formulaire coche exactement ce groupe, et le compte passe entre Administrateurs et Utilisateurs ; le compte sous lequel WinWeaver s'exécute reste dans Administrateurs.

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

- **Menus et onglets** : chaque élément d'un menu a une icône. Quand les onglets sont plus larges que la
  fenêtre, la rangée défile avec la molette, l'onglet actif vient en vue et un bouton à droite liste tous
  les onglets.
- **Actives seules** dans la barre d'un profil n'affiche que les règles actives, leurs groupes ouverts ;
  les outils MCP `list_groups`, `list_rules` et `search_rules` prennent `only_on` de la même façon.
- **Paramètres** : les champs d'une règle prennent la largeur du panneau ; un texte limité par le fichier
  de réponses (les contacts du support) compte ses caractères.
- **Travail en arrière-plan** : une lecture de ce PC, une importation de modèles, une modification de ce
  PC et une recherche dans le registre s'affichent dans un panneau en bas à droite avec le temps écoulé
  et Arrêter là où le travail peut s'arrêter ; le panneau se replie en un indicateur.

## Ce PC

**Ce PC** dans la barre de commandes lit les paramètres de cet ordinateur par l'intermédiaire de
Windows, sans rien modifier :

**Le profil.** Le formulaire Profil, premier de l'arbre, modifie le nom, l'auteur et le commentaire du profil ; la première partie de la ligne de navigation montre le profil avec ses formulaires en bref. Un modèle s'ouvre avec la langue de la fenêtre et la région de votre Windows, et les langues de saisie de ce Windows (English International quand elles ne se lisent pas). **Menu, Ce PC** fonctionne depuis tout onglet : la lecture dans un nouveau profil n'a pas besoin d'un profil ouvert.

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
signets (`.wwmarks`), les pages Markdown, les fichiers de réponses avec un profil intégré (le profil
qu'ils contiennent), les anciens profils en JSON et les rapports d'audit (`.json`).

Un fichier de registre (`.reg`) s'ouvre dans un onglet à lui, comme le décrit la section sur le registre.

## Modèles de stratégie

**Paramètres, Modèles de stratégie** importe les modèles ADMX de ce Windows, d'un dossier ou d'un fichier
de catalogue ; leurs stratégies deviennent des règles sous Stratégies, et les importations sont
gardées dans `imports/`. Seules les importations cochées sont lues et chargées ; les autres n'occupent pas
de mémoire. Un profil peut contenir des options d'une importation non chargée : elles restent inchangées
dans le profil, sont listées sous **Options inconnues** à la fin de l'arborescence et sont exclues de la
génération, avec un avertissement à l'ouverture du profil et à la génération du fichier de réponses.
**Charger les catalogues** dans ce groupe trouve les importations qui les contiennent et ne charge que
celles-ci. Un paramètre (Général, Fenêtre) masque le groupe.

Le menu **Catalogues** de la barre d'un profil liste tous les fichiers de catalogue de `imports/`, chargés ou non : un fichier chargé porte une coche et la couleur d'accent, et un clic ouvre son onglet ; un autre se charge d'un clic et ouvre aussi son onglet. La liste est relue à chaque ouverture du menu. Par défaut, un clic sur un catalogue du menu le charge ou le décharge ; dans Paramètres, Général, l'autre choix ne laisse dans le menu que les catalogues chargés, et un clic ouvre l'onglet de l'un d'eux.

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
(Ctrl+S) écrit le fichier et le recharge. **Supprimer le groupe** retire le groupe sélectionné, après une question, avec tous les groupes et stratégies qu'il contient ; le groupe supérieur du fichier reste.

La comparaison s'ouvre dans un onglet : les stratégies qui diffèrent dans l'arbre de leurs catégories,
avec des cases pour celles du premier catalogue seul, du second seul et les différentes, et la stratégie
sélectionnée telle qu'elle est dans les deux catalogues, côte à côte. **Enregistrer le rapport** écrit le
rapport en fichier Markdown, **En page** l'affiche en page. **Copier dans**, dans la barre d'un onglet de
catalogue, copie la stratégie sélectionnée ou le groupe sélectionné avec les groupes qu'il contient dans un
catalogue ouvert dans son éditeur (vous choisissez lequel quand il y en a plusieurs) ou dans un nouveau
catalogue, avec les mêmes identifiants et les textes des autres langues : un catalogue des seules
stratégies nécessaires, à transmettre en fichier.

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
et la flèche d'un tel commentaire montre la règle dans le profil. Une clé que le fichier supprime puis
écrit de nouveau (une liste qui remplace les valeurs de sa clé est enregistrée ainsi) apparaît une fois
dans l'arbre, marquée ↻ comme recréée, avec les valeurs qu'elle reçoit.

Un fichier de stratégie de groupe (`Registry.pol`) s'ouvre aussi dans son propre onglet : les fichiers de
l'ordinateur et des utilisateurs de ce Windows (**Menu, Stratégie de groupe** ; la barre d'un profil, d'un
catalogue et du registre) ou tout fichier `.pol`. Les clés forment un arbre à gauche, les entrées de la clé
sélectionnée sont à droite avec ce que fait chacune. **Nouvelle clé**, **Nouvelle valeur**, **Modifier**
(ou un double clic) et **Retirer** changent les entrées dans une fenêtre au-dessus de l'onglet, comme les
valeurs d'un fichier du registre ; **Racine** choisit si un fichier hors de la stratégie de groupe
s'applique à l'ordinateur ou à l'utilisateur (les fichiers de ce Windows le tiennent de leur dossier) ;
**Ajouter un fichier** prend les entrées d'un autre fichier de stratégie de groupe ou les modifications
d'un fichier du registre de cette racine ; **En .reg** enregistre le fichier en fichier du registre.
Enregistrer réécrit le format de la stratégie de groupe ; les fichiers de ce Windows sont enregistrés en
copies ailleurs, car la stratégie de groupe elle-même les écrit. Les modifications non enregistrées
reviennent d'un brouillon après un redémarrage.

## Le serveur MCP

**Menu, Serveur MCP** ouvre l'onglet du serveur par lequel des agents d'IA lisent les profils et, quand le mode le permet, les modifient. L'onglet modifie un fichier de configuration, `mcp/default.wwmcp` à côté du programme, et l'enregistre à chaque changement : le port et le jeton, le mode (read lit seulement, edit modifie les profils en mémoire, files les enregistre aussi et écrit le fichier de réponses) et le journal détaillé. Sous **Outils**, chaque outil peut être désactivé seul ou avec son groupe, et la carte de WinWeaver porte ses options : la lecture de ce PC, la lecture ou la modification du registre, la modification des signets et des catalogues et le profil avec lequel le serveur démarre. **Appeler un outil** exécute n'importe quel outil par son nom avec ses arguments, comme le ferait un agent, et montre la réponse. **Ouvrir** prend un autre fichier `.wwmcp` et **Enregistrer sous** en fait une copie (un fichier d'une version antérieure de WinWeaver est écrit dans le nouveau format à l'ouverture) ; `winweaver --mcp FILE` sert sans fenêtre comme le dit le fichier. Le journal liste chaque appel ; **Afficher les détails** ouvre à sa droite un panneau avec les arguments et la réponse de l'appel choisi et des boutons vers les règles, profils, catalogues et clés de registre qu'il nomme. Le journal du serveur est `logs/mcp-<date>.jsonl`, un objet JSON par ligne, et **Ouvrir le journal** l'affiche ; le journal détaillé, désactivé par défaut, garde chaque requête et réponse entières, sans mots de passe, clés ni jetons.

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
| `--build PROFILE` | Vérifie un profil et génère son fichier de réponses sans la fenêtre : `autounattend.xml` dans `output`, avec à côté les scripts qu'il intègre ; PROFILE est un fichier `.wwprofile`, un préréglage (`Office`, `Strict`, `Laptop`, `Home`) ou le nom d'un profil de `profiles/` ; une erreur de la vérification arrête la génération |
| `--import-admx SOURCE` | Importe des modèles de stratégie comme le fait Paramètres, Modèles de stratégie : `system` (les modèles de ce Windows), un dossier de fichiers ADMX, un fichier `.admx` avec les ADML des dossiers de ses langues, une archive `.zip` ou un fichier `.wwcatalog` ; le fichier de catalogue va dans `imports/`, et la fenêtre et le serveur MCP le chargent à leur prochain démarrage (à lancer fenêtre fermée) |
| `--export-catalog SOURCE` | Écrit un fichier de catalogue (`.wwcatalog`) comme le fait Exporter dans Paramètres, Modèles de stratégie : d'une importation par son identifiant (le nom de son fichier dans `imports/`), ou d'une source de `--import-admx` lue sans l'importer ; `<id>.wwcatalog` dans `output` |
| `--read-pc` | Sans `--mcp` : lit ce PC dans un nouveau profil au nom de l'ordinateur, comme Ce PC, Lire ce PC, et l'enregistre dans `profiles/` ; avec un fichier `.xml` dans `--out`, écrit à la place le fichier de réponses de ce profil ; les fonctionnalités et capacités de Windows ne sont lues que si le programme s'exécute en tant qu'administrateur |
| `--base PROFILE` | Avec `--read-pc` : le profil dont part la lecture, donné comme pour `--build` ; le préréglage Office par défaut |
| `--diff A B` | Écrit les différences du profil A au profil B dans un fichier de différences, `A-vs-B.wwdiff` dans `profiles/` ; A et B sont donnés comme pour `--build` |
| `--script KIND FILE` | Écrit sans la fenêtre le script autonome d'un profil (`.wwprofile`) ou d'une différence (`.wwdiff`) : `apply`, `revert` (retour aux valeurs par défaut de Windows) ou `audit` ; un script d'application ou de retour reçoit `Undo-Apply.ps1` à côté de lui |
| `--reg KIND FILE` | Écrit le fichier de registre de l'application (`apply`) ou du retour aux valeurs par défaut (`revert`) d'un profil ou d'une différence |
| `--only IDS` | Avec `--script` ou `--reg` : seulement ces règles ou groupes, leurs identifiants séparés par des virgules |
| `--out FILE` | Avec `--build`, `--export-catalog`, `--read-pc`, `--diff`, `--script` ou `--reg` : le fichier à écrire, quand la valeur se termine par l'extension que la commande écrit (`.xml`, `.wwcatalog`, `.wwprofile`, `.wwdiff`, `.ps1`, `.reg`) ; toute autre valeur est un dossier, créé s'il manque, pour le fichier de son nom habituel ; `Undo-Apply.ps1` et les scripts du fichier de réponses vont à côté du fichier ; sans `--out`, les fichiers vont dans `output` à côté du programme (`profiles/` pour `--read-pc` et `--diff`) |
| `--help` | Affiche la liste des paramètres |
| `--smoke-window` | Ouvre une petite fenêtre, dessine deux images et affiche avec quoi elles ont été dessinées (aussi avec `--renderer`) : une vérification de l'affichage |
| `--elevated-apply PLAN DIGEST` | La fenêtre se démarre ainsi elle-même pour appliquer un plan confirmé en administrateur ; pas pour un démarrage à la main |

Une commande qui écrit des fichiers nomme chaque fichier écrit et se termine avec le code 0 ; une mauvaise
utilisation des paramètres se termine avec le code 2, un échec avec 1, et la raison est affichée.

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
