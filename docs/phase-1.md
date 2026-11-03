# Phase 1 — Import & base de données

Statut : **implémentée pour Winamax MTT — en attente de ta validation** (cash et Expresso : en attente de fichiers) · Salle prioritaire : **Winamax** · Joueur : `Tantaculejen`

## Critères d'acceptation (spec)

- Les fixtures Winamax et PokerStars (cash + tournoi) se parsent avec 0 erreur.
- 100 000 mains importées à ≥ 3 000 mains/s.
  Sur l'i7-8650U de l'utilisateur, la vitesse sera mesurée et rapportée ; cible réaliste ≈ 1 500–2 500 mains/s.
- Une ré-importation ajoute 0 doublon.
- Le watcher reprend après un redémarrage, sans re-parser les mains déjà vues.

⚠️ PokerStars fait partie du critère. Si tu n'y joues pas, je propose de le remplacer par
« Winamax cash + MTT + Expresso » et de garder PokerStars pour la phase 9.

## Informations déjà connues

- Dossier d'historiques réel : `%APPDATA%\winamax\documents\accounts\<pseudo>\history`
  (et non `Documents\Winamax Poker\…` comme le supposait la spec). L'assistant cherchera aux deux endroits.
- Pseudo héros Winamax : `Tantaculejen` (proposé automatiquement : c'est le nom du dossier du compte).

## Tâches

### 1. `core-model`
- `Card`, `Rank`, `Suit` ; `HoleCards` = liste de longueur variable (prête pour le PLO, cf. spec).
- `Hand` normalisé, comme décrit dans la spec : salle, id, table, type de jeu, limites, devise, blindes,
  ante / BB ante / straddle, niveau, tournoi et buy-in, sièges, bouton, positions calculées (UTG…BB),
  cartes, board par street, actions (avec pot avant/après et all-in), rake, jackpot, gains, showdown, horodatage.
- Montants en entiers : centimes (cash) ou jetons (tournoi). Les unités en big blinds sont dérivées.
- Calcul des positions depuis le bouton et le nombre de joueurs, du heads-up au 10-max.

### 2. `hh-parsers`
- Trait `HandHistoryParser` (`room_id`, `sniff`, `split`, `parse`) et registre des parseurs.
- **Parseur Winamax** écrit **uniquement à partir de tes fichiers** : cash, MTT, Expresso, Go Fast
  si présent, plus les résumés de tournoi s'ils existent.
- Détection automatique du format, gestion de l'encodage (UTF-8 avec ou sans BOM) et des fins de ligne Windows.
- `fixtures/winamax/` avec tes fichiers anonymisés si tu le souhaites, et des tests de snapshot `insta`
  de chaque `Hand` parsée.
- Vérifications de cohérence par main : la somme des mises plus le rake est égale aux gains,
  les stacks ne deviennent jamais négatifs, il y a un seul bouton.

### 3. `db` (SQLite WAL, `rusqlite`)
- Migrations versionnées.
- Tables de la spec : `players`, `hands`, `hand_players`, `hand_actions`, `hand_player_stats`,
  `tournaments`, `sessions`, `tags`, `hand_tags`, `notes`.
- Tables techniques en plus : `import_files` (chemin, taille, mtime, offset déjà lu) et `import_errors`.
- Index de la spec ; contrainte d'unicité `(room, hand_no)` pour le dédoublonnage.
- `config/defaults/counters.toml` : un premier lot de compteurs, ceux qu'utiliseront les stats de base
  de la phase 2 (VPIP, PFR, 3-bet, WTSD, W$SD…), calculés à l'import.

### 4. Pipeline d'import
- Watcher (`notify`) sur les dossiers configurés, plus un import manuel de dossier, de fichiers ou d'un zip.
- Lecture incrémentale : on reprend à l'offset enregistré, donc seules les nouvelles mains sont parsées.
- Parsing en parallèle (`rayon`, `performance.worker_threads`), écriture par lots de 1 000 mains dans une transaction.
- Événement `hands_imported` sur le bus, avec le compte et les ids de mains.
- Journal d'erreurs : la main brute, l'erreur et un bouton « copier comme cas de test ».

### 5. Configuration
- Nouvelle famille `rooms/*.toml` avec son schéma : dossiers candidats, pseudo héros, et
  `hud_allowed` (utilisé en phase 3).
- `rooms/winamax.toml` par défaut.

### 6. Reporté de la phase 0
- **Assistant de premier lancement** :
  1. langue ;
  2. détection de Winamax, avec dossiers et pseudo proposés ;
  3. profil par défaut ;
  4. import initial.
- **Sauvegarde SQLite planifiée** : quotidienne, on garde N copies, restauration en un clic.
- Mesure et optimisation du démarrage si les lancements suivants restent au-dessus de 2 s.
  Par exemple : thème, profil et réglages livrés en une seule requête au premier rendu.

### 7. UI du module Import
- Dossiers surveillés et état du watcher.
- Mains importées : total et par session d'import, avec la vitesse.
- Liste des erreurs et bouton d'import manuel.

## Tests
- Snapshots `insta` de chaque fixture, avec 0 erreur et les vérifications de cohérence vertes.
- Benchmark sur 100 000 mains générées à partir des fixtures (ids renumérotés), mains/s rapportées.
- Ré-import complet : 0 nouvelle ligne.
- Reprise du watcher : on ajoute une main à un fichier existant, on redémarre, seule cette main est lue.
- Positions calculées pour des tables de 2 à 10 joueurs.
- UI : assistant de premier lancement et écran d'import (Playwright, backend simulé).

## Résultats (3 nov. 2026)

Fixtures reçues : 3 historiques de MTT KO Winamax (288 mains, 6-max et 3-max) et 2 résumés de tournoi.
Les 40 adversaires sont anonymisés (`Player01…`), ton pseudo est conservé.

| Critère | Résultat |
|---|---|
| Fixtures Winamax MTT, 0 erreur | ✅ 288/288 mains, plus les 2 résumés. Chaque main passe les contrôles de cohérence : argent conservé, pot total = somme des mises, aucune carte en double, positions attribuées |
| Fixtures Winamax cash / Expresso | ⏳ aucun fichier reçu ; ces en-têtes sont refusés avec un message clair, jamais devinés |
| PokerStars | ➖ hors périmètre : l'utilisateur ne joue pas sur PokerStars (décision du 3 nov. 2026). Le critère de la phase 1 porte sur Winamax seul |
| 100 000 mains à ≥ 3 000 mains/s | ✅ **≈ 4 000 mains/s** (100 224 mains en 24,8 s, conteneur à 4 vCPU, build release). Commande : `cargo test -p lps-db --release -- --ignored --nocapture bench` |
| Ré-import : 0 doublon | ✅ fichier inchangé = 0 octet relu ; mêmes mains sous un autre nom = 288 doublons ignorés, 0 inséré |
| Reprise du watcher après redémarrage | ✅ test : après redémarrage, seule la main ajoutée au fichier est lue (offset stocké par fichier) |
| Nouvelle main → base de données | ✅ **264 ms** mesurés dans la vraie app (cible < 1 s) |
| Assistant de premier lancement | ✅ déroulé dans la vraie app Tauri (Linux) : compte `Tantaculejen` détecté, dossier et pseudo écrits dans `rooms/winamax.toml`, profil MTT, 288 mains importées |
| Sauvegarde SQLite planifiée + restauration | ✅ tests + vue dans l'app (première sauvegarde 60 s après le lancement, puis toutes les 24 h ; 7 gardées) |
| `cargo test`, `clippy -D warnings`, `pnpm test`, `pnpm e2e` | ✅ 68 tests Rust, 20 Vitest, 9 Playwright |

Particularités Winamax découvertes dans tes fichiers, et gérées :
- **Petite blinde morte** quand un joueur vient de sauter : les positions sont déduites des blindes réellement postées.
- **Pas de ligne « Uncalled bet »** : la mise non suivie revient dans `collected`, parfois sous forme de *side pot*.
- **Joueur assis mais pas servi** (il vient d'arriver à la table) : il est exclu de la main.

Stats héros sur ces 288 mains, calculées par la vraie app : VPIP 23,6 %, PFR 17,4 %, 3-bet 5,4 %, WTSD 40,8 %.

Non fait dans cette phase :
- EV all-in ajustée : la colonne `allin_ev` existe, elle sera remplie en phase 4 avec l'évaluateur.
- Sessions auto-groupées : prévu avec le module Bankroll en phase 2.
- Démarrage : 0,79 s au deuxième lancement sur ton PC ✅.

## Décision du 3 nov. 2026 — salles

- **Pas de PokerStars.**
- Demande de l'utilisateur, pour plus tard : **détecter automatiquement tous les clients poker installés**
  et leurs dossiers d'historiques, avec une recherche automatique sur le disque au moment de l'import.
  Le mécanisme existe déjà (`crates/hh-parsers/src/install.rs`, utilisé par l'assistant). Il s'étendra
  salle par salle quand chaque parseur arrivera avec ses fichiers d'exemple, et il faudra lui ajouter
  un bouton « Rechercher des historiques » dans l'écran Import. Planifié au plus tard en phase 9.

## Premier test sur le PC de l'utilisateur (3 nov. 2026)

- Comptes détectés automatiquement : `Tantaculejen` et `GTOverBet`.
- Import : **7 108 mains, 97 tournois, 1 502 joueurs**, du 24/12/2024 au 02/11/2026.
- 285 erreurs, puis 1 884 mains relues (en doublon) à chaque lancement. Corrigé :
  - **Résumés avec recave** : un fichier contient un résumé par entrée, la première suffixée
    ` - Late Registration`, et la ligne `You won Bounty 1.07€` n'a pas de gain hors bounty.
    Un tournoi = une ligne avec `entries` (nombre de buy-ins) ; les gains et bounties sont additionnés.
  - **Fichiers pas en UTF-8** : les octets invalides sont décodés en Windows-1252, et les offsets
    stockés redeviennent des positions dans le fichier. Avant, l'offset était une position dans le
    texte décodé, plus long que le fichier, donc le fichier était relu depuis le début à chaque scan.
  - La migration v2 remet en file les fichiers qui avaient des erreurs : ils sont réimportés
    automatiquement avec le nouveau parseur.
  - Nouveau bouton « Exporter toutes les erreurs » : un fichier texte dans `logs/`, à envoyer pour
    corriger les cas restants.
