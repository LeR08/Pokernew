# ICM et push/fold (`crates/icm`)

## Modèles d'équité de tournoi
### Malmuth-Harville (ICM, par défaut)
- Chaque joueur finit 1ᵉʳ avec une probabilité égale à `son tapis / total des tapis`.
- Une fois des joueurs placés, la place suivante revient à chaque joueur restant avec une probabilité égale à
  `son tapis / total des tapis restants`.
- Son équité est la somme, sur toutes les places, de P(place) × gain de la place.

Le calcul est **exact** : une programmation dynamique sur les ensembles de joueurs déjà placés (2ⁿ
ensembles, jusqu'à 16 joueurs avec des jetons). `prob[S]` est la probabilité que les joueurs de S prennent
les |S| premières places. Les joueurs éliminés (0 jeton) prennent les dernières places et se partagent leurs
gains à parts égales. La somme des équités est donc toujours égale à la somme des gains des places
atteignables. C'est un critère de la phase, testé sur 180 cas tirés au hasard, de 2 à 10 joueurs.

### Chip EV
`équité = part des jetons × prize pool`. Sert à comparer.

### Bubble factor et prime de risque
- Bubble factor de i contre j : ce que i perd en perdant un tapis contre j (pour le plus petit des deux
  tapis), divisé par ce qu'il gagne en le gagnant. 1 = les jetons valent leur poids en argent.
- Prime de risque = `BF / (1 + BF) − 50 %`. C'est l'équité en plus dont i a besoin pour payer un tapis
  sans argent mort, par rapport au chip EV.

### FGS (Future Game Simulation)
Au lieu d'appliquer l'ICM aux tapis de fin de main, on joue 1 à 3 mains de plus, en push/fold à
l'équilibre, avec les blinds qui avancent d'un siège à chaque main. L'ICM ne s'applique qu'après la
dernière. On ajoute ainsi la valeur de la position et du tapis par rapport aux blinds, que l'ICM seul
ignore. Les résultats sont mis en cache par tapis. Si un joueur serait à tapis rien qu'en payant les blinds,
la simulation s'arrête là.

### KO (PKO)
Chaque joueur a une valeur de bounty : l'argent encaissé tout de suite par celui qui l'élimine. Dans un
KO progressif, c'est la moitié de son bounty (l'autre moitié s'ajoute au bounty du gagnant). Dans la revue,
la part encaissée est réglable (50 % par défaut). Les bounties s'ajoutent à la valeur de chaque fin de main
où un joueur est éliminé.

## Solveur push/fold
### Le jeu
- Les joueurs sont rangés dans l'ordre de parole préflop : `[premier, …, SB, BB]`. En tête-à-tête,
  c'est `[SB, BB]`.
- On pose les antes (ou l'ante de la BB), puis les blinds.
- Tant que tout le monde s'est couché avant lui, chaque joueur se couche ou fait tapis.
- Face au tapis, les joueurs suivants paient ou se couchent dans l'ordre. Le **premier qui paie** va à
  l'abattage, et les autres se couchent : les sur-calls sont rares à ces tapis et ne sont pas modélisés.
- Si tout le monde se couche jusqu'à la BB, elle ramasse les blinds.

### Valeurs
Chaque façon de finir la main donne les tapis finaux. Leur valeur vaut, selon le modèle :
- les jetons (chip EV) ;
- l'argent de l'ICM, avec les tapis des autres tables s'ils sont connus, plus les bounties ;
- la valeur FGS.

### Mains
On travaille sur les 169 classes. L'équité à tapis de chaque classe contre chaque autre est **exacte**. Elle
est calculée une fois (`crates/equity/examples/gen_preflop_matrix.rs` → `data/preflop_matrix.txt`) en
énumérant tous les boards, pour chaque paire de combos sans carte commune. Les paires qui ne diffèrent que
par un échange de couleurs ne sont calculées qu'une fois.

On tient compte des cartes bloquées entre les deux joueurs de l'abattage : la main adverse est tirée en
sachant la nôtre. Les probabilités de fold des autres joueurs les ignorent.

### Résolution : fictitious play
- À chaque itération, chaque décision reçoit sa meilleure réponse aux stratégies **moyennes** des
  autres.
- La moyenne se rapproche de cette meilleure réponse avec un pas de 1/(t+2).
- La stratégie moyenne converge vers un équilibre.
- **Exploitabilité** : ce qu'un joueur pourrait encore gagner en changeant de stratégie. On la somme sur
  ses décisions, pondérée par la probabilité d'arriver à chacune, et on affiche celle du joueur le plus
  exploitable (en bb ou en €).

### Mode exploitant
Une décision peut être **verrouillée** sur une range donnée : la range réelle d'un adversaire, ou une range
tirée de ses stats. Les autres décisions jouent alors leur meilleure réponse à cette range.

### Référence (critère de la phase)
Le test `tests/hu_nash.rs` compare notre équilibre tête-à-tête aux tableaux Nash publiés par HoldemResources
(`tests/hune_hu_noante.txt` : sans ante, SB 0,5 bb, BB 1 bb, publiés en 2007). Les stacks testés vont de 2 à
19,5 bb. Une main est jouée quand le stack est dans l'intervalle publié. On écarte les mains dont la limite
publiée tombe à moins de 0,15 bb du stack testé, car le tableau est arrondi à 0,1 bb et ces mains jouent une
stratégie mixte.

## Revue de mains de tournoi
- Le spot est reconstruit depuis l'historique : tapis de début de main dans l'ordre de parole, blinds et
  antes.
- On regarde la décision du héros :
  - **tapis** : tout le monde s'est couché avant lui et il a fait tapis (ou s'est couché) ;
  - **payer** : un seul joueur a fait tapis avant lui, les autres se sont couchés, et il a payé (ou s'est
    couché).
- Tout le reste n'est pas une décision push/fold et est ignoré : limps, relances qui ne sont pas à tapis,
  plusieurs tapis. Les décisions au-dessus de 20 bb aussi.
- On résout l'équilibre pour tout le monde, puis on lit, au nœud du héros et pour sa classe de main, la
  valeur de chaque option :
  - en jetons (chip EV) ;
  - en argent, si une structure de gains est donnée.
- La perte, c'est l'écart entre la meilleure option et l'option jouée.

Limite : les historiques Winamax ne donnent ni les tapis des autres tables ni le nombre de joueurs restants.
L'ICM ne voit donc que la table. C'est exact en Expresso et en table finale, et une approximation avant. Tu
peux ajouter les tapis des autres tables à la main. Les structures de gains sont saisies par toi
(`payouts/*.toml`).
