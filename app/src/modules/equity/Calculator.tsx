import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { Plus, Square, Trash2, Grid3x3 } from "lucide-react";
import { CardPicker } from "@/components/range/CardPicker";
import { RangeEditor } from "@/components/range/RangeEditor";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import type { EquityResult } from "@/lib/equity";
import { liveCount, parseCards, tryParseRange } from "@/lib/range";
import { EquityCurves, seriesColour, type Curve } from "./charts";
import { combos, count, pct } from "./format";
import { Modal } from "./Modal";
import { useEquity } from "./store";
import { ChartFolderPicker, QuickRanges, rangeOf, useTrackedPlayers } from "./QuickRanges";
import type { RangeKind } from "@/lib/playerRange";

export function Calculator() {
  const { t } = useTranslation();
  const s = useEquity();
  const [params, setParams] = useSearchParams();
  const [editing, setEditing] = useState<number | null>(null);
  const [result, setResult] = useState<EquityResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);

  // Pre-filled from the replayer: ?players=AhKd|random&board=Ah7c2d&est=1:name:pfr,2:name:call
  // ("est" = players whose cards are unknown get the range estimated from the tracker).
  const [pendingEst] = useState(() => params.get("est"));
  const tracked = useTrackedPlayers(!!pendingEst).data;
  const [estNote, setEstNote] = useState<string | null>(null);
  useEffect(() => {
    const p = params.get("players");
    if (p) {
      s.set({ players: p.split("|"), board: params.get("board") ?? "", dead: "", who: [] });
      setParams({}, { replace: true });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(() => {
    if (!pendingEst || !tracked) return;
    const st = useEquity.getState();
    const players = [...st.players];
    const who: (string | null)[] = players.map(() => null);
    const done: string[] = [];
    const missing: string[] = [];
    for (const item of pendingEst.split(",")) {
      const [i, name, kind] = item.split(":").map(decodeURIComponent);
      const idx = Number(i);
      const text = rangeOf(tracked, name, kind as RangeKind);
      if (text && idx < players.length) {
        players[idx] = text;
        who[idx] = name;
        done.push(name);
      } else missing.push(name);
    }
    st.set({ players, who });
    setEstNote(
      [done.length ? t("equity.est_done", { names: done.join(", ") }) : "", missing.length ? t("equity.est_missing", { names: missing.join(", ") }) : ""]
        .filter(Boolean)
        .join(" "),
    );
  }, [pendingEst, tracked, t]);

  const boardCards = parseCards(s.board) ?? [];
  const deadCards = parseCards(s.dead) ?? [];
  const blocked = [...boardCards, ...deadCards];
  const parsed = s.players.map((p) => tryParseRange(p));
  const valid = parsed.every((w) => w && liveCount(w, blocked) > 0) && parseCards(s.board) !== null && parseCards(s.dead) !== null;

  const setPlayer = (i: number, v: string) => s.set({ players: s.players.map((p, j) => (j === i ? v : p)) });

  const run = () => {
    setRunning(true);
    setError(null);
    void getBackend()
      .then((b) => b.equityCalc({ players: s.players.map((p) => p.trim()), board: s.board, dead: s.dead, mode: s.mode, precision: s.precision }))
      .then(setResult)
      .catch((e: Error) => setError(e.message))
      .finally(() => setRunning(false));
  };

  const curves: Curve[] = useMemo(() => {
    if (!result) return [];
    return result.players
      .map((p, index) => {
        const sorted = [...p.perCombo].sort((a, b) => b.equity - a.equity);
        const total = sorted.reduce((a, c) => a + c.weight, 0);
        let acc = 0;
        const points: [number, number][] = [];
        for (const c of sorted) {
          points.push([acc / total, c.equity]);
          acc += c.weight;
          points.push([acc / total, c.equity]);
        }
        return { name: t("equity.player", { n: index + 1 }), points, index, label: pct(p.equity), n: sorted.length };
      })
      .filter((c) => c.n > 1);
  }, [result, t]);

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4">
        <ChartFolderPicker />
        {estNote && (
          <p className="text-xs text-muted" data-testid="est-note">
            {estNote}
          </p>
        )}
        <ul className="flex flex-col gap-d2" data-testid="players">
          {s.players.map((p, i) => {
            const w = parsed[i];
            return (
              <li key={i} className="flex flex-col gap-1" data-testid="player-row">
                <div className="flex flex-wrap items-center gap-d2">
                <span className="inline-block h-2.5 w-2.5 rounded-full" style={{ background: seriesColour(i) ?? "transparent" }} aria-hidden />
                <span className="w-20 truncate text-sm" title={s.who[i] ?? undefined}>
                  {s.who[i] ?? t("equity.player", { n: i + 1 })}
                </span>
                <Input
                  className="min-w-48 flex-1 font-mono"
                  value={p}
                  onChange={(e) => setPlayer(i, e.target.value)}
                  aria-invalid={!w}
                  placeholder={t("equity.player_placeholder")}
                  data-testid="player-input"
                  aria-label={t("equity.player", { n: i + 1 })}
                />
                <span className="w-24 text-xs text-muted">{w ? t("range.combos", { n: combos(liveCount(w, blocked)) }) : t("equity.invalid")}</span>
                <Button size="sm" variant="outline" onClick={() => setEditing(i)} data-testid="player-edit">
                  <Grid3x3 className="h-4 w-4" />
                  {t("equity.edit_range")}
                </Button>
                <Button
                  size="icon"
                  variant="ghost"
                  disabled={s.players.length <= 2}
                  onClick={() => s.set({ players: s.players.filter((_, j) => j !== i), who: s.who.filter((_, j) => j !== i) })}
                  aria-label={t("equity.remove_player")}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
                </div>
                <QuickRanges index={i} />
              </li>
            );
          })}
        </ul>
        <div>
          <Button
            size="sm"
            variant="outline"
            disabled={s.players.length >= 10}
            onClick={() => s.set({ players: [...s.players, "random"] })}
            data-testid="add-player"
          >
            <Plus className="h-4 w-4" />
            {t("equity.add_player")}
          </Button>
        </div>
        <div className="flex flex-wrap items-end gap-d4">
          <CardPicker label={t("equity.board")} value={s.board} onChange={(v) => s.set({ board: v })} max={5} blocked={deadCards} testId="calc-board" />
          <CardPicker label={t("equity.dead")} value={s.dead} onChange={(v) => s.set({ dead: v })} max={10} blocked={boardCards} testId="dead" />
          <label className="flex flex-col gap-1 text-sm">
            {t("equity.method")}
            <Select value={s.mode} onChange={(e) => s.set({ mode: e.target.value as typeof s.mode })} className="w-44" data-testid="mode">
              <option value="auto">{t("equity.mode.auto")}</option>
              <option value="exhaustive">{t("equity.mode.exhaustive")}</option>
              <option value="monte_carlo">{t("equity.mode.monte_carlo")}</option>
            </Select>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t("equity.precision")}
            <Select value={s.precision} onChange={(e) => s.set({ precision: Number(e.target.value) })} className="w-32" data-testid="precision">
              <option value={0.005}>± 0,5 %</option>
              <option value={0.001}>± 0,1 %</option>
              <option value={0.0005}>± 0,05 %</option>
            </Select>
          </label>
          {running ? (
            <Button variant="outline" onClick={() => void getBackend().then((b) => b.equityCancel())} data-testid="cancel">
              <Square className="h-4 w-4" />
              {t("equity.stop")}
            </Button>
          ) : (
            <Button disabled={!valid} onClick={run} data-testid="run">
              {t("equity.run")}
            </Button>
          )}
        </div>
        {error && (
          <p className="text-sm text-danger" data-testid="calc-error">
            {error}
          </p>
        )}
      </Card>

      {result && (
        <Card className="flex flex-col gap-d3 p-d4" data-testid="results">
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-muted">
              <tr>
                <th className="py-1">{t("equity.player_col")}</th>
                <th>{t("equity.hand_col")}</th>
                <th className="text-right">{t("equity.equity")}</th>
                <th className="text-right">{t("equity.win")}</th>
                <th className="text-right">{t("equity.tie")}</th>
                <th className="text-right">{t("equity.combos")}</th>
              </tr>
            </thead>
            <tbody>
              {result.players.map((p, i) => (
                <tr key={i} className="border-t border-border" data-testid="result-row">
                  <td className="py-1.5">
                    <span className="mr-1.5 inline-block h-2.5 w-2.5 rounded-full" style={{ background: seriesColour(i) ?? "transparent" }} aria-hidden />
                    {t("equity.player", { n: i + 1 })}
                  </td>
                  <td className="max-w-64 truncate font-mono text-xs">{s.players[i]}</td>
                  <td className="text-right text-base font-semibold" data-testid="result-equity">
                    {pct(p.equity)}
                  </td>
                  <td className="text-right">{pct(p.win)}</td>
                  <td className="text-right">{pct(p.tie)}</td>
                  <td className="text-right">{combos(p.combos)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="text-xs text-muted" data-testid="result-method">
            {result.method === "exhaustive"
              ? t("equity.done_exhaustive", { n: count(result.samples), ms: result.elapsedMs })
              : t("equity.done_mc", { n: count(result.samples), ms: result.elapsedMs, margin: pct(result.margin, 2) })}
          </p>
          <EquityCurves curves={curves} />
        </Card>
      )}

      {editing !== null && (
        <Modal title={t("equity.range_of", { n: editing + 1 })} onClose={() => setEditing(null)}>
          <RangeEditor value={s.players[editing]} onChange={(v) => setPlayer(editing, v)} dead={blocked} />
        </Modal>
      )}
    </div>
  );
}
