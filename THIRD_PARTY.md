# Composants tiers / Third-party components

## Inclus dans l'application / Shipped in the app
- **postflop-solver** (Wataru Inariba), AGPL-3.0-or-later : moteur du solveur postflop, intégré et maintenu dans
  `crates/solver/engine`. Les modifications sont listées dans `crates/solver/engine/FORK.md`.
  <https://github.com/b-inary/postflop-solver>
- **Tauri**, **React**, **ECharts**, **CodeMirror**, **rusqlite / SQLite**, **rayon**, **serde**, **zstd** et les
  autres bibliothèques déclarées dans `Cargo.toml`, `app/package.json` et leurs fichiers de verrouillage, chacune
  sous sa propre licence (MIT, Apache-2.0, BSD, domaine public pour SQLite).

## Utilisés seulement pour vérifier les résultats (non inclus) / Used only to check results (not shipped)
- **eval7** (open source) : calcul des 20 équités de référence d'Equity Lab et vérification de l'évaluateur
  (`tools/reference_equity.py`, `tools/reference_evaluator.py`).
- **Tableaux Nash tête-à-tête de HoldemResources.net**, publiés en 2007
  (<https://www.holdemresources.net/hune>) : données du test d'acceptation du solveur push/fold
  (`crates/icm/tests/hune_hu_noante.txt`).
- **TexasSolver** (AGPL) : référence prévue pour le solveur postflop. Les spots sont dans
  `crates/solver/tests/texassolver/`.
