# Phase 2 — Tracker (stats, rapports, graphiques, replayer)

Statut : **première partie livrée, en attente de ton test** · Choix de l'utilisateur (3 nov. 2026) :
- stats **simples**, chacune avec une explication en clair dans l'app ;
- objectif : **être en bénéfice** → le gain est la première chose affichée partout ;
- gains en **jetons / big blinds** par défaut, case à cocher **« Afficher en € »**.

## Critères d'acceptation (spec)
- VPIP, PFR, 3B, WTSD, W$SD sur une fixture d'environ 500 mains = référence comptée indépendamment ;
- la courbe de gains finit exactement à la somme des `net_won` ;
- une stat créée dans l'UI apparaît dans les rapports (et dans le HUD en phase 3) ;
- le replayer rejoue toutes les mains des fixtures.

## Les stats (jeu par défaut, modifiable dans `stats/*.toml`)

| Stat | Ce qu'elle dit | Calcul |
|---|---|---|
| Gain (bb/100) | Ce que tu gagnes en moyenne pour 100 mains, en big blinds | net en bb × 100 / mains |
| VPIP | Combien de mains tu joues : % de mains où tu mets de l'argent volontairement avant le flop | `cnt_vpip / cnt_hands` |
| PFR | Combien de mains tu joues en relançant avant le flop | `cnt_pfr / cnt_hands` |
| 3-Bet | Quand quelqu'un relance avant toi, combien de fois tu sur-relances | `cnt_3bet / opp_3bet` |
| Fold to 3-Bet | Quand tu as ouvert et qu'on te sur-relance, combien de fois tu te couches | |
| Vol (ATS) | Quand tout le monde s'est couché jusqu'à toi au CO, au bouton ou en SB, combien de fois tu relances | |
| Fold to steal | En blinde, face à une tentative de vol, combien de fois tu te couches | |
| C-Bet flop | Quand tu as relancé avant le flop, combien de fois tu mises au flop | |
| Fold to C-Bet | Face à une mise de continuation au flop, combien de fois tu te couches | |
| WTSD | Quand tu vois le flop, combien de fois tu vas jusqu'à l'abattage | |
| W$SD | Quand tu vas à l'abattage, combien de fois tu gagnes | |
| WWSF | Quand tu vois le flop, combien de fois tu gagnes le pot | |
| AF | Agressivité après le flop : (mises + relances) / suivis | |

Repères : un joueur rentable en 6-max a souvent un VPIP de 20 à 28 % et un PFR proche de son VPIP
(écart inférieur à 5 points). Un gros écart VPIP − PFR signale trop de suivis passifs.

## Unités
- Par défaut : **big blinds** (bb/100, courbe en bb). En cash, c'est l'unité standard. En tournoi,
  les blindes montent, donc compter en bb rend les mains comparables entre niveaux.
- **« Afficher en € »** :
  - cash : les gains réels en € ;
  - tournois : le résultat réel de chaque tournoi (gains + bounties + tickets − buy-ins × entrées),
    pris dans les résumés Winamax, avec le ROI.

  On ne convertit pas les jetons d'une main en € : en tournoi, un jeton n'a pas de valeur fixe (ICM).

## Tâches
1. `stats-engine` : langage d'expression (`+ - * /`, comparaisons, `and/or/not`, `if()`, compteurs,
   champs de main) compilé en SQL avec paramètres liés ; définitions de stats en TOML (famille `stats`).
2. Requêtes : vue d'ensemble, rapport par position / par format / par mois, courbe de gains
   (bb, € cash, € tournois ; abattage / hors abattage), liste des tournois avec ROI, liste des mains.
3. Filtres : période, format (cash / MTT / Spin), salle, compte, position.
4. UI Tracker : Vue d'ensemble, Positions, Graphique, Tournois, Mains → replayer.
5. Replayer : table, cartes, mises, pot, pas à pas et lecture automatique.
6. Éditeur de stats : formule avec validation en direct et aperçu.
7. Référence indépendante pour le critère d'acceptation : script Python qui recompte les stats
   directement depuis le texte des historiques.

Viendront ensuite dans la même phase, une fois cette base validée : constructeur de rapports
glisser-déposer, notes et auto-notes, détecteur de fuites, bankroll et sessions.

## Résultats — première partie (3 nov. 2026)

| Critère | Résultat |
|---|---|
| VPIP, PFR, 3B, WTSD, W$SD = référence indépendante (≈ 500 mains) | ✅ **identiques** sur les 496 mains des fixtures. Référence : `tools/reference_stats.py`, qui recompte directement depuis le texte, sans code Rust. Résultat : VPIP 144/496, PFR 102/496, 3-bet 6/136, WTSD 71/152, W$SD 28/71. Test : `core_stats_match_the_independent_reference` |
| La courbe finit sur la somme des `net_won` | ✅ en bb et en € cash. Test : `graph_ends_on_the_sum_of_results` |
| Une stat créée dans l'UI apparaît dans les rapports | ✅ Playwright : création dans « Mes stats », puis visible dans la vue d'ensemble. Le HUD la reprendra en phase 3 |
| Le replayer rejoue toutes les mains des fixtures | ✅ 496 mains : à chaque étape les stacks restent ≥ 0, le pot plus les mises égale le pot total, et les stacks finaux sont justes. Test : `replay.test.ts` |
| Vérifié dans la vraie app (Tauri, Linux) | ✅ vue d'ensemble, graphique et replayer sur une base réelle |

Livré : moteur de stats (langage de formules → SQL avec paramètres liés), 13 stats expliquées en
français, vue d'ensemble centrée sur le résultat, rapports par position / format / mois / limite / compte,
graphique en bb (résultat, abattage, hors abattage) ou en € (cash, et tournois tournoi par tournoi),
liste des tournois avec ROI et ITM, liste des mains, replayer, éditeur de stats avec aperçu en direct.

Couleurs du graphique : emplacements 1 à 3 de la palette de référence, validés avec `validate_palette.js`
sur nos surfaces. En clair, l'aqua est sous 3:1 de contraste : les valeurs finales sont donc écrites dans la légende.

Reste pour la phase 2 :
- filtres avancés (cartes, texture du board, séquence d'actions, profondeur, type de pot) ;
- constructeur de rapports, grille 13×13 par main, export CSV ;
- notes manuelles et auto-notes, détecteur de fuites ;
- bankroll et sessions, analyse de population.
