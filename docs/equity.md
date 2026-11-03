# Equity Lab (`crates/equity`)

## Ranges
Une range donne à chacun des 1 326 combos un poids de 0 à 1. La notation est la même côté Rust
(`src/range.rs`) et côté interface (`src/lib/range.ts`). Les deux sont testés sur les mêmes cas
(`data/range_cases.json`) :

| Écriture | Signification |
|---|---|
| `AA`, `AKs`, `AKo`, `AK` | une paire, un assorti, un dépareillé, les deux à la fois |
| `TT+` | de TT à AA |
| `ATs+`, `ATo+`, `AT+` | le kicker monte jusqu'à une carte sous la première (AT à AK) |
| `99-66`, `A5s-A2s` | un intervalle avec la même première carte |
| `AhKh` | un seul combo |
| `random` ou `any` | les 1 326 combos |
| `15%` | le top 15 % des mains (classement ci-dessous) |
| `KQo:50%`, `KQo:50`, `KQo:0.5` | un poids. Une valeur ≤ 1 est une fraction, sinon des pourcents |

Quand un combo apparaît deux fois, c'est la dernière écriture qui compte (`AK, AKo:0` = `AKs`).

## Calcul d'équité
- Une **donne**, c'est un combo par joueur, sans carte commune, plus les cartes de board qui manquent. Une
  donne pèse le produit des poids de ses combos, et tous les boards possibles sont équiprobables.
- L'équité d'un joueur est sa part moyenne du pot : 1 s'il gagne seul, 1/k quand k joueurs partagent.
- **Exhaustif** : quand `nombre de donnes × joueurs` reste sous 120 millions, on visite toutes les donnes et
  le résultat est exact. Le travail est réparti entre les cœurs selon le combo du joueur 1 et la première
  carte du board.
- **Monte-Carlo** : sinon, on tire chaque combo selon son poids. Si deux joueurs ont une carte en commun, on
  retire toute la donne (échantillonnage par rejet, qui donne exactement la pondération ci-dessus), puis on
  complète le board au hasard. On calcule par lots sur tous les cœurs et on s'arrête quand la
  demi-largeur de l'intervalle de confiance à 95 % (1,96 × erreur type) passe sous la précision demandée
  pour **chaque** joueur, ou quand la limite de temps est atteinte.
- La **distribution d'équité** donne l'équité de chaque combo d'une range contre les autres joueurs. On la
  trace en classant les combos du meilleur au moins bon.

### Référence (critère de la phase 4)
`tools/reference_equity.py` calcule 20 cas avec **eval7**, une bibliothèque open source. Chaque cas est
énuméré entièrement, donc la valeur est exacte, et les ranges sont lues par le parseur d'eval7. Les cas :
main contre main préflop, à 3 et 4 joueurs, au flop et à la turn, main contre range, ranges pondérées, range
contre random, 3 ranges. `cargo test -p lps-equity --test reference` les compare à notre moteur : écart
maximal de **0,033 %** (18 cas sont exhaustifs avec un écart de 0, 2 passent en Monte-Carlo).

### Performances (4 cœurs, release)
| Cas | Objectif | Mesuré |
|---|---|---|
| AA contre KK préflop, exhaustif | < 50 ms | 19 ms |
| top 15 % contre top 30 % préflop à ±0,1 % | < 1 s sur 8 cœurs | 41 ms |

`cargo run --release -p lps-equity --example bench` refait la mesure.

## Classement préflop (top X %)
C'est l'équité de chaque classe contre une main au hasard (Monte-Carlo à ±0,02 %). Elle est calculée par
notre moteur (`examples/gen_ranking.rs`) et figée dans `data/preflop_ranking.json`. Exemples : AA 85,19 %,
32o 32,29 %. « Top X % » prend des classes entières dans cet ordre et s'arrête à la limite de classe la
plus proche de X % des 1 326 combos.

## Décomposition sur un board (façon Flopzilla)
Chaque combo vivant a **une seule** catégorie de main faite, dans cet ordre de priorité :
1. quinte flush, carré, full, couleur, quinte : données par l'évaluateur, y compris quand le board fait la
   main tout seul ;
2. **set** (paire servie qui touche le board) ; **brelan** (tous les autres brelans) ;
3. les paires ne comptent que si une **carte privative** les fait (une paire du board seule ne compte pas) :
   - deux paires : les deux cartes privatives touchent deux rangs différents du board ;
   - overpair : paire servie plus haute que la plus haute carte du board ;
   - top paire : une carte privative touche le rang le plus haut du board ;
   - paire servie sous la top paire : entre les deux rangs les plus hauts du board ;
   - paire du milieu : une carte privative touche le 2ᵉ rang du board ;
   - petite paire : une carte privative touche un rang plus bas, ou une paire servie sous le 2ᵉ rang ;
4. hauteur As : aucune paire grâce aux cartes privatives, mais un as en main ;
5. rien.

Les **tirages** (au flop et à la turn seulement) se cumulent et demandent tous au moins une carte
privative :
- tirage couleur : 4 cartes d'une couleur, sans couleur faite ;
- quinte bilatérale : au moins 2 rangs complètent une quinte (les doubles ventrales comptent ici). Ventrale :
  exactement 1 rang. Un rang ne compte que s'il donne une meilleure quinte que le board plus cette carte ;
- backdoor couleur (flop) : 3 cartes d'une couleur ;
- backdoor quinte (flop) : aucun tirage quinte, mais deux rangs de plus feraient une quinte ;
- deux overcards : aucune paire grâce aux cartes privatives, et les deux cartes au-dessus du board.

Les nombres de combos sont exacts, poids compris. Les tests comparent des cas comptés à la main et vérifient
que les catégories de mains faites découpent toujours toute la range.

**Garder la sélection** ne garde que les combos des lignes cochées, avec leurs poids. On ajoute ensuite la
turn (puis la river) pour la street suivante.

## Textures de flop
Les 22 100 flops se regroupent en 1 755 classes qui ne diffèrent que par un échange des couleurs. Une range
peut viser des couleurs précises (`AhKh`) : on calcule donc la décomposition sur **chacun** des 22 100 flops,
puis on additionne par classe. Le résultat est exact pour toute range, et chaque flop réel compte
pareil. Filtres : couleurs (monocolore, deux couleurs, arc-en-ciel), paires, carte haute, connecté (trois
rangs différents qui tiennent dans une fenêtre de quinte).

## Fair-play
Equity Lab est un module d'étude. Il est verrouillé dès qu'une table est ouverte, et chaque commande refuse
de tourner en mode jeu. Rien n'apparaît dans le HUD.
