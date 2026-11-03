# Phase 5 — ICM & push/fold

Statut : **implémentée — à tester sur ton PC** · remplace ICMIZER et HRC.

## Critères d'acceptation (spec)
- les ranges de push/fold Nash en tête-à-tête sont identiques aux tableaux Nash publiés ;
- la somme des $EV ICM de tous les joueurs est égale au prize pool ;
- la revue d'une main de tournoi s'ouvre depuis le tracker.

## Tâches
1. **`icm`** : les maths documentées dans `docs/icm.md`.
   - **Malmuth-Harville** (par défaut) : chaque joueur finit 1ᵉʳ avec une probabilité proportionnelle à
     son tapis, et ainsi de suite pour les places suivantes. Calcul exact par récursion mémorisée (jusqu'à
     10 joueurs). En plus : chip-EV pour comparer, prime de risque et matrice des bubble factors ;
   - **FGS** (Future Game Simulation) sur 1 à 3 mains, réglable : on simule les mains suivantes (blinds qui
     tournent, push/fold) avant d'appliquer l'ICM ;
   - **PKO** : la part de bounty (progressive : la moitié pour toi, la moitié sur ta tête), ajoutée à l'EV de
     chaque all-in.
2. **Solveur push/fold** : de 2 à 10 joueurs, jusqu'à 25 bb, en ICM ou en chip-EV, avec antes, ante BB et
   PKO. Les équités préflop des 169 × 169 classes viennent du moteur de la phase 4 (précalculées une fois).
   Méthode : meilleures réponses itérées (fictitious play) jusqu'à la convergence. L'exploitabilité est
   affichée.
   - grilles 13×13 par position (push, call contre chaque pousseur), EV de chaque main, curseurs « et si »
     sur les tapis ;
   - **mode exploitant** : tu fixes les ranges de call des adversaires (à la main ou depuis les stats du
     tracker), et tu obtiens la range de push qui rapporte le plus ;
   - **quiz** : spots au hasard, tu choisis push ou fold, noté contre la solution (les exercices complets
     arrivent en phase 8).
3. **Calculateur ICM** : tapis, structure des gains, bounties, antes. Résultats : $EV par joueur, prime de
   risque, bubble factors.
4. **Structures de gains** : fichiers `payouts/*.toml` (avec un schéma JSON), plus un import CSV
   (`place;gain`). **Je ne livre aucune structure inventée** (voir les questions).
5. **Revue de mains de tournoi** : dans le tracker, sur une main de tournoi, le bouton « Revue ICM » ouvre
   le spot pré-rempli avec les vrais tapis, le niveau et la structure de gains. On y voit l'EV de ta
   décision, l'EV de la meilleure, et l'écart en $ et en jetons. **Revue groupée** : toutes tes décisions
   all-in ou fold à 20 bb ou moins dans un tournoi, triées par EV perdue.
6. **Fair-play** : le module ICM est déjà verrouillé en mode jeu, et ses commandes refusent de tourner
   pendant qu'une table est ouverte.

## Tests
- ICM : la somme des $EV est égale au prize pool (cas aléatoires de 2 à 10 joueurs) ; cas calculés à la main ;
  symétrie (tapis égaux, EV égales) ; chip-EV.
- Push/fold tête-à-tête : comparaison avec un tableau Nash publié (source citée dans le test).
- Multi-joueurs : l'exploitabilité tombe sous un seuil ; un joueur qui dévie ne gagne rien.
- Revue : depuis une main de la fixture Winamax (tapis, niveau, gains) jusqu'à l'EV de la décision.
- Front : vitest et Playwright (calculateur, solveur, revue depuis le tracker, verrou en mode jeu).

## Réponse de l'utilisateur
Les répartitions de gains changent avec le nombre d'inscrits : tu les saisis à la main dans **ICM >
Structures de gains** (montants ou pourcentages, ou un CSV collé). Aucune structure n'est livrée.

## Résultats (3 nov. 2026)
- **Nash tête-à-tête** contre les tableaux publiés de HoldemResources (sans ante), sur 12 tapis de 2 à
  19,5 bb : **4 003 décisions vérifiées, 3 999 identiques**. Les 4 autres (53s à 4 bb, 96o à 5 bb, 94s à
  6 bb, 84s à 9 bb) sont des mains **indifférentes** : pousser ou se coucher y diffère de moins de 0,01 bb
  dans notre solution convergée (exploitabilité de 0,00002 bb). Le test ne tolère que ce cas, et sous 1 %.
- **ICM** : la somme des $EV est égale au prize pool sur 180 cas tirés au hasard (2 à 10 joueurs), plus
  des cas calculés à la main ; les joueurs éliminés se partagent les dernières places.
- **Table d'équité préflop** : 169 × 169 classes, exacte (93 769 situations différentes énumérées sur
  tous les boards).
- **Revue** : elle trouve et évalue tes décisions push/fold dans les mains Winamax de la fixture (tapis,
  call, fold). Elle s'ouvre depuis le replayer (« Revue ICM ») et depuis la liste des tournois
  (« Revue push/fold »).
- **Tests** : cargo test, clippy -D warnings, vitest et Playwright sont verts.

## Limites
- Push/fold : seul le premier qui paie va à l'abattage (pas de sur-call).
- L'ICM de la revue ne voit que la table : c'est exact en Expresso et en table finale, et une
  approximation avant. Tu peux ajouter les tapis des autres tables à la main.

## À tester sur ton PC
1. **Structures de gains** : saisis la répartition d'un de tes tournois.
2. **Push/fold** : tête-à-tête à 10 bb, puis 6 joueurs en ICM avec ta structure ; bouge les curseurs « Et
   si… » ; fixe la range d'un adversaire (mode exploitant).
3. **Calculateur ICM** : tapis et gains, bubble factors.
4. **Revue** : depuis le replayer, sur une main de tournoi courte (« Revue ICM »), puis un tournoi entier
   depuis Tracker > Tournois (« Revue push/fold »).
5. **Quiz** : quelques spots.
6. Ouvre une table : le module ICM se verrouille.
