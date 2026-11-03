# Phase 3 — HUD

Statut : **implémentée — à tester sur ton PC Windows** · PC cible : Windows 11, Dell Latitude 5590 (écran tactile)

## Critères d'acceptation (spec)
- suit 6 tables déplacées et redimensionnées sur 2 écrans ;
- stats rafraîchies moins d'une seconde après la fin d'une main ;
- les modifications de disposition sont conservées ;
- `hud_allowed = false` masque le HUD.

## Comment le HUD trouve les tables (sans deviner le format des titres)
Je n'ai pas encore de capture des fenêtres de table Winamax. Plutôt que de deviner le format des titres,
on rapproche **les fenêtres ouvertes** et **les tables des mains importées dans les 15 dernières minutes** :
une fenêtre est une table quand son titre contient le nom de table de l'historique (`SPACE KO(1195603616)#0043`)
ou le nom et le numéro du tournoi. Le nom du programme (`process`) et un motif de titre (`title_pattern`)
restent réglables dans `rooms/winamax.toml`.

Un **inspecteur de fenêtres**, dans le module HUD, liste les fenêtres ouvertes avec leur titre, leur classe
et leur programme. Tu pourras me le copier pour régler la détection exactement sur ton client.

## Tâches
1. `hud-overlay` : suivi des fenêtres Windows (EnumWindows + SetWinEventHook : création, déplacement,
   redimensionnement, fermeture, premier plan), DPI, multi-écrans ; rapprochement fenêtres ↔ tables, en
   logique pure et testée.
2. Service HUD (src-tauri) : une fenêtre transparente, toujours au-dessus et sans barre des tâches par table,
   qui suit la table. Stats par siège depuis la base, rafraîchies à chaque import.
3. Dispositions `hud/*.toml` : cellules (stat, format, couleurs), positions des sièges relatives (0–1) par
   taille de table (2, 3, 6, 9 et 10 joueurs), opacité, police, panneau héros, mode compact.
4. Mode « édition de disposition » : on glisse les panneaux et la disposition est enregistrée dans le fichier.
5. Popup au survol d'un panneau : stats détaillées et notes du joueur, y compris les auto-notes.
6. Raccourci global pour afficher ou masquer le HUD (`hotkeys.toml`, `toggle_hud`).
7. **Mode jeu (fair-play)** : dès qu'une table est ouverte, Solveur, Entraîneur, ICM, Préflop et le bilan de
   session sont verrouillés. Ils se déverrouillent 60 s après la fermeture de la dernière table. C'est une règle
   fixe, pas un réglage.
8. `hud_allowed = false` : aucun HUD pour cette salle. Le tracker continue d'importer.
9. **Table simulée** : un aperçu du HUD sur une fausse table, pour régler la disposition sans jouer. Ça sert
   aussi à le tester ici, sous Linux.

## Limites
Le suivi des fenêtres est écrit pour Windows. Ici sous Linux, je vérifie la compilation Windows et je teste la
logique de rapprochement et la table simulée. Le comportement réel sur tes tables Winamax devra être confirmé par toi.

## Résultats (3 nov. 2026)

| Critère | État |
|---|---|
| Suit 6 tables déplacées et redimensionnées sur 2 écrans | Code en place : SetWinEventHook sur création, déplacement, taille, titre, réduction, premier plan, hors processus et sans notre propre processus ; position et taille en pixels physiques ; DPI par fenêtre. **À vérifier par toi sous Windows** |
| Stats rafraîchies < 1 s après la fin d'une main | Import ≈ 0,3 s (phase 1), puis l'événement `hud://refresh` et une seule requête par siège (index joueur + date). **À mesurer chez toi** |
| Modifications de disposition conservées | ✅ vérifié dans la vraie app (table simulée) : le panneau déplacé est enregistré dans `hud/standard.toml`, commentaires conservés |
| `hud_allowed = false` masque le HUD | ✅ aucune fenêtre HUD n'est ouverte pour une salle sans autorisation ; le tracker continue d'importer |

Vérifié ici (Linux, vraie app Tauri) :
- la table simulée ouvre une fenêtre HUD avec un panneau par adversaire et ses vraies stats ;
- le mode édition, le glisser-déposer et le popup détaillé avec les notes fonctionnent ;
- le raccourci global Ctrl+Alt+H masque puis réaffiche le HUD.

Testé :
- le rapprochement fenêtre ↔ table (6 tests) ;
- la règle du mode jeu (verrouillé pendant le jeu, déverrouillé 60 s après) ;
- la rotation des sièges (héros en slot 0) ;
- les stats par siège ;
- le verrouillage des modules d'étude dans l'interface ;
- le refus du bilan de session côté serveur en mode jeu.

Choix techniques :
- **HUD transparent aux clics** : en mode normal, la fenêtre laisse passer tous les clics vers la table, pour ne jamais gêner le client. Le popup et le déplacement ne marchent qu'en mode édition (Ctrl+Alt+E).
- **Uniquement des stats historiques** dans le HUD : pas d'équité, de range ni de conseil sur la main en cours (fair-play).

À confirmer avec tes captures et l'inspecteur de fenêtres :
- que les titres des fenêtres de table Winamax contiennent bien le nom de la table ;
- l'ordre des sièges (sens horaire, héros en bas) pour les positions par défaut.
