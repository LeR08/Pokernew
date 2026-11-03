# Solveur postflop (`crates/solver`)

## Moteur : Discounted CFR (postflop-solver, intégré)
Le moteur est le **Counterfactual Regret Minimization** de postflop-solver, dans sa variante *Discounted*
(DCFR). Le principe :

- À chaque itération, on parcourt tout l'arbre pour chaque joueur, avec les ranges des deux joueurs.
- Pour chaque main, à chaque nœud de décision, on calcule la valeur contrefactuelle de chaque action : ce
  qu'elle rapporterait si le joueur jouait pour arriver à ce nœud.
- Le **regret** d'une action, c'est ce qu'elle rapporte de plus que la stratégie courante. Les regrets
  s'accumulent.
- La stratégie suivante joue chaque action en proportion de son regret positif (*regret matching*).
- DCFR **escompte** les vieux regrets à chaque itération (puissance 1,5 pour les positifs, 0,5 pour les
  négatifs, 2 pour la stratégie moyenne). Les erreurs des débuts pèsent donc vite moins, et la convergence est
  bien plus rapide.
- La **stratégie moyenne** converge vers un équilibre de Nash.

Ce qui accélère le calcul :
- l'**isomorphisme des couleurs**, qui ne calcule qu'une fois les turns et rivers équivalentes ;
- l'évaluation des abattages en temps linéaire, par tri des mains ;
- le parallélisme (rayon) ;
- en option, le stockage sur 16 bits, qui divise la mémoire par deux.

## Exploitabilité
C'est ce que gagneraient les deux joueurs, en moyenne, à jouer la meilleure réponse contre la stratégie de
l'autre. On l'exprime en **% du pot de départ**. 0 % correspond à l'équilibre exact. La cible par défaut est
0,5 %.

## Notre surcouche
- **Spot** : les ranges sont écrites dans la notation d'Equity Lab et converties combo par combo. Les montants
  sont en bb, avec une précision de 0,01 (le moteur travaille en centièmes de bb).
- **Arbre** : tailles par street et par joueur (`33%`, `3x`, `e`, `a`), donks, seuils de tapis, fusion
  des tailles proches. On y ajoute un **plafond de relances par street**, que le moteur n'a pas : au-delà, il
  ne reste que suivre ou se coucher. Sans ce plafond, un arbre à 2 tailles avec des relances 2,5x pèse
  13 Go ; avec une seule relance, 3 Go compressé.
- **Calcul en tâche de fond** : une itération à la fois, avec pause, reprise et annulation entre deux
  itérations. L'exploitabilité est mesurée toutes les 10 itérations. Le calcul refuse de démarrer si la
  mémoire estimée dépasse le plafond réglé.
- **Verrouillage** : une stratégie fixée pour un nœud (fréquence de chaque action, par main). Le moteur
  l'applique avant de calculer. Pour relancer avec de nouveaux verrous, on reconstruit la partie.
- **Cache** : `solver-cache/<empreinte>.bin` (format du moteur, compressé en zstd) et `index.json`.
  L'empreinte vient du spot et de ses verrous (FNV-1a de la configuration). Le rechargement ne recalcule rien.
- **Lecture** : à chaque nœud, stratégie, EV et équité de chaque main, EQR = EV ÷ (équité × pot), grilles
  13×13 pondérées par la présence de chaque combo dans la range.
