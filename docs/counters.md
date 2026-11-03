# Compteurs / Counters

Chaque stat est un rapport de deux compteurs stockés une fois par joueur et par main
(`hand_player_stats`), calculés à l'import par `crates/db/src/counters.rs`.
Exemple : VPIP = `SUM(cnt_vpip) / SUM(cnt_hands)`.

Les définitions ci-dessous sont celles du code ; les tests de `crates/db/src/tests.rs`
les vérifient sur des mains réelles comptées à la main.

| Compteur | Définition |
|---|---|
| `cnt_hands` | 1 par main où le joueur est servi |
| `cnt_vpip` | a fait call, bet ou raise préflop (les blindes et antes ne comptent pas ; un check de BB non plus) |
| `cnt_pfr` | a relancé préflop |
| `cnt_limp` | sa première action volontaire préflop est un call alors que personne n'a relancé (la SB qui complète compte) |
| `opp_3bet` / `cnt_3bet` | a dû agir face à exactement une relance faite par un autre ; a relancé |
| `opp_fold_to_3bet` / `cnt_fold_to_3bet` | était le premier relanceur et fait face à une 3-bet ; s'est couché |
| `opp_steal` / `cnt_steal` | en CO, BTN ou SB, première décision, tout le monde s'est couché avant ; a relancé (ATS) |
| `opp_fold_to_steal` / `cnt_fold_to_steal` | en SB ou BB, face à cette relance de vol, sans call entre les deux ; s'est couché |
| `cnt_saw_flop` | toujours dans le coup quand le flop est distribué (y compris à tapis) |
| `opp_cbet_flop` / `cnt_cbet_flop` | dernier relanceur préflop, sa première action au flop arrive sans mise avant lui ; a misé |
| `opp_fold_to_cbet_flop` / `cnt_fold_to_cbet_flop` | première action après ce c-bet, sans relance entre-temps ; s'est couché |
| `cnt_wtsd` | a vu le flop et est allé à l'abattage |
| `cnt_wsd` | est allé à l'abattage et son gain net sur la main est positif |
| `cnt_wwsf` | a vu le flop et son gain net est positif |
| `cnt_postflop_bets_raises` / `calls` / `folds` | nombre de bets+raises, de calls, de folds au flop, au turn et à la river (pour AF et AFq) |
| `cnt_allin_preflop` | une de ses actions volontaires préflop est à tapis |
| `cnt_won_hand` | gain net positif sur la main |

« Gain net positif » plutôt que « a ramassé un pot » : sur Winamax, une mise non suivie revient
au joueur sous forme de *side pot* ; ce retour ne doit pas compter comme une main gagnée.

Ajouter un compteur : l'ajouter à `config/defaults/counters.toml`, à `COUNTERS` et à `compute()`.
La colonne est créée au lancement suivant et toute la base est recalculée (`recompute_counters`).
