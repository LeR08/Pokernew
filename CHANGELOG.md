# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Versions follow the roadmap phases.

## [Unreleased] — Phase 2 : Tracker (première partie)

### Ajouté / Added
- `stats-engine` : langage de formules (`+ - * /`, comparaisons, `and/or/not`, `if()`) compilé en SQL avec paramètres liés ; stats en TOML (`stats/*.toml`, rechargées à chaud).
- 13 stats par défaut, chacune avec une explication en français et en anglais.
- Tracker : vue d'ensemble (résultat d'abord), rapports par position / format / mois / limite / compte, graphique (bb ; « Afficher en € » pour le cash et les tournois), tournois avec ROI et ITM, liste des mains, éditeur de stats avec aperçu en direct.
- Replayer : table, cartes, mises, pot, pas à pas, lecture automatique, flèches du clavier, montants en bb ou en jetons / €.
- Référence indépendante pour les stats (`tools/reference_stats.py`).
- Sessions (regroupées par trous de 30 min) et **bilan de session** : mains à revoir et bons coups, expliqués, avec un lien vers le replayer ; alerte de tilt.
- Détecteur de fuites (`leaks/targets.toml`, intervalle de Wilson 95 %, résultat dans le spot).
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
