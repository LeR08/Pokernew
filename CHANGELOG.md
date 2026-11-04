# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Versions follow the roadmap phases.

## [1.0.0] — 2026-11-04 — Phase 9 : Finitions

### Ajouté / Added
- Version 1.0.0. README réécrit en français et en anglais, `THIRD_PARTY.md` (composants tiers et références de test).
- Le zip portable contient aussi le README, la licence, `THIRD_PARTY.md` et le CHANGELOG.

### Salles / Rooms
- Winamax seulement : aucun lecteur n'est écrit pour une salle sans vrais fichiers (règle de la spec). La marche à suivre pour en ajouter une est dans `docs/phase-9.md`.

## Phase 8 : Entraîneur et analyse

### Ajouté / Added
- `lps-trainer` :
  - exercices postflop tirés du cache du solveur : un nœud, une street ou une main complète, chrono, « mes points faibles » ;
  - correction par l'EV de chaque action pour ta main, perte en bb et en % du pot, note avec seuils réglables ;
  - analyse des mains jouées contre le solveur : ranges tirées de tes charts, ligne réelle suivie, EV perdue à chaque décision, fuites par type de spot ;
  - file en tâche de fond qui se met en pause dès qu'une table est ouverte.
- Module Entraîneur : Postflop, Préflop (depuis tes charts), Push/fold, Analyse de mains, Historique (courbe de la perte moyenne).
- Tracker : bouton « Analyser contre le solveur » sur la liste des mains.

## Phase 7 : Ranges et solveur préflop

### Ajouté / Added
- `lps-preflop` : arbre préflop simplifié (open, limp de la SB, 3-bet, 4-bet, tapis) résolu par CFR+ sur les 169 mains, avec les équités exactes et les cartes bloquées. Tête-à-tête exact, approximation à 6 joueurs au plus, réalisation d'équité IP/OOP réglable, exploitabilité exacte du modèle.
- Bibliothèque de ranges : export et import d'un dossier entier (`.lpsranges`), import des ranges PioSOLVER / GTO+, aller-retour à l'identique.
- **Charts par défaut** pour 9 formats (cash 6-max 100 bb, MTT de 10 à 100 bb, Spin 25 bb), 46 situations chacun, calculés par notre solveur préflop calibré (rake, réalisation d'équité). Ce sont des points de départ, qui ne remplacent jamais tes fichiers.
- Solveur préflop : rake (pas de flop, pas de rake) et jouabilité postflop.

### Corrigé / Fixed
- Solveur préflop : en tête-à-tête, la SB (bouton) est bien en position après le flop.

### Module
- Module Préflop :
  - Charts : dossiers par format (cash 6-max 100 bb, MTT de 10 à 100 bb, Spin), situations vides créées pour toi, édition, import et export ;
  - Solveur : réglages, arbre cliquable, grille 13×13 par action, enregistrement de toutes les situations dans la bibliothèque.

## Phase 6 : Solveur postflop

### Ajouté / Added
- Moteur `postflop-solver` (Discounted CFR, AGPL) intégré dans `crates/solver/engine` : c'est désormais notre fork (`FORK.md`).
- `lps-solver` :
  - spot (ranges dans la notation d'Equity Lab, board, pot, tapis, rake) et arbre de mises (tailles par street et par joueur, donks, seuils de tapis, **plafond de relances par street**) ;
  - estimation de la mémoire et refus au-delà du plafond réglé ;
  - calcul en tâche de fond avec pause, reprise et annulation ;
  - verrouillage de nœuds ;
  - cache compressé `solver-cache/` ;
  - export CSV et JSON ;
  - batch sur une sélection de flops.
- Arbres prédéfinis `solver_trees/*.toml` : « SRP 100bb · 2 tailles », « SRP 100bb · simple », « 3BP · 2 tailles ».
- Module Solveur :
  - nouveau calcul (éditeur de range, aperçu de l'arbre, courbe d'exploitabilité) ;
  - visionneuse (fil des actions, grille 13×13 par action, combos, EV, équité et EQR, décomposition de la range, verrouillage, export) ;
  - bibliothèque, comparaison côte à côte, batch avec rapport par texture.

## Phase 5 : ICM & push/fold

### Ajouté / Added
- `icm` : ICM Malmuth-Harville exact (jusqu'à 16 joueurs avec des jetons), chip EV, bubble factors, primes de risque, FGS sur 1 à 3 mains, bounties (KO).
- Solveur push/fold Nash de 2 à 10 joueurs (fictitious play), en chip EV ou en ICM, avec antes, ante BB et bounties. Mode exploitant (ranges fixées) et exploitabilité affichée.
- Table exacte d'équité préflop 169 × 169 (`crates/equity/data/preflop_matrix.txt`).
- Tête-à-tête identique aux tableaux Nash publiés (HoldemResources) sur 3 999 des 4 003 décisions ; les 4 autres sont des mains indifférentes (moins de 0,01 bb d'écart).
- Module ICM : push/fold (grilles 13×13 par décision, EV par main, curseurs « et si »), calculateur ICM, revue de mains (une main ou un tournoi entier, triés par EV perdue), quiz, structures de gains saisies par toi (`payouts/*.toml`, import CSV).
- Liens « Revue ICM » dans le replayer et « Revue push/fold » dans la liste des tournois.

## Phase 4 : Equity Lab

### Ajouté / Added
- `evaluator` : notre évaluateur de mains de 5 à 7 cartes (masques de bits et table de quintes). Il compte juste toutes les mains de 5 et de 7 cartes et donne le même gagnant qu'eval7 sur 20 000 paires.
- `equity` : ranges pondérées et notation standard (`TT+`, `A5s-A2s`, `KQo:50%`, `AhKh`, `random`, `15%`) ; équité de 2 à 10 joueurs, exhaustive ou Monte-Carlo multi-cœur avec arrêt à la précision demandée ; décomposition d'une range sur un board (mains faites et tirages, combos exacts) ; filtre pour la street suivante ; textures sur les 1 755 flops ; classement préflop pour « top X % ».
- 20 cas de référence calculés avec eval7 : écart maximal de 0,033 % (critère : ±0,1 %).
- Module Equity Lab : calculateur (résultats, courbe de distribution), analyse de range (tableau, sélection → street suivante, histogramme d'équité contre une range adverse), textures de flop.
- Éditeur de range partagé : grille 13×13 (clic, glisser, souris ou tactile), poids, couleurs par case, texte, top X %, compteur de combos qui tient compte du board et des cartes mortes, bibliothèque `ranges/**.txt`.
- Replayer : lien « Ouvrir dans Equity Lab », qui pré-remplit ta main, les mains montrées et le board.

### Fair-play
- Equity Lab est verrouillé en mode jeu (tuile et commandes).

### Corrigé / Fixed
- Replayer : le lien vers Equity Lab prend toujours le board complet de la main jouée.

## Phase 3 : HUD

### Ajouté / Added
- Suivi des fenêtres de table sous Windows (EnumWindows + SetWinEventHook) et rapprochement avec les tables des mains récentes, sans deviner le format des titres.
- Une fenêtre HUD transparente, toujours au-dessus et transparente aux clics par table ; stats historiques par siège, rafraîchies à chaque import ; popup détaillé avec les notes.
- Dispositions `hud/*.toml` (stats, positions par taille de table, opacité…), mode édition par glisser-déposer, raccourcis globaux (Ctrl+Alt+H, Ctrl+Alt+E).
- Mode jeu (fair-play) : modules d'étude verrouillés quand une table est ouverte, déverrouillés 60 s après la dernière.
- Module HUD : tables détectées automatiquement, **édition du HUD table par table** (« Modifier le HUD de cette table » → barre « Terminer » sur la table), masquer le HUD d'une table, afficher son propre panneau, inspecteur de fenêtres.

### Retiré / Removed
- Mode « table simulée » (remplacé par la détection automatique).

### Corrigé / Fixed
- Titres de fenêtre Winamax réels (`Winamax Monster Stack(1196250156)(#0000)`) reconnus : comparaison normalisée (casse, espaces, ponctuation).
- Joueurs assis sans être servis gardés dans la main (panneau HUD de chaque adversaire).
- Onglet Adversaires vide quand un compte héros était sélectionné.

## Phase 2 : Tracker

### Ajouté / Added
- `stats-engine` : langage de formules (`+ - * /`, comparaisons, `and/or/not`, `if()`) compilé en SQL avec paramètres liés ; stats en TOML (`stats/*.toml`, rechargées à chaud).
- 13 stats par défaut, chacune avec une explication en français et en anglais.
- Tracker : vue d'ensemble (résultat d'abord), rapports par position / format / mois / limite / compte, graphique (bb ; « Afficher en € » pour le cash et les tournois), tournois avec ROI et ITM, liste des mains, éditeur de stats avec aperçu en direct.
- Replayer : table, cartes, mises, pot, pas à pas, lecture automatique, flèches du clavier, montants en bb ou en jetons / €.
- Référence indépendante pour les stats (`tools/reference_stats.py`).
- Sessions (regroupées par trous de 30 min) et **bilan de session** : mains à revoir et bons coups, expliqués, avec un lien vers le replayer ; alerte de tilt.
- Détecteur de fuites (`leaks/targets.toml`, intervalle de Wilson 95 %, résultat dans le spot).
- Filtres avancés (cartes, type de pot, tapis, rue, texture du flop, abattage, tapis préflop, buy-in), population et onglet Adversaires, auto-notes (`notes/auto.toml`).
- Grille 13×13 des mains de départ, bankroll (dépôts/retraits, courbe, règle des 30 buy-ins), notes et couleurs par joueur, export CSV.

## Phase 1 : Import & base de données

### Ajouté / Added
- `core-model` : cartes, positions (dead small blind et dead button gérés), modèle de main normalisé avec contrôles de cohérence.
- Parseur **Winamax** (MTT, knockout, 6-max et 3-max) et résumés de tournoi, écrits à partir de vrais fichiers ; une ligne inconnue est une erreur, pas un saut.
- Base SQLite (WAL) : schéma de la spec, migrations, 25 compteurs calculés à l'import (`docs/counters.md`), recalcul automatique quand un compteur est ajouté.
- Import incrémental (offset par fichier), parsing parallèle, écriture par lots de 1 000 mains, dédoublonnage, import de dossiers, fichiers et zip.
- Surveillance des dossiers : une nouvelle main arrive en base en 0,3 s environ.
- Journal d'erreurs avec la main brute et un bouton « copier comme cas de test » ; bouton « réessayer ».
- Assistant de premier lancement : langue, détection des comptes Winamax, profil, import.
- Configuration `rooms/*.toml` (dossiers, pseudos, `hud_allowed`) et onglet « Salles ».
- Sauvegardes SQLite planifiées (24 h, 7 gardées) et restauration en un clic.

### Corrigé / Fixed
- Winamax : résumés de tournoi avec recave (plusieurs entrées, « Late Registration ») et gains en bounty seul.
- Fichiers pas en UTF-8 : décodés en Windows-1252, offsets en octets du fichier ; ils ne sont plus relus à chaque lancement.
- Les fichiers en erreur sont réimportés automatiquement après la mise à jour (migration v2).
- Nouveau : « Exporter toutes les erreurs » dans le module Import.
- Winamax : cash games (en centimes, rake), tournois sans ante, Expresso (format Spin), niveaux négatifs, cartes montrées sans abattage, tickets gagnés (satellites).
- Les fichiers en erreur sont réimportés automatiquement à chaque fois que les parseurs apprennent un format (`PARSER_VERSION`).
- Front : deux appels simultanés à `getBackend()` pouvaient créer deux backends, dont les événements ne se voyaient pas.

## Phase 0 : Shell

### Ajouté / Added
- Workspace Cargo avec toutes les crates de la spec (squelettes) + crate `config`.
- Dossier de données : `%APPDATA%\LocalPokerSuite`, `--data-dir`, `LPS_DATA_DIR`, mode portable (`portable.flag`).
- Fichiers par défaut commentés FR/EN (`settings`, `hotkeys`, `themes`, `profiles`) + JSON Schemas, copiés au premier lancement sans jamais écraser.
- Validation JSON Schema ; un fichier invalide garde la dernière version valide et l'erreur s'affiche.
- Rechargement à chaud (watcher, debounce 200 ms) relayé par un bus d'événements interne.
- Sauvegarde depuis l'UI qui conserve les commentaires TOML.
- Launcher à tuiles (glisser-déposer), barre latérale, ouverture d'un module dans sa fenêtre — le tout construit depuis les manifests de modules.
- Centre de réglages : formulaires générés depuis les schémas + éditeur TOML brut avec validation en direct.
- Thèmes clair/sombre (tokens de couleur), densité, taille du texte, profils, raccourcis réaffectables, i18n FR/EN.
- Réglages de performance calibrés pour un i7-8650U / 16 Go (6 threads, plafond solveur 8 Go).
- Journal de démarrage `<data>/logs/startup.log`.
- CI GitHub Actions Windows + Linux, installeur Windows et zip portable.
