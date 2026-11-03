# Évaluateur de mains (`crates/evaluator`)

## Cartes et mains
- Une carte est un nombre `0..52` : `rang × 4 + couleur`. Le rang `0` est le deux et `12` l'as. Les couleurs
  sont dans l'ordre `c d h s` (trèfle, carreau, cœur, pique).
- Une main (un ensemble de cartes) est un masque de 64 bits : 4 tranches de 16 bits, une par couleur. Le bit
  `rang` de la tranche `couleur` vaut 1 si la carte est là. Ajouter le board à une main se fait avec un
  simple OU.

## Algorithme
On lit les 4 masques de couleur `a, b, c, d` (13 bits chacun).
1. **Couleur** : si une couleur a au moins 5 bits, c'est une couleur ou une quinte flush. Avec 7 cartes au
   plus, une couleur exclut le carré et le full (il faudrait au moins 8 cartes), donc c'est la réponse.
2. Sinon, on combine les rangs avec des opérations sur les bits :
   - `any = a|b|c|d` : les rangs présents ;
   - `two = (a&b)|(c&d)|((a|b)&(c|d))` : les rangs présents dans au moins 2 couleurs ;
   - `three = (a&b&(c|d))|(c&d&(a|b))` : les rangs présents dans au moins 3 couleurs ;
   - `four = a&b&c&d` : les carrés.
3. **Quinte** : une table précalculée de 8 192 cases (`STRAIGHT[masque de rangs]`) donne la plus haute carte
   de la meilleure quinte, ou 0. La roue A-2-3-4-5 a le 5 comme carte haute.
4. « Les N meilleurs rangs d'un masque » : on efface les bits les plus bas jusqu'à en garder N.

## Valeur
`valeur = catégorie << 26 | majeur << 13 | mineur`. `majeur` et `mineur` sont des masques de rangs, par
exemple pour un full : `majeur` est le rang du brelan et `mineur` celui de la paire. Deux masques qui ont le
même nombre de bits se comparent comme les listes de rangs triées qu'ils représentent. Donc **la plus grande
valeur gagne**, et deux valeurs égales se partagent le pot.

## Vérification
- On compte exhaustivement les 2 598 960 mains de 5 cartes et les 133 784 560 mains de 7 cartes. Pour
  chaque catégorie, le nombre trouvé est celui qu'on calcule par la combinatoire (par ex. 4 047 644
  couleurs sur 7 cartes).
- Sur 20 000 paires de mains tirées au hasard, il désigne le même gagnant qu'eval7
  (`tools/reference_evaluator.py`).
- Vitesse : environ 65 millions de mains de 7 cartes par seconde et par cœur.
