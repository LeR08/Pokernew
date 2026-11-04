# Entraîneur et analyse (`crates/trainer`)

## Note d'une réponse
- Pour ta main au nœud, on lit l'EV de chaque action dans la solution (`expected_values_detail` du moteur).
- **Perte** = EV de la meilleure action − EV de l'action choisie, en bb et en % du pot au moment de la
  décision.
- Notes, selon la perte en % du pot (réglable) :
  - meilleur coup : l'action de plus grande EV (ou moins de 0,05 %) ;
  - correct : jusqu'à 1 % ;
  - imprécision : jusqu'à 3 % ;
  - erreur : jusqu'à 8 % ;
  - grosse erreur : au-delà.
- Préflop (charts) : pas d'EV dans un chart, donc on note selon la fréquence de l'action choisie. La plus
  fréquente est le meilleur coup ; au moins 25 % est correct ; au moins 5 % est une imprécision ; moins,
  c'est une erreur ; 0 %, une grosse erreur.

## Tirage des questions
- **Un nœud** : la première décision d'OOP, ou celle d'IP après un check.
- **Une street** : on joue depuis la racine, avec les fréquences globales des deux ranges, et on s'arrête à
  une décision au hasard.
- **Main complète** : après ta réponse, l'adversaire joue selon les fréquences de sa range, puis les cartes
  tombent au hasard (sans tes cartes), jusqu'à ta prochaine décision.
- La main du héros est tirée selon ses **poids à ce nœud** (combien de fois elle arrive là).
- **Points faibles** : on tire 6 questions et on garde celle dont le type de spot (street · position ·
  premier à parler ou face à une mise) a la plus grosse perte moyenne dans l'historique.

## Analyse d'une main
1. Elle exige un pot en tête-à-tête au flop avec le héros. Le pot et le tapis effectif sont mesurés au flop.
   En position : celui qui parle en dernier après le flop (en tête-à-tête, la SB / bouton).
2. **Ranges** (charts du format choisi) :
   - pot simplement relancé : `<ouvreur>/open` contre `<caller> vs <ouvreur>/call` ;
   - pot 3-betté : `<3-bettor> vs <ouvreur>/3bet` contre `<ouvreur> vs 3bet/call`.

   Les positions à 9 joueurs sont ramenées à celles du 6-max : UTG/UTG1/UTG2/MP → UTG, LJ/HJ → HJ.
3. Le spot est résolu avec l'arbre et la cible choisis, ou retrouvé dans le cache s'il existe déjà.
4. **La ligne réelle** :
   - fold, check et call sont repris tels quels ;
   - une mise ou une relance devient la taille la plus proche de l'arbre (le tapis si le joueur était à
     tapis), avec une alerte si l'écart dépasse 30 % du pot ;
   - les vraies turn et river sont données.

   À chaque décision du héros, on calcule la perte pour sa vraie main.
5. Les fuites sont regroupées par type de spot : street · PFR ou caller · IP ou OOP · premier à parler ou
   face à une mise.
