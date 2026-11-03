# Phase 2 — Tracker (stats, rapports, graphiques, replayer)

Statut : **validée par l'utilisateur le 3 nov. 2026** · Choix de l'utilisateur (3 nov. 2026) :
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

## Résultats — deuxième partie (3 nov. 2026)

Demande de l'utilisateur : « sur les mains jouées, me conseiller sur les erreurs de session et les bons coups ».

| Fonction | Détail |
|---|---|
| **Sessions** | Regroupées automatiquement : un trou de plus de 30 min entre deux mains ouvre une nouvelle session. Résultat, durée, mains et tournois |
| **Bilan de session** | 7 règles « à revoir » et 6 « bons coups » (liste et seuils dans `crates/stats-engine/src/review.rs`), chacune expliquée en clair avec un lien vers le replayer. VPIP/PFR de la session comparés à ton habitude, avec une alerte de tilt quand le VPIP dépasse ton habitude de 10 points ou plus sur au moins 50 mains |
| **Détecteur de fuites** | Repères par format dans `leaks/targets.toml`, modifiables. Intervalle de Wilson à 95 % : « Fuite » si tout l'intervalle sort du repère, « À surveiller » sinon. Pour les stats de décision : bb/100 quand tu le fais vs quand tu ne le fais pas |
| **Grille 13×13** | bb/100, résultat ou nombre de mains par main de départ ; échelle divergente rouge ↔ gris ↔ bleu de la palette de référence |
| **Bankroll** | Dépôts, retraits, bonus et ajustements ; courbe (mouvements + cash + tournois) ; règle des 30 buy-ins moyens |
| **Notes** | Notes et couleur par joueur, depuis le replayer (en clic sur un siège) ; le HUD les affichera en phase 3 |
| **Export CSV** | Rapports et mains, séparateur `;` et BOM UTF-8 pour ouvrir directement dans Excel en français |

Fair-play : le bilan ne porte que sur des mains **terminées**. Comme les autres modules d'étude, il sera
verrouillé pendant le jeu dès que la phase 3 saura détecter les tables ouvertes.

Les règles sont des règles de poker classiques (force des mains de départ : formule de Chen), pas un solveur.
L'analyse par solveur, avec la perte d'EV main par main, arrive en phases 6 et 8.

Tests : 107 Rust (règles vérifiées sur des mains réelles lues à la main, sessions, Wilson, grille, bankroll),
23 Vitest, 19 Playwright. Vérifié dans la vraie app : sessions, bilan et fuites sur une base réelle.

## Résultats — troisième partie (3 nov. 2026)

| Fonction | Détail |
|---|---|
| **Filtres avancés** | Cartes (classes « AKs, QQ » ou groupes : paires, assorties, broadways, As-x, connecteurs assortis), type de pot (limp, 1 relance, 3-bet, 4-bet+), tapis min/max en bb, rue atteinte, texture du flop (pairé, monocolore, deux couleurs, arc-en-ciel, connecté, As/Roi, 9 ou moins), abattage, tapis préflop, buy-in du tournoi. Données dérivées calculées à l'import (migration v5, recalculées automatiquement pour les mains déjà en base) |
| **Population** | Filtre « Moi / tous les adversaires » sur tous les rapports ; onglet **Adversaires** avec l'écart à la population en points |
| **Auto-notes** | Règles dans `notes/auto.toml`, écrites dans le langage des stats (`vpip >= 0.40 and vpip - pfr >= 0.20`), avec un échantillon minimum ; recalculées après les imports (au plus une fois par minute) ou à la demande ; les notes manuelles ne sont jamais touchées |

Vérifié dans la vraie app : la migration recalcule les 496 mains existantes ; les auto-notes donnent 6 notes
cohérentes avec les stats affichées (ex. Player11 : VPIP 60 %, PFR 19 % → « joue beaucoup de mains en suivant »).
Tests : 113 Rust, 23 Vitest, 20 Playwright.

## Bilan de la phase 2

Les 4 critères d'acceptation de la spec sont remplis. Le dernier, « la stat apparaît dans le HUD », sera vérifié
quand le HUD existera, en phase 3.

Gardé pour plus tard, parce que ça dépend d'autres phases ou que c'est peu utile tant que tu joues surtout en MTT :
- filtre par séquence d'actions (« BTN ouvre, BB 3-bet, je suis ») et par étape ICM (bulle, ITM, table finale) :
  en phase 5, avec l'ICM ;
- constructeur de rapports par glisser-déposer : les rapports sont déjà des regroupements combinés aux filtres et aux
  stats personnalisées ;
- courbe « EV all-in » : en phase 4, avec l'évaluateur d'équité ;
- journal de session (humeur, concentration) : à ajouter si tu le veux.
