# Phase 6 — Solveur postflop

Statut : **implémentée — à tester sur ton PC** (référence TexasSolver en attente, voir plus bas) · remplace PioSOLVER, GTO+ et TexasSolver.

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

## Résultats (3-4 nov. 2026)
- **Moteur intégré** : il compile avec le Rust actuel (3 petites adaptations, listées dans `FORK.md`). Ses
  propres tests passent (39).
- **Plafond de relances** ajouté à l'arbre, car le moteur n'en a pas. Sur le flop SRP 100 bb (QsJh2h), avec
  33 % et 75 % sur chaque street et une relance 3x :

  | relances max par street | nœuds | mémoire | compressée |
  |---|---|---|---|
  | sans limite | 1 968 | 13,0 Go | 6,8 Go |
  | 1 | 860 | 6,0 Go | 3,1 Go |

- **Performance** (exemple `bench`, ce flop, préréglage « SRP 100bb · 2 tailles ») : **0,435 % du pot en
  272 s** sur les 4 cœurs de la machine de build, et 295 s en compression 16 bits. Sur un vrai PC 8 cœurs,
  ça donnerait environ 2 à 2,5 minutes, donc juste au-dessus de l'objectif de 2 minutes. Sur ton portable
  (4 cœurs / 8 threads, 16 Go), compte 4 à 5 minutes, et il faut la compression pour cet arbre. Le
  préréglage « SRP 100bb · simple » (0,9 Go) va bien plus vite.
- **Verrouillage** : un adversaire bloqué en fold à 100 % fait miser QQ à 100 %, alors qu'à l'équilibre
  QQ ne bluffe qu'environ un tiers du temps (test).
- **Cache** : l'aller-retour est identique et le rechargement prend moins d'une seconde (test).
- **Tâche de fond** : pause, reprise et annulation testées ; le plafond mémoire refuse bien le départ.
- **Tests** : cargo test, clippy -D warnings, vitest (60) et Playwright (39) sont verts.

## En attente : référence TexasSolver
Les 5 spots sont prêts dans `crates/solver/tests/texassolver/` : 2 river, 1 turn, 2 flops, au format
TexasSolver (`.txt`) et au nôtre (`.json`). Il me manque les résultats de TexasSolver lui-même. Je n'ai pas
le droit de compiler et lancer un programme externe ici sans ton accord.
