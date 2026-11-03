# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Versions follow the roadmap phases.

## [Unreleased] — Phase 4 : Equity Lab

### Ajouté / Added
- `evaluator` : notre évaluateur de mains de 5 à 7 cartes (masques de bits et table de quintes). Il compte juste toutes les mains de 5 et de 7 cartes et donne le même gagnant qu'eval7 sur 20 000 paires.
- `equity` : ranges pondérées et notation standard (`TT+`, `A5s-A2s`, `KQo:50%`, `AhKh`, `random`, `15%`) ; équité de 2 à 10 joueurs, exhaustive ou Monte-Carlo multi-cœur avec arrêt à la précision demandée ; décomposition d'une range sur un board (mains faites et tirages, combos exacts) ; filtre pour la street suivante ; textures sur les 1 755 flops ; classement préflop pour « top X % ».
- 20 cas de référence calculés avec eval7 : écart maximal de 0,033 % (critère : ±0,1 %).
- Module Equity Lab : calculateur (résultats, courbe de distribution), analyse de range (tableau, sélection → street suivante, histogramme d'équité contre une range adverse), textures de flop.
- Éditeur de range partagé : grille 13×13 (clic, glisser, souris ou tactile), poids, couleurs par case, texte, top X %, compteur de combos qui tient compte du board et des cartes mortes, bibliothèque `ranges/**.txt`.
- Replayer : lien « Ouvrir dans Equity Lab », qui pré-remplit ta main, les mains montrées et le board.

### Fair-play
- Equity Lab est verrouillé en mode jeu (tuile et commandes).

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
