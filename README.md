# Local Poker Suite

**FR** · [EN](#english)

Suite poker 100 % locale (tracker + HUD, solveur, équité, ICM) pour un seul joueur sur un seul PC.
Aucun compte, aucune télémétrie, aucun appel réseau.

> État : **phase 4 (Equity Lab)** : import Winamax, tracker, replayer, HUD, équité et analyse de ranges. Voir `docs/phase-*.md` et [`CHANGELOG.md`](CHANGELOG.md).

## Installer sans compiler

Chaque push construit l'installeur sur GitHub Actions : ouvre l'onglet *Actions*, choisis le dernier run vert, puis télécharge l'artefact **windows**. Il contient l'installeur `…_x64-setup.exe`, le `.msi` et `LocalPokerSuite-portable.zip`. SmartScreen peut avertir, car l'app n'est pas signée : choisis *Informations complémentaires*, puis *Exécuter quand même*.

## Compiler et lancer sous Windows 10/11

Prérequis, à installer une seule fois :

1. **Visual Studio Build Tools 2022** avec la charge de travail « Développement Desktop en C++ ».
2. **Rust** via <https://rustup.rs> (toolchain `stable-x86_64-pc-windows-msvc`).
3. **Node.js 22** puis **pnpm** :
   ```powershell
   corepack enable
   ```
4. **WebView2** est déjà présent sur Windows 10/11 à jour.

Dans le dossier du dépôt :

```powershell
pnpm install
```

Mode développement (rechargement du front à chaud) :

```powershell
pnpm tauri dev
```

Installeur `.exe` (NSIS) et `.msi`, générés dans `target\release\bundle\` :

```powershell
pnpm tauri build
```

### Mode portable

Place un fichier vide `portable.flag` à côté de `local-poker-suite.exe`. Les données vont alors dans
`data\` à côté de l'exécutable au lieu de `%APPDATA%\LocalPokerSuite`. La CI produit
directement `LocalPokerSuite-portable.zip`.

Autres façons de choisir le dossier de données :

- `local-poker-suite.exe --data-dir D:\Poker\LPS`
- la variable d'environnement `LPS_DATA_DIR`

## Configuration

Tout se trouve dans le dossier de données, en TOML commenté :

| Fichier | Contenu |
|---|---|
| `settings.toml` | langue, profil actif, thème, densité, taille du texte, ordre et visibilité des modules, performances |
| `hotkeys.toml` | raccourcis de l'application |
| `themes/*.toml` | thèmes (tokens de couleur) |
| `profiles/*.toml` | profils (HUD + filtres + thème) |
| `rooms/*.toml` | salles : dossiers d'historiques surveillés, pseudos, HUD autorisé |
| `database.sqlite` | la base de mains (SQLite, mode WAL) |
| `backups/` | sauvegardes automatiques de la base |
| `schemas/*.schema.json` | schémas de validation (régénérés à chaque lancement) |
| `logs/startup.log` | temps de démarrage mesurés |

- Les fichiers sont **rechargés à chaud** : édite-les dans n'importe quel éditeur et l'application se met à jour.
- Un fichier invalide est ignoré : la dernière version valide reste active et l'erreur s'affiche.
- Le centre de réglages propose un formulaire (généré depuis le schéma) et un éditeur texte brut. Les commentaires sont conservés quand tu enregistres depuis le formulaire.

## Salles supportées

| Salle | Formats | État |
|---|---|---|
| Winamax | MTT, knockout, satellites, Expresso, cash (6-max, 5-max, 3-max) + résumés de tournoi | ✅ |
| Winamax | Go Fast | ⏳ en attente de fichiers d'exemple |
| Autres salles | — | phase 9, une salle = un fichier dans `crates/hh-parsers/src/` |

Un parseur n'est écrit qu'à partir de vrais fichiers : une ligne inconnue apparaît dans
le journal d'erreurs du module Import, avec un bouton « copier comme cas de test ».

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

Le front tourne aussi dans un navigateur (`pnpm dev`, <http://localhost:1420>) avec un backend simulé, pratique pour l'UI.

Structure : `crates/` (cœur Rust, sans Tauri), `app/src-tauri/` (seul code qui connaît Tauri),
`app/src/` (React, un dossier par module dans `app/src/modules/`), `config/defaults/` (fichiers par défaut + schémas).

Ajouter un module : créer `app/src/modules/<id>/manifest.ts`, puis ajouter les clés i18n `modules.<id>.*`.
Le launcher, la barre latérale et les routes le prennent en compte automatiquement.

## Equity Lab

Calculateur d'équité (2 à 10 joueurs, mains ou ranges, exhaustif ou Monte-Carlo), éditeur de range partagé
(grille, poids, couleurs, texte, top X %), analyse d'une range sur un board, textures de flop et
bibliothèque de ranges dans `<données>/ranges/**.txt`. Maths et définitions : [`docs/equity.md`](docs/equity.md),
[`docs/evaluator.md`](docs/evaluator.md).

```bash
cargo run --release -p lps-equity --example bench
```

## Fair-play

C'est un outil d'**étude et de revue**. Aucun module ne donne de conseil en temps réel pendant le jeu.
Les modules d'étude (Equity Lab, ICM, solveur, préflop, entraîneur) sont verrouillés tant qu'une table est ouverte, et 60 s après.
Le HUD n'affiche que des statistiques historiques. Pas d'OCR, pas d'automatisation.

## Licence

AGPL-3.0-or-later. Le solveur s'appuiera sur `postflop-solver`, qui est sous AGPL.

---

## English

100 % local poker suite (tracker + HUD, solver, equity, ICM) for one player on one PC.
No account, no telemetry, no network calls.

> Status: **phase 4 (Equity Lab)**: Winamax import, tracker, replayer, HUD, equity and range analysis. See `docs/phase-*.md` and [`CHANGELOG.md`](CHANGELOG.md).

### Build and run on Windows 10/11

Prerequisites, installed once:

1. **Visual Studio Build Tools 2022** with "Desktop development with C++".
2. **Rust** from <https://rustup.rs> (`stable-x86_64-pc-windows-msvc`).
3. **Node.js 22**, then enable **pnpm**:
   ```powershell
   corepack enable
   ```
4. **WebView2** ships with up-to-date Windows 10/11.

In the repository folder:

```powershell
pnpm install
```

Development mode:

```powershell
pnpm tauri dev
```

Installer `.exe` (NSIS) and `.msi`, written to `target\release\bundle\`:

```powershell
pnpm tauri build
```

**Portable mode:** put an empty `portable.flag` file next to `local-poker-suite.exe`. Data then lives in `data\`
next to the executable instead of `%APPDATA%\LocalPokerSuite`. You can also use `--data-dir <path>` or the
`LPS_DATA_DIR` environment variable.

### Configuration

Everything is commented TOML in the data folder: `settings.toml`, `hotkeys.toml`, `themes/*.toml` and
`profiles/*.toml`, with JSON Schemas in `schemas/`.

- Files are **hot-reloaded**.
- An invalid file is ignored: the last valid version stays active and the error is shown.
- The settings centre offers a schema-generated form and a raw text editor. Comments are kept when you save from the form.

### Development

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

The front end also runs in a browser (`pnpm dev`) with a mock backend.

To add a module, create `app/src/modules/<id>/manifest.ts` and the `modules.<id>.*` i18n keys. The shell picks it up automatically.

### Fair play

This is a **study and review** tool. No module gives live advice during play.
Study modules (Equity Lab, ICM, solver, preflop, trainer) are locked while a table is open, and for 60 s after. The HUD only shows historical stats.

### License

AGPL-3.0-or-later.
