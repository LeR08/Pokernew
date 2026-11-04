# Charts préflop par défaut / Default preflop charts

Ces ranges sont calculées par le solveur préflop de l'app (`cargo run --release -p lps-preflop --example gen_charts`).
Ce sont des **points de départ**, à modifier selon tes envies. L'app ne les recopie jamais par-dessus tes fichiers.

- **Le modèle** : arbre simplifié (open, 3-bet ×3, 4-bet ×2,2, tapis, limp de la SB). À plusieurs, seul le premier
  joueur qui répond à l'open reste face à lui. Quand on voit le flop, la réalisation d'équité est calibrée sur des
  repères connus :
  - **cash 6-max 100 bb** : rake de 5 % plafonné à 3 bb, jouabilité 1,5, 85 % de réalisation hors position ;
  - **MTT** (6 joueurs, ante de 0,12 bb) et **Spin** : jouabilité 2,5, 80 % hors position.
- **Ce que veut dire un poids** : la part de la main qui prend cette action dans cette situation. Par exemple,
  `A5s:81%` dans « UTG/open » signifie qu'A5s ouvre 81 % du temps.
- « X vs 3bet/… » fait la moyenne sur les joueurs qui 3-bettent, pondérée par la fréquence de chaque cas.

These ranges come from the app's preflop solver: starting points, to edit as you like; the app never copies them over your
files. Simplified tree (first responder only in multi-way pots), equity realisation calibrated as above.
