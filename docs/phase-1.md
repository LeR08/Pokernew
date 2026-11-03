# Phase 1 — Import & base de données

Statut : **plan — en attente des fichiers Winamax** · Salle prioritaire : **Winamax** · Joueur : `Tantaculejen`

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
