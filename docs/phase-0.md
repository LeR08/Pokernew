# Phase 0 — Shell (launcher, modules, configuration)

Statut : **validée par l'utilisateur le 3 nov. 2026** (installeur CI testé sur son PC Windows) · Spec : *Local Poker Suite — Specification* (3 nov. 2026)

## Critères d'acceptation (rappel de la spec)

- L'application démarre en < 2 s.
- Tuiles et sidebar construites à partir des manifests de modules.
- Formulaires de réglages générés à partir des JSON Schemas.
- Éditer un fichier TOML le recharge à chaud (sans redémarrage).
- Le mode portable fonctionne.
- `cargo test`, `cargo clippy -D warnings`, `pnpm test` au vert.

## Tâches

### 1. Squelette du dépôt
- Workspace Cargo racine (`Cargo.toml`) avec les crates de la spec créées vides
  (`core-model`, `hh-parsers`, `db`, `stats-engine`, `evaluator`, `equity`, `icm`,
  `solver`, `preflop`, `hud-overlay`) : `lib.rs` + un test trivial, pour figer la structure.
- Nouvelle crate **`config`** (non listée dans la spec, à valider) : dossier de données,
  chargement/validation TOML, watcher. Elle reste sans dépendance Tauri pour être testable seule.
- `app/` : projet Tauri 2 (`src-tauri/`) + front React 18 / TS / Vite / Tailwind / shadcn/ui,
  géré par pnpm (`pnpm-workspace.yaml` à la racine).
- `rust-toolchain.toml`, `.editorconfig`, `.gitignore`, `CHANGELOG.md`, `README.md`
  (build Windows), `LICENSE` = AGPL-3.0 (imposée plus tard par le solveur vendoré).

### 2. Crate `config` (Rust)
- **Résolution du dossier de données** : `%APPDATA%/LocalPokerSuite/` par défaut ;
  surcharge via `--data-dir` ou variable `LPS_DATA_DIR` ; **mode portable** si un fichier
  `portable.flag` (ou un dossier `data/`) est à côté de l'exécutable → données dans `<exe>/data/`.
- **Premier lancement** : copie des défauts de `config/defaults/` (embarqués dans le binaire
  via `include_dir`) vers le dossier de données, sans jamais écraser un fichier existant.
- **Chargement** : `settings.toml`, `hotkeys.toml`, `themes/*.toml`, `profiles/*.toml`
  (les autres familles — stats, hud, reports, filters, rooms — arrivent avec leurs phases ;
  le loader est générique : `famille → schéma → fichiers`).
- **Validation** par JSON Schema (crate `jsonschema`) après conversion TOML → JSON ;
  erreurs avec chemin de fichier + chemin de clé.
- **Hot reload** : `notify` + debounce 200 ms ; fichier valide → nouvelle version en mémoire
  + événement `config_changed { family, id }` ; fichier invalide → on garde l'ancienne
  version et on émet `config_error`.
- Écriture atomique (fichier temporaire + rename) quand l'UI sauvegarde, en conservant
  les commentaires quand c'est possible (`toml_edit`).

### 3. `config/defaults/`
- `settings.toml` (langue, thème, densité, échelle de police, ordre/activation des modules,
  dossier de données, sauvegardes) — commenté FR/EN.
- `hotkeys.toml`, `themes/dark.toml`, `themes/light.toml`, `profiles/cash-6max.toml`,
  `profiles/mtt.toml`, `profiles/spin.toml` (squelettes : thème seulement pour l'instant).
- `schemas/*.schema.json` pour chacun, avec `title`/`description` traduisibles
  (clés i18n) utilisées par le générateur de formulaires.

### 4. `src-tauri` (seul endroit qui connaît Tauri)
- Commandes : `config_get(family, id)`, `config_list(family)`, `config_save(family, id, toml)`,
  `config_validate(family, toml)`, `schema_get(family)`, `app_paths()`, `open_module_window(id)`.
- **Bus d'événements interne** : `tokio::sync::broadcast` côté Rust, relayé au front via
  `app.emit(...)` ; premiers événements : `config_changed`, `config_error`.
- Fenêtres : fenêtre principale + ouverture d'un module dans sa propre fenêtre
  (`WebviewWindowBuilder`, route `#/module/<id>`).
- Aucun appel réseau (pas de plugin updater en phase 0 ; CSP stricte, `connect-src` local).

### 5. Front React
- **Contrat de module** : `app/src/modules/<id>/manifest.ts` exportant
  `{ id, name (clé i18n), icon, routes, settingsSchema?, commands? }`.
  Découverte automatique via `import.meta.glob('./modules/*/manifest.ts')` → ajouter un
  module ne touche pas le shell.
- Les 9 modules de la spec déclarés avec une page « bientôt » (phase indiquée).
- **Launcher** : grille de tuiles (ordre et activation lus dans `settings.toml`),
  glisser-déposer pour réordonner, clic droit → « ouvrir dans une fenêtre ».
- **Sidebar** globale construite depuis les mêmes manifests.
- **Centre de réglages** : formulaires générés depuis les JSON Schemas
  (générateur maison léger sur shadcn/ui : string, number, boolean, enum, array, object,
  couleur, raccourci) + onglet **éditeur texte brut** (CodeMirror 6, coloration TOML,
  validation en direct via `config_validate`).
- **Thèmes** : tokens de couleur → variables CSS ; clair/sombre ; densité compact/confortable ;
  échelle de police. Appliqués à chaud sur `config_changed`.
- **i18n** : `i18next`, `locales/fr.json` + `locales/en.json` ; aucune chaîne en dur
  (règle ESLint `i18next/no-literal-string`).
- Profils : sélecteur dans la sidebar (bascule d'un clic ; contenu limité au thème en phase 0).
- État : Zustand (UI) + TanStack Query (appels backend, invalidés par `config_changed`).

### Reporté hors phase 0 (assigné plus tard)
- Assistant de premier lancement → phase 1 (il dépend de la détection des salles).
- Export/import `.lps.zip` des profils → phase 2 (quand HUD/filtres existent).
- Sauvegarde SQLite planifiée → phase 1 (avec la base).
- Raccourcis globaux → phase 3 (HUD) ; en phase 0 seuls les raccourcis in-app sont rebindables.

## Crates / dossiers touchés
`Cargo.toml` (workspace), `crates/*` (squelettes), `crates/config` (nouvelle),
`app/src-tauri`, `app/src`, `config/defaults`, `docs/`.

## Tests
| Niveau | Contenu |
|---|---|
| `cargo test` (config) | résolution du dossier de données (normal / `LPS_DATA_DIR` / portable) ; copie des défauts sans écrasement ; tous les défauts valident contre leur schéma ; fichier invalide → ancienne version conservée ; hot reload : écriture d'un fichier → événement reçu < 1 s (test avec `tempdir`) |
| `cargo test` (src-tauri) | commandes testées sans fenêtre via `tauri::test::mock_builder` |
| Vitest | découverte des manifests ; générateur de formulaires (snapshot par type de champ) ; parité des clés `fr.json`/`en.json` ; application des tokens de thème |
| Playwright | démarrage du front (Vite + backend mocké) : tuiles = manifests, réordonnancement, formulaire de réglages → sauvegarde, éditeur brut affiche une erreur de validation |
| Mesure | temps de démarrage (horodatage `setup` → premier rendu) loggé, cible < 2 s en build release |
| Lint | `cargo clippy --workspace -- -D warnings`, `eslint`, `tsc --noEmit` |

## Limites de cet environnement (conteneur Linux cloud)
- Je peux compiler et tester les crates Rust, le front (Vitest/Playwright sur Vite) et
  vraisemblablement le binaire Tauri Linux (après installation de `webkit2gtk-4.1`).
- **Je ne peux pas lancer l'app sous Windows** : le démarrage < 2 s, le mode portable à côté
  du `.exe` et l'ouverture de fenêtres devront être confirmés par toi sur ton PC.
  J'ajouterai un workflow GitHub Actions `windows-latest` (build + tests) pour couvrir
  la compilation Windows automatiquement.

## Décisions prises avec l'utilisateur (3 nov. 2026)

- **Salle prioritaire : Winamax** (puis PokerStars par défaut).
- **PC cible** : Dell Latitude 5590, i7-8650U (4 cœurs / 8 threads, 15 W), 16 Go RAM,
  Windows 64 bits, écran tactile.
  - `performance.worker_threads = 6` (2 threads laissés au client poker + HUD).
  - `performance.ram_cap_mb = 8192` (moitié de la RAM pour le solveur).
  - ⚠️ Les cibles de la spec sont écrites pour un 8 cœurs / 32 Go. Sur ce portable, il faut s'attendre à :
    import ≈ 1 500–2 500 mains/s au lieu de ≥ 3 000 ; solve SRP 100bb à 2 tailles ≈ 4–6 min au lieu de < 2 min ;
    batchs de 1 755 flops impossibles en une nuit avec un arbre complet (utiliser les sous-ensembles 184/95 flops).
    Les critères d'acceptation des phases 1 et 6 seront mesurés et rapportés sur cette machine.

## Résultats

| Critère | Résultat |
|---|---|
| Tuiles + sidebar depuis les manifests | ✅ découverte automatique (`import.meta.glob`), 10 modules ; testé (Vitest + Playwright) |
| Formulaires depuis les JSON Schemas | ✅ settings, hotkeys, thèmes, profils, réglages de module ; widgets couleur, raccourci, ordre/activation des modules |
| Hot reload d'un TOML | ✅ test Rust (< 1 s) + vérifié dans l'app Tauri réelle (Linux) : thème et langue changent sans redémarrage, fichier cassé → bandeau d'erreur et dernière version valide gardée |
| Mode portable | ✅ testé (Rust) + vérifié avec le binaire réel (`portable.flag` → `data/` créé à côté) |
| Démarrage < 2 s | ✅ build release, conteneur Linux sans GPU (4 vCPU) : **0,84 – 0,96 s** jusqu'au premier écran (cœur prêt en 3–4 ms) ; binaire 11 Mo. ⚠️ Sur le PC de l'utilisateur (installeur, **premier** lancement) : **2,77 s** (cœur 35 ms). Le premier lancement inclut la création du profil WebView2 ; à re-mesurer sur les lancements suivants, optimisation prévue en phase 1 si > 2 s |
| `cargo test`, `clippy -D warnings`, `pnpm test`, `pnpm e2e` | ✅ 33 tests Rust, 20 Vitest, 5 Playwright |
| Build Windows | CI GitHub Actions `windows-latest` (tests + installeur NSIS/MSI + zip portable) |

Non vérifié ici : l'ouverture d'un module dans sa propre fenêtre (code en place, `open_module_window`), à tester sur Windows.

Écarts avec le plan : aucun fonctionnel. Le linting i18n est un test Vitest (aucun texte en dur dans le JSX,
parité des clés FR/EN, tous les titres de schéma traduits) au lieu d'une règle ESLint.
