# Local Poker Suite

**FR** · [EN](#english)

Suite poker 100 % locale pour un seul joueur sur un seul PC. Elle remplace un tracker et un HUD (PokerTracker 4,
Hand2Note), un solveur postflop (PioSOLVER, GTO+), un outil d'équité (Equilab, Flopzilla) et un outil ICM (ICMIZER,
HRC). Aucun compte, aucune télémétrie, aucun appel réseau.

> Version **1.0.0**. Les 10 phases de la feuille de route sont faites, voir `docs/phase-*.md` et [`CHANGELOG.md`](CHANGELOG.md).

## Les modules

| Module | Ce qu'il fait |
|---|---|
| **Import & base** | Suit les dossiers d'historiques Winamax, importe en continu (sans doublon), lit les résumés de tournoi, journal d'erreurs, sauvegardes |
| **Tracker** | Résultats, sessions et bilan de session, rapports par position / format / mois, graphique, tournois (ROI, ITM), grille 13×13, fuites, adversaires, éditeur de stats |
| **Replayer & notes** | Rejoue chaque main, notes et auto-notes sur les joueurs, liens vers Equity Lab et la revue ICM |
| **HUD** | Panneaux transparents sur les tables Winamax, statistiques historiques seulement, dispositions réglables table par table |
| **Bankroll & sessions** | Suivi de bankroll, dépôts et retraits |
| **Equity Lab** | Équité de 2 à 10 joueurs, éditeur de range, analyse d'une range sur un board, textures de flop |
| **ICM & push/fold** | ICM, FGS, KO ; solveur push/fold Nash de 2 à 10 joueurs ; revue ICM de tes mains de tournoi ; quiz |
| **Solveur postflop** | Discounted CFR (moteur postflop-solver intégré), arbre de mises, visionneuse, verrouillage de nœuds, cache, batch |
| **Ranges & solveur préflop** | Charts par format (9 formats prêts à l'emploi), import / export, solveur préflop CFR+ |
| **Entraîneur & analyse** | Exercices postflop, préflop et push/fold, analyse de tes mains contre le solveur, historique |

## Fair-play (règles fixes)

C'est un outil d'**étude et de revue** :
- dès qu'une table est ouverte (et 60 s après), Equity Lab, ICM, solveurs, préflop et entraîneur sont
  **verrouillés**, et leurs commandes refusent de tourner ;
- le HUD n'affiche **que des statistiques historiques** : jamais d'équité, de solution ni de conseil sur la main en
  cours ;
- pas de lecture d'écran (OCR), pas d'automatisation ;
- le HUD peut être coupé salle par salle (`hud_allowed`).

## Installer

Chaque push construit l'app sur GitHub Actions :
1. ouvre l'onglet *Actions* ;
2. choisis le dernier run vert ;
3. télécharge l'artefact **windows**.

Il contient :
- `Local Poker Suite_1.0.0_x64-setup.exe` (installeur) et le `.msi` ;
- `LocalPokerSuite-portable.zip` : à dézipper n'importe où. Les données restent dans `data\` à côté de
  l'exécutable.

SmartScreen peut avertir, car l'app n'est pas signée : choisis *Informations complémentaires*, puis *Exécuter quand
même*. WebView2 est déjà présent sur Windows 10/11 à jour.

Au premier lancement, l'assistant trouve tes dossiers d'historiques Winamax
(`%APPDATA%\winamax\documents\accounts\<pseudo>\history`) et lance l'import.

## Compiler sous Windows 10/11

À installer une seule fois :
1. **Visual Studio Build Tools 2022**, avec « Développement Desktop en C++ » ;
2. **Rust** via <https://rustup.rs> (`stable-x86_64-pc-windows-msvc`) ;
3. **Node.js 22**, puis `corepack enable` (pour pnpm).

```powershell
pnpm install
```

```powershell
pnpm tauri dev
```

```powershell
pnpm tauri build
```

`pnpm tauri build` produit l'installeur et le `.msi` dans `target\release\bundle\`.

**Mode portable** : un fichier vide `portable.flag` à côté de l'exécutable. Autres façons de choisir le dossier de
données : `--data-dir D:\Poker\LPS`, ou la variable `LPS_DATA_DIR`.

## Ton dossier de données

Par défaut, `%APPDATA%\LocalPokerSuite`. Tout est en fichiers lisibles, rechargés à chaud quand tu les modifies :

| Fichier / dossier | Contenu |
|---|---|
| `settings.toml`, `hotkeys.toml` | réglages, raccourcis |
| `themes/`, `profiles/` | thèmes, profils |
| `rooms/*.toml` | salles : dossiers surveillés, pseudos, HUD autorisé |
| `stats/`, `leaks/`, `notes/`, `hud/` | stats (formules), repères de fuites, règles d'auto-notes, dispositions du HUD |
| `payouts/*.toml` | structures de gains de tes tournois (saisies dans le module ICM) |
| `solver_trees/*.toml` | arbres de mises du solveur |
| `ranges/**/*.txt` | bibliothèque de ranges et charts préflop |
| `database.sqlite`, `backups/` | base de mains (SQLite) et ses sauvegardes |
| `solver-cache/` | solutions du solveur (compressées) |
| `trainer/history.jsonl` | historique de l'entraîneur |
| `exports/`, `logs/` | exports CSV / JSON, journaux |
| `schemas/` | schémas JSON des fichiers de configuration (régénérés à chaque lancement) |

Un fichier invalide est ignoré : la dernière version valide reste active et l'erreur s'affiche. Les fichiers par
défaut ne remplacent jamais les tiens.

## Salles

| Salle | Formats |
|---|---|
| Winamax | MTT, KO, satellites, Expresso, cash (6-max, 5-max, 3-max), résumés de tournoi |

Un lecteur d'historiques n'est écrit qu'à partir de **vrais fichiers** de la salle, jamais en devinant le format.
Pour une autre salle, envoie quelques fichiers d'historique et de résumé. Une salle correspond à un fichier dans
`crates/hh-parsers/src/` (voir [`docs/phase-9.md`](docs/phase-9.md)).

## Développement

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
pnpm test
```

```bash
pnpm e2e
```

- Le front tourne aussi dans un navigateur (`pnpm dev`, <http://localhost:1420>), avec un backend simulé.
- Structure :
  - `crates/` : cœur Rust, sans Tauri ;
  - `crates/solver/engine` : le moteur postflop-solver, notre fork ;
  - `app/src-tauri/` : le seul code qui connaît Tauri ;
  - `app/src/modules/` : un dossier par module ;
  - `config/defaults/` : fichiers par défaut et schémas.
- Les maths sont expliquées dans `docs/` : évaluateur, équité, ICM, solveur, préflop, entraîneur.

## Licence

AGPL-3.0-or-later. Le moteur de solveur postflop est **postflop-solver** de Wataru Inariba (AGPL-3.0). Voir
[`THIRD_PARTY.md`](THIRD_PARTY.md).

---

## English

100 % local poker suite for one player on one PC: tracker + HUD, postflop solver, equity tools, ICM / push-fold,
preflop charts and solver, trainer and hand analysis. No account, no telemetry, no network calls.

> Version **1.0.0**: the 10 roadmap phases are done, see `docs/phase-*.md` and [`CHANGELOG.md`](CHANGELOG.md).

### Modules

Import & database · Tracker · Replayer & notes · HUD · Bankroll & sessions · Equity Lab · ICM & push/fold · Postflop
solver · Preflop ranges & solver · Trainer & analysis. They are described in the French table above.

### Fair play (fixed rules)

While a poker table is open (and for 60 s after), every study module is locked and its commands refuse to run.
The HUD only shows historical statistics. There is no screen reading and no automation, and the HUD can be
switched off per room.

### Install

GitHub Actions builds the app on every push. Open *Actions*, pick the latest green run and download the
**windows** artifact. It contains the NSIS installer, the `.msi` and `LocalPokerSuite-portable.zip` (unzip it
anywhere; its data stays in `data\` next to the exe). The app is not signed: on the SmartScreen warning,
choose *More info* → *Run anyway*.

### Build on Windows 10/11

You need Visual Studio Build Tools 2022 (Desktop C++), Rust (`stable-x86_64-pc-windows-msvc`), Node.js 22 and
pnpm (`corepack enable`).

```powershell
pnpm install
```

```powershell
pnpm tauri dev
```

```powershell
pnpm tauri build
```

For portable mode, put an empty `portable.flag` file next to the exe. You can also use `--data-dir <path>` or
`LPS_DATA_DIR`.

### Data folder

By default `%APPDATA%\LocalPokerSuite`. Everything in it is a readable file (TOML, text ranges, SQLite) and is
hot-reloaded when you edit it. Defaults never overwrite your files. See the French table above for the full list.

### Rooms

Winamax: MTT, KO, satellites, Expresso, cash, tournament summaries. A parser is written only from real files of
the room, never by guessing the format.

### Development

Run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `pnpm test` and
`pnpm e2e`. The front end also runs in a browser with a mock backend (`pnpm dev`). The maths are documented
in `docs/`.

### License

AGPL-3.0-or-later. The postflop solver engine is **postflop-solver** by Wataru Inariba (AGPL-3.0); see
[`THIRD_PARTY.md`](THIRD_PARTY.md).
