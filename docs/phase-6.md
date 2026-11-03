# Phase 6 — Solveur postflop

Statut : **en cours** · remplace PioSOLVER, GTO+ et TexasSolver.

## Critères d'acceptation (spec)
- 5 spots de référence donnent la même EV que TexasSolver, à 1 % du pot près ;
- objectif de performance : un flop SRP 100 bb avec 2 tailles de mise, résolu à 0,5 % d'exploitabilité,
  en moins de 2 minutes sur un PC 8 cœurs / 32 Go ;
- une solution en cache se recharge instantanément ;
- le verrouillage de nœuds fonctionne.

## Moteur
`postflop-solver` (Wataru Inariba, AGPL-3.0, Discounted CFR, isomorphisme des couleurs, compression
16 bits) est intégré dans `crates/solver/engine`. Son développement est en pause : c'est désormais notre
fork. Les changements par rapport à l'original sont listés dans `FORK.md` (adaptation aux compilateurs
récents, versions de dépendances figées).

## Tâches
1. **`lps-solver`** (notre surcouche) :
   - **spot** : ranges OOP et IP dans notre notation, board de 3 à 5 cartes, pot, tapis effectif, rake
     (% et plafond). Les montants sont en bb, à 0,01 près ;
   - **arbre de mises** : par street et par joueur, mises (% du pot, `x` de la dernière mise, `e`
     géométrique, `a` tapis), relances et donks, seuils de tapis (ajout et forcé), fusion des tailles
     proches. Préréglages dans `solver_trees/*.toml` (schéma JSON). On compte les nœuds de l'arbre avant le
     calcul ;
   - **calcul en tâche de fond** : progression (itération, exploitabilité en % du pot, temps), pause,
     reprise, annulation, nombre de threads. La mémoire nécessaire est estimée avant le départ, et le calcul
     refuse de partir au-delà du plafond réglé ;
   - **lecture d'un nœud** : actions possibles, stratégie de chaque main, EV, équité, EQR, poids de la range,
     grille 13×13 par action, fréquences globales. On navigue action par action et carte par carte ;
   - **verrouillage de nœuds** : on fixe la stratégie d'un nœud (par ex. « l'adversaire c-bet 100 % »), puis
     on relance le calcul du reste ;
   - **cache** : chaque solution est enregistrée compressée dans `solver-cache/`, indexée par l'empreinte
     du spot (ranges, board, tapis, arbre). On la recharge sans rien recalculer ;
   - **export** de la stratégie d'un nœud en CSV ou JSON ;
   - **batch** : une file de flops (les 1 755, ou un sous-ensemble filtré par texture comme dans Equity
     Lab) pour un spot. Chaque solution va dans le cache, et un rapport donne par flop la fréquence de mise,
     la répartition des tailles et l'EV.
2. **Commandes Tauri**, refusées en mode jeu (le module Solveur est déjà verrouillé).
3. **Interface** :
   - réglages du spot avec l'éditeur de range partagé et ses préréglages, aperçu de l'arbre (nombre de
     nœuds) et estimation de la mémoire ;
   - suivi du calcul (courbe de l'exploitabilité) ;
   - **visionneuse** : fil des actions cliquable, grille 13×13 colorée par action, panneau d'une main
     (fréquences, EV, équité), décomposition de la range par catégorie de main (celle d'Equity Lab) ;
   - deux solutions côte à côte ;
   - bibliothèque des solutions en cache ; batch avec son rapport par texture.

## Référence TexasSolver
TexasSolver ne publie pas d'EV de référence. Il faut donc le faire tourner sur les 5 spots (fichiers de
commande dans `crates/solver/tests/texassolver/`) :
- soit je le compile et je le lance ici, mais cela demande ton autorisation ;
- soit tu le lances sur ton PC (version Windows) et tu m'envoies les résultats.

## Tests
- arbre et spot : validation des entrées, nombre de nœuds, empreinte stable ;
- calcul : l'exploitabilité descend sous la cible ; la somme des EV des deux joueurs plus le rake égale le
  pot ; pause, reprise et annulation ;
- verrouillage : un adversaire bloqué en fold à 100 % fait miser 100 % le joueur suivant ;
- cache : aller-retour à l'identique, rechargement en moins d'une seconde ;
- performances : exemple `bench` (flop SRP 100 bb, 2 tailles) mesuré ici ;
- front : vitest et Playwright (réglages, calcul, visionneuse, verrouillage, cache).
