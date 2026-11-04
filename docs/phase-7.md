# Phase 7 — Ranges et solveur préflop

Statut : **implémentée — à tester sur ton PC** · remplace la partie préflop de GTO Wizard.

## Critères d'acceptation (spec)
- l'import et l'export de la bibliothèque de ranges font l'aller-retour à l'identique ;
- le solveur préflop tête-à-tête converge sous l'exploitabilité visée.

## Tâches
1. **Bibliothèque de charts** (elle existe depuis la phase 4 : `ranges/**/*.txt`) :
   - un **navigateur de charts** par format (cash 6-max 100 bb, MTT 10/15/20/25/40/60/100 bb, Spin à 3) et
     par situation (open, 3-bet, call, 4-bet, contre un limp), avec l'éditeur de range partagé ;
   - **export et import** d'un dossier entier en un seul fichier (`.lpsranges`, du texte lisible), plus
     l'import des ranges au format PioSOLVER / GTO+ (`AA:1.0, AKs:0.5`) ;
   - aucune range « propriétaire » livrée : les dossiers proposés sont vides, à remplir par toi ou par le
     solveur.
2. **Solveur préflop** (`lps-preflop`) :
   - arbre simplifié : open (une taille), limp de la SB (option), 3-bet (× l'open), 4-bet (× le 3-bet),
     tapis à chaque étape (option), tapis de 10 à 200 bb, blinds et antes ;
   - **tête-à-tête complet** par CFR+ sur les 169 mains, avec les équités préflop exactes et les cartes
     bloquées (table de la phase 5) ;
   - **multi-joueurs approché** (jusqu'à 6) avec le même arbre de tailles fixes : le premier à relancer ouvre,
     les suivants répondent dans l'ordre, et le premier qui agit (call, 3-bet, tapis) reste seul face à
     l'ouvreur ;
   - quand on voit le flop sans être à tapis, chaque joueur réalise une part de son équité (réglable : IP et
     OOP) ;
   - exploitabilité suivie pendant le calcul (meilleure réponse exacte du modèle) ; arrêt sous la cible ;
   - les solutions s'enregistrent dans la bibliothèque, une range par situation et par action.
3. **Interface** : module Préflop avec deux onglets, Charts (bibliothèque) et Solveur (réglages, calcul,
   arbre cliquable, grille 13×13 par action, enregistrement).
4. **Fair-play** : module d'étude, verrouillé en mode jeu.

## Tests
- export → import → mêmes poids, combo par combo (Rust et front), y compris les poids partiels et les noms de
  dossiers avec espaces ;
- tête-à-tête : l'exploitabilité passe sous la cible ; la somme des gains vaut 0 (chip EV) ;
- recoupement : avec le tapis pour seule relance, le solveur préflop donne les mêmes ranges que le solveur
  push/fold de la phase 5 (lui-même vérifié contre les tableaux Nash publiés) ;
- multi-joueurs : l'exploitabilité baisse et chaque situation somme à 100 % ;
- front : vitest et Playwright (charts, import et export, calcul, enregistrement).

## Résultats (4 nov. 2026)
- **Aller-retour export → import** : mêmes poids combo par combo, testé côté Rust (bundle et dossiers de la
  bibliothèque) et dans l'interface (Playwright).
- **Tête-à-tête** : 0,0048 bb/main d'exploitabilité en 250 itérations, pour une cible de 0,005 (critère
  atteint).
- **Recoupement** : en tapis seulement, on retrouve les ranges du solveur push/fold de la phase 5.
- **6-max** (approximation) : l'exploitabilité tombe de 0,34 à 0,0038 bb/main en 300 itérations.
- **Tests** : cargo test, clippy -D warnings, vitest et Playwright sont verts.

## Limites
- Une seule taille par étape (open, 3-bet, 4-bet) en plus du tapis. Pas de cold call à plusieurs, ni de
  squeeze.
- La réalisation d'équité postflop est un simple facteur (IP/OOP), pas un vrai calcul postflop. Pour ça, il
  y a le solveur postflop.
