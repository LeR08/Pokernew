# Solveur préflop (`crates/preflop`)

## Arbre simplifié
- **Ordre de parole préflop** : `[premier, …, SB, BB]`. En tête-à-tête : `[SB, BB]`.
- **Open** : tant que tout le monde s'est couché avant lui, un joueur se couche, relance à `open` bb, fait
  tapis (si l'option est cochée) ou, s'il est la SB, limpe (si l'option est cochée).
- **Réponses** : les joueurs derrière répondent dans l'ordre : fold, call, 3-bet (× l'open) ou tapis. Le
  **premier** qui ne se couche pas reste seul face à l'ouvreur, et les autres se couchent. C'est
  l'approximation multi-joueurs ; en tête-à-tête, c'est exact.
- Ensuite :
  - l'ouvreur face au 3-bet : fold, call, 4-bet (× le 3-bet) ou tapis ;
  - le 3-bettor face au 4-bet : fold, call ou tapis ;
  - face au tapis : fold ou call ;
  - après un limp, la BB checke, relance ou fait tapis.
- Une relance qui met au moins le tapis effectif devient le tapis.

## Valeurs
- **Fold** : le dernier joueur ramasse le pot.
- **Tapis payé** : chacun gagne pot × équité (table exacte des 169 × 169 classes de la phase 5, cartes
  bloquées comprises).
- **Flop vu sans tapis** : les équités sont multipliées par la part réalisée de chaque joueur (en position /
  hors position, réglable), puis renormalisées pour que les deux parts fassent toujours le pot. Avec 1 et 1,
  c'est l'équité brute.
- Les valeurs sont en bb par rapport aux tapis de départ ; leur somme vaut 0.

## CFR+
- Calcul vectoriel sur les 169 classes : à chaque nœud, chaque classe de chaque joueur a sa stratégie.
- La valeur contrefactuelle d'un joueur pour une classe `a` additionne les mains des autres joueurs,
  pondérées par la probabilité qu'ils arrivent là. L'adversaire encore présent est tiré en sachant `a`
  (cartes bloquées). Ceux qui se sont couchés ne comptent que par leur probabilité totale de se coucher.
- **Regret matching+** : regrets cumulés gardés positifs, mises à jour alternées, moyenne linéaire de la
  stratégie (l'itération t compte t fois).
- **Exploitabilité** : la meilleure réponse exacte de chaque joueur à la stratégie moyenne des autres. On
  fait la moyenne sur les joueurs de (meilleure réponse − valeur réelle), en bb par main. Elle est mesurée
  toutes les 10 itérations, et le calcul s'arrête sous la cible.

## Réalisation d'équité et rake
- **Position** : facteurs IP et OOP. En tête-à-tête, la SB est le bouton et parle en dernier après le flop.
- **Jouabilité** `k` : la part de chaque joueur vaut `e^k × facteur / (e^k × facteur + (1−e)^k × facteur adverse)`.
  Au-dessus de 1, les mains fortes réalisent plus que leur équité et les faibles moins. Le modèle à
  équité fixe surestimait toutes les mains faibles : UTG ouvrait 38 % en cash.
- **Rake** : pourcentage et plafond, pris seulement sur les pots qui voient le flop ou un abattage (pas de flop,
  pas de rake).

## Charts par défaut (`config/defaults/ranges`)
`examples/gen_charts.rs` les calcule pour 9 formats. Chaque format a 46 situations (13 pour le Spin) :
- **cash 6-max 100 bb** : rake 5 % plafonné à 3 bb, jouabilité 1,5, 85 % hors position.
  Ouvertures : UTG 17 %, HJ 21 %, CO 26 %, BTN 36 % ; la SB relance 39 % ou limpe 22 %.
- **MTT 10 à 100 bb** (6 joueurs, ante 0,12 bb) et **Spin 25 bb** : jouabilité 2,5, 80 % hors position.
  Ouvertures à 40 bb : UTG 23 %, CO 32 %, BTN 45 %.

Ce sont des points de départ, installés sans jamais remplacer les fichiers de l'utilisateur.

## Vérifications
- **Tête-à-tête 100 bb** (open 2,5, 3-bet ×3, 4-bet ×2,2, limp et tapis) : **0,0048 bb/main en 250
  itérations** (cible 0,005). Somme des gains : 0.
- **Recoupement** : avec le tapis pour seule relance (8 bb), on obtient les mêmes ranges que le solveur
  push/fold de la phase 5, lui-même vérifié contre les tableaux Nash publiés. Seules les mains
  indifférentes à 0,01 bb près sont tolérées.
- **6-max 100 bb** (approximation) : de 0,34 à 0,0038 bb/main en 300 itérations ; UTG ouvre plus serré que
  le bouton.

## Bibliothèque : fichiers `.lpsranges`
Une ligne `[dossier/nom]`, puis la range dans la notation d'Equity Lab. L'import accepte aussi les ranges
PioSOLVER / GTO+ (`AA:1.0, AKs: 0.5`). L'export suivi de l'import redonne les mêmes poids, combo par combo
(test).
