# Phase 9 — Finitions

Statut : **implémentée — à tester sur ton PC**

## Critères d'acceptation (spec)
- un lecteur d'historiques pour chaque salle restante, avec des fixtures ;
- un installeur Windows et un zip portable ;
- un README en français et en anglais.

## Ce qui a été fait
- **Salles** : tu ne joues que sur Winamax, et aucun fichier d'une autre salle n'a été fourni. La spec interdit
  de deviner un format (« never guess a format »), donc **aucun autre lecteur n'est écrit**. Winamax est couvert
  (MTT, KO, satellites, Expresso, cash, résumés), testé sur tes fichiers (0 erreur sur 7 108 mains).
  Pour ajouter une salle (voir plus bas), il suffit de m'envoyer quelques fichiers.
- **Installeur et zip portable** : la CI construit l'installeur NSIS, le `.msi` et `LocalPokerSuite-portable.zip`.
  Le zip contient l'exécutable, `portable.flag`, le README, la licence, `THIRD_PARTY.md` et le CHANGELOG.
- **Version 1.0.0** (Cargo, Tauri, front).
- **README** réécrit en français et en anglais : modules, fair-play, installation, compilation, dossier de
  données, salles, développement, licence.
- **`THIRD_PARTY.md`** : les composants tiers (postflop-solver en AGPL, intégré) et les références de test (eval7,
  tableaux Nash HoldemResources).

## Ajouter une salle
1. Mettre de vrais fichiers d'historique et de résumé dans `crates/hh-parsers/fixtures/<salle>/`.
2. Écrire `crates/hh-parsers/src/<salle>/mod.rs`, qui implémente `HandHistoryParser` (`sniff`, `split`, `parse`,
   et en option `sniff_summary` / `parse_summaries`) comme `winamax/mod.rs`.
3. L'ajouter à `registry()` dans `crates/hh-parsers/src/lib.rs`, puis ajouter `config/defaults/rooms/<salle>.toml`
   (dossiers, `hud_allowed`, processus et motif de titre des fenêtres de table).
4. Tests : 0 erreur sur toutes les fixtures, aller-retour en base, stats contre une référence.

## Toujours en attente
- La comparaison du solveur postflop avec TexasSolver sur les 5 spots préparés (phase 6). Il faut soit ton accord
  pour le compiler et le lancer ici, soit tes résultats faits sur ton PC.
- Winamax Go Fast : il manque des fichiers d'exemple.
