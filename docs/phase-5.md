# Phase 5 — ICM & push/fold

Statut : **plan — à valider** · remplace ICMIZER et HRC.

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

## Questions pour toi
1. **Structures de gains** : les historiques et les résumés Winamax donnent le prize pool, mais pas la
   répartition par place. Pour la revue ICM de tes MTT, il me faut la répartition :
   - soit tu me donnes une capture (ou une copie) de l'onglet « Gains » du lobby d'un tournoi type ;
   - soit tu les saisis toi-même dans l'app : un tableau ou un CSV `place;gain`, enregistré pour être
     réutilisé.
   Pour les **Expresso**, tu peux me donner la grille de gains (multiplicateurs et répartition) ? Je ne veux
   pas l'inventer.
2. Les formats que tu joues le plus : MTT KO (PKO), Expresso, autres ? Ça fixe l'ordre des préréglages.
