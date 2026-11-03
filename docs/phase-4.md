# Phase 4 — Equity Lab

Statut : **implémentée — à tester sur ton PC** · remplace Equilab, PokerStove et Flopzilla.

## Critères d'acceptation (spec)
- 20 cas d'équité de référence identiques à un calculateur public à ±0,1 % près ;
- le nombre de combos de la décomposition d'une range sur un board est exact.

Le calculateur public de référence est **eval7** (bibliothèque open source en C, utilisée par beaucoup de
calculateurs Python). Les 20 valeurs de référence sont calculées une fois avec eval7, de façon **exhaustive**
(aucun tirage aléatoire), avec `tools/reference_equity.py`. Elles sont ensuite figées dans
`crates/equity/tests/reference_cases.json`, et `cargo test` compare notre moteur à ces valeurs.

## Tâches
1. **`evaluator`** : notre propre évaluateur de mains de 5 à 7 cartes (masques de bits et tables de
   correspondance précalculées). Les maths sont documentées dans le code et dans `docs/evaluator.md`.
   Tests : il trouve le bon nombre de mains de chaque catégorie sur les 2 598 960 mains de 5 cartes et sur
   les 133 784 560 mains de 7 cartes ; sur 20 000 paires de mains aléatoires, il donne le même gagnant
   qu'eval7.
2. **`equity`** :
   - cartes, 1 326 combos, ranges pondérées. Notation standard : `AKs`, `TT+`, `A5s-A2s`, `KQo:50%`,
     `AhKh`, `random`, `15%` (top 15 %). Les cartes mortes et le board sont pris en compte ;
   - calcul de 2 à 10 joueurs, chacun avec une main ou une range, sur un board de 0 à 5 cartes. Le calcul
     est **exhaustif** quand il est rapide, sinon c'est un **Monte-Carlo** multi-cœur (rayon) qui s'arrête dès
     la précision demandée (±0,1 % par défaut, intervalle à 95 %). Résultats : équité, % de victoires et
     d'égalités, et équité de chaque combo (pour la courbe de distribution) ;
   - classement préflop des 169 mains (équité contre une main au hasard), calculé par notre moteur et figé
     dans `crates/equity/data/preflop_ranking.json`. C'est lui qui donne le curseur « top X % » ;
   - **décomposition d'une range sur un board** (Flopzilla) : catégories de mains faites (quinte flush …
     hauteur) et tirages (couleur, quinte bilatérale, ventrale, backdoors, overcards). Combos et % exacts,
     définitions écrites dans `docs/equity.md` ;
   - filtre « garder ces catégories » pour construire la range de la street suivante (flop → turn → river) ;
   - histogramme de l'équité de la range contre une range adverse (tranches de 10 %) ;
   - **textures de flop** : agrégat sur les 1 755 flops stratégiquement différents (ou un sous-ensemble :
     monocolores, pairés, …), calculé exactement sur les 22 100 flops.
3. **Bibliothèque de ranges** : fichiers `ranges/**/*.txt` dans le dossier de données, rangés en
   sous-dossiers (par ex. `6max/100bb/CO_open.txt`), en notation standard. On peut les éditer à la main.
4. **Commandes Tauri** : calcul (en tâche de fond, annulable), décomposition, filtre, textures, bibliothèque.
   Elles refusent de tourner en mode jeu.
5. **Interface** :
   - éditeur de range partagé (il servira aussi au solveur, à l'ICM et au préflop) : grille 13×13 avec
     clic ou glisser, poids de 0 à 100 %, choix des couleurs par case, saisie en texte, curseur top X %,
     compteur de combos qui tient compte du board et des cartes mortes, bibliothèque ;
   - module **Equity Lab** avec 3 onglets : Calculateur, Analyse de range, Textures de flop ;
   - bouton « Equity Lab » dans le replayer, qui pré-remplit ta main et le board (verrouillé en mode jeu).
6. **Fair-play** : Equity Lab est un module d'étude, verrouillé dès qu'une table est ouverte (comme le
   solveur). Rien de ce module n'est jamais affiché dans le HUD.

## Crates touchés
`evaluator`, `equity`, `app/src-tauri`, front (`src/lib/range.ts`, `src/components/range/`,
`src/modules/equity/`).

## Tests
- `evaluator` : comptage exhaustif des catégories (5 et 7 cartes) et comparaison avec eval7 ;
- `equity` : 20 cas de référence eval7 (±0,1 %), notation des ranges (cas partagés avec le front dans
  `crates/equity/data/range_cases.json`), comptage exact des combos de la décomposition (cas comptés à la
  main + vérification indépendante par force brute), 1 755 flops (poids total 22 100) ;
- performances (en release) : main contre main préflop exhaustif < 50 ms ; deux ranges préflop à ±0,1 %
  en moins d'1 s (`cargo run --release -p lps-equity --example bench`) ;
- front : vitest (notation, grille), Playwright (calculateur, éditeur, analyse, verrouillage en mode jeu).

## Résultats (3 nov. 2026)
- **20 cas eval7** : tous à ±0,1 %. Écart maximal de 0,033 % : 18 cas exhaustifs, identiques à 10⁻⁶ près
  (arrondi du fichier), et 2 en Monte-Carlo.
- **Évaluateur** : les 133 784 560 mains de 7 cartes sont bien comptées dans chaque catégorie, et il donne le
  même gagnant qu'eval7 sur 20 000 paires.
- **Décomposition** : les cas comptés à la main sont justes, et les catégories découpent toujours toute la
  range.
- **Flops** : 1 755 classes, 22 100 flops ; les comptes monocolore, pairé, brelan et arc-en-ciel sont
  vérifiés.
- **Performances** (4 cœurs de la machine de build) : AA contre KK exhaustif en 19 ms, deux ranges préflop à
  ±0,1 % en 41 ms.
- **Tests** : `cargo test`, `clippy -D warnings`, vitest (60), Playwright (30) : tout est vert.

Écart à la spec : Equity Lab ne figure pas dans la liste des modules verrouillés de la spec, mais une équité
calculée pendant une main est une aide en temps réel. Je le verrouille donc en mode jeu, comme le solveur.

## À tester sur ton PC
1. Ouvre **Equity Lab** et lance un calcul : `AsKs` contre `QQ+,AK`, puis ajoute un board.
2. Ouvre l'éditeur (« Range… ») : clic, glisser, « Choisir les couleurs », texte, curseur top X %.
   Enregistre une range dans la bibliothèque (par ex. `6max/100bb/BTN_open`).
3. **Analyse de range** : coche « Top paire » et « Tirage couleur » → « Garder la sélection », ajoute une
   turn, et regarde l'histogramme contre une range adverse.
4. **Textures de flop** : lance l'agrégat sur les flops monocolores.
5. Depuis le **replayer**, ouvre une main dans Equity Lab.
6. Ouvre une table Winamax : Equity Lab doit se verrouiller.
