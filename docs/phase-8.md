# Phase 8 — Entraîneur et analyse de mains

Statut : **implémentée — à tester sur ton PC** · remplace l'entraîneur de GTO Wizard et la revue façon
PokerSnowie.

## Critères d'acceptation (spec)
- les exercices sont notés d'après les solutions du cache ;
- le rapport d'analyse donne l'EV perdue main par main ;
- les modules d'étude se verrouillent en mode jeu.

## Ce qui a été fait
1. **`lps-trainer`** :
   - **exercices postflop** tirés d'une solution du cache, ou de toutes les solutions du même spot
     (autres boards, par exemple après un batch) :
     - modes : un nœud (première décision), une street, une main complète (l'adversaire et les cartes
       suivent la solution) ;
     - côté OOP, IP ou au hasard ; chrono ; « seulement mes points faibles » (les types de spots où tu perds
       le plus dans l'historique sont tirés plus souvent) ;
     - ta main est tirée dans ta range *à ce nœud*, donc les situations suivent la solution ;
   - **correction** : fréquence de chaque action dans la solution, EV de chaque action pour ta main, perte
     en bb et en % du pot, note (meilleur coup / correct / imprécision / erreur / grosse erreur) avec des
     seuils réglables (Réglages > Modules > Entraîneur) ;
   - **analyse d'une main jouée** :
     - pots en tête-à-tête qui vont au flop ;
     - les ranges viennent des charts du format choisi : l'open ou le 3-bet du relanceur, le call de
       l'autre ;
     - le spot est résolu, ou retrouvé dans le cache ;
     - ta vraie ligne est suivie : les tailles réelles sont ramenées à la plus proche de l'arbre (signalé
       quand l'écart est grand), et les vraies turn et river sont données ;
     - l'EV perdue est calculée à chaque décision.
2. **File d'analyse en tâche de fond** :
   - progression et annulation ;
   - respect du plafond mémoire des réglages ;
   - **pause automatique dès qu'une table est ouverte** ;
   - chaque spot résolu va dans le cache du solveur.
3. **Interface** : module Entraîneur avec cinq onglets :
   - Postflop, Préflop (questions tirées de tes charts, notées selon la fréquence dans le chart) ;
   - Push/fold (le quiz du module ICM) ;
   - Analyse de mains (rapport par main, du plus coûteux au moins coûteux, et fuites par type de spot, comme
     « turn · PFR · OOP · face à une mise ») ;
   - Historique (compte par note, courbe de la perte moyenne par jour).

   Dans le tracker, la liste des mains a un bouton « Analyser contre le solveur ».
4. **Fair-play** : le module est verrouillé en mode jeu, les commandes refusent de tourner, et la file
   d'analyse attend que les tables soient fermées.

## Tests
- les exercices des 3 modes sont notés contre la solution (la meilleure action perd 0, les autres ≥ 0) ; la
  main complète se joue jusqu'au bout ; les points faibles sont bien privilégiés ;
- une vraie main Winamax de la fixture est analysée, avec l'EV perdue à chaque décision ;
- les modules d'étude sont verrouillés en mode jeu (test du registre et de l'app) ;
- front : vitest et Playwright (exercices postflop et préflop, analyse depuis le tracker, historique, push/fold).

## Limites
- L'analyse ne couvre que les pots en tête-à-tête vus au flop, en pot simplement relancé ou 3-betté (pas
  les limps ni les 4-bets). Les ranges sont celles des charts, pas celles que l'adversaire joue vraiment.
- Une analyse résout un flop par main. Sur ton portable, compte 20 à 60 s par main avec l'arbre « simple ».
  Elle est faite pour tourner pendant que tu ne joues pas.
