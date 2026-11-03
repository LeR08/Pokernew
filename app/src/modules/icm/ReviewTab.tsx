/**
 * Tournament hand review: hero's push/call/fold against the push/fold
 * equilibrium, in chips and money. One hand (?hand=ID, from the replayer) or a
 * whole tournament (?tournament=ID, from the tournament list).
 */
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useSearchParams } from "react-router-dom";
import { PlayingCard } from "@/components/PlayingCard";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import type { BatchRow, Review, ReviewRequest } from "@/lib/icm";
import type { TournamentRow } from "@/lib/tracker";
import { cn } from "@/lib/utils";
import { useFilter } from "../tracker/api";
import { parsePlaces } from "./payouts";
import { PayoutPicker } from "./PayoutPicker";

function Verdict({ r }: { r: Review }) {
  const { t } = useTranslation();
  const ok = r.played === r.best;
  const money = r.moneyAct !== null && r.moneyFold !== null;
  return (
    <div className="flex flex-col gap-d2" data-testid="review-verdict">
      <p className={cn("text-lg font-semibold", ok ? "text-success" : "text-warning")}>
        {ok ? t("icm.review.good", { c: t(`icm.review.choice.${r.played}`) }) : t("icm.review.bad", { c: t(`icm.review.choice.${r.played}`), b: t(`icm.review.choice.${r.best}`) })}
      </p>
      <table className="max-w-lg text-sm">
        <thead className="text-left text-xs text-muted">
          <tr>
            <th />
            <th className="text-right">{t(r.pusher === null ? "icm.review.choice.push" : "icm.review.choice.call")}</th>
            <th className="text-right">{t("icm.review.choice.fold")}</th>
            <th className="text-right">{t("icm.review.loss")}</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td className="py-0.5 text-muted">{t("icm.review.chips")}</td>
            <td className="text-right tabular-nums">{Math.round(r.chipsAct)}</td>
            <td className="text-right tabular-nums">{Math.round(r.chipsFold)}</td>
            <td className="text-right tabular-nums" data-testid="loss-chips">
              {Math.round(r.lossChips)}
            </td>
          </tr>
          {money && (
            <tr>
              <td className="py-0.5 text-muted">{t("icm.review.money")}</td>
              <td className="text-right tabular-nums">{r.moneyAct!.toFixed(3)} €</td>
              <td className="text-right tabular-nums">{r.moneyFold!.toFixed(3)} €</td>
              <td className="text-right tabular-nums" data-testid="loss-money">
                {r.lossMoney!.toFixed(3)} €
              </td>
            </tr>
          )}
        </tbody>
      </table>
      {!money && <p className="text-xs text-muted">{t("icm.review.chips_only")}</p>}
    </div>
  );
}

function SpotTable({ r }: { r: Review }) {
  const { t } = useTranslation();
  return (
    <table className="max-w-lg text-sm" data-testid="review-spot">
      <tbody>
        {r.names.map((name, i) => (
          <tr key={i} className={i === r.hero ? "font-semibold" : ""}>
            <td className="py-0.5 pr-d3 text-muted">{r.positions[i] || "—"}</td>
            <td className="pr-d3">{name}</td>
            <td className="text-right tabular-nums">{t("icm.review.bb", { n: (r.spot.stacks[i] / r.spot.bb).toFixed(1) })}</td>
            <td className="pl-d2 text-xs text-muted">{i === r.pusher ? t("icm.review.shoves") : ""}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export function ReviewTab() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const handId = Number(params.get("hand")) || null;
  const tournamentId = Number(params.get("tournament")) || null;
  const filter = useFilter();
  const [tournaments, setTournaments] = useState<TournamentRow[]>([]);
  const [prizesText, setPrizesText] = useState("");
  const [others, setOthers] = useState("");
  const [bounties, setBounties] = useState(false);
  const [share, setShare] = useState(50);
  const [one, setOne] = useState<Review | null>(null);
  const [batch, setBatch] = useState<BatchRow[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void getBackend()
      .then((b) => b.trackerTournaments(filter))
      .then((r) => setTournaments(r.rows))
      .catch(() => setTournaments([]));
  }, [filter]);

  const req = (): ReviewRequest | null => {
    const payouts = prizesText.trim() ? parsePlaces(prizesText) : [];
    const oth = others.trim() ? parsePlaces(others) : [];
    if (payouts === null || oth === null) return null;
    return { payouts, others: oth, useBounties: bounties, bountyShare: share / 100 };
  };

  const run = () => {
    const r = req();
    if (!r) return setError(t("icm.bad_prizes"));
    setBusy(true);
    setError(null);
    void getBackend()
      .then(async (b) => {
        if (handId) setOne(await b.icmReviewHand(handId, r));
        else if (tournamentId) setBatch(await b.icmReviewTournament(tournamentId, r));
      })
      .catch((e: Error) => setError(e.message))
      .finally(() => setBusy(false));
  };

  useEffect(() => {
    setOne(null);
    setBatch(null);
    if (handId || tournamentId) run();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [handId, tournamentId]);

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4">
        <div className="flex flex-wrap items-end gap-d3 text-sm">
          <label className="flex min-w-72 flex-col gap-1">
            {t("icm.review.tournament")}
            <Select value={tournamentId ?? ""} onChange={(e) => setParams(e.target.value ? { tournament: e.target.value } : {})} data-testid="review-tournament">
              <option value="">{handId ? t("icm.review.one_hand", { id: handId }) : t("icm.review.pick")}</option>
              {tournaments.map((x) => (
                <option key={x.id} value={x.id}>
                  {x.name} · {x.hands} {t("icm.review.hands")}
                </option>
              ))}
            </Select>
          </label>
        </div>
        <PayoutPicker value={prizesText} onChange={setPrizesText} testId="review-payouts" />
        <div className="flex flex-wrap items-end gap-d3 text-sm">
          <label className="flex min-w-56 flex-1 flex-col gap-1">
            {t("icm.pf.others_chips")}
            <Input value={others} onChange={(e) => setOthers(e.target.value)} placeholder="12000, 30500" />
          </label>
          <label className="flex items-center gap-1">
            <input type="checkbox" checked={bounties} onChange={(e) => setBounties(e.target.checked)} data-testid="review-bounties" />
            {t("icm.review.bounties")}
          </label>
          {bounties && (
            <label className="flex flex-col gap-1">
              {t("icm.review.share")}
              <Input type="number" min={0} max={100} className="w-24" value={share} onChange={(e) => setShare(Number(e.target.value))} />
            </label>
          )}
          <Button onClick={run} disabled={busy || (!handId && !tournamentId)} data-testid="review-run">
            {busy ? t("icm.review.running") : t("icm.review.run")}
          </Button>
        </div>
        <p className="text-xs text-muted">{t("icm.review.help")}</p>
        {error && (
          <p className="text-sm text-danger" data-testid="review-error">
            {error}
          </p>
        )}
      </Card>

      {one && (
        <Card className="flex flex-col gap-d3 p-d4" data-testid="review-one">
          <div className="flex items-center gap-d3">
            <span className="flex gap-0.5">
              {[0, 2].map((i) => (
                <PlayingCard key={i} card={one.heroCards.slice(i, i + 2)} />
              ))}
            </span>
            <span className="text-sm text-muted">{t("icm.review.eff", { n: one.effectiveBb.toFixed(1) })}</span>
            {handId && (
              <Link className="text-sm text-primary underline" to={`/m/replayer/${handId}`}>
                {t("icm.review.replay")}
              </Link>
            )}
          </div>
          <SpotTable r={one} />
          <Verdict r={one} />
        </Card>
      )}

      {batch && (
        <Card className="flex flex-col gap-d2 p-d4" data-testid="review-batch">
          <p className="text-sm">{t("icm.review.batch_total", { n: batch.length, bad: batch.filter((b) => b.review.played !== b.review.best).length })}</p>
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-muted">
              <tr>
                <th className="py-1">{t("icm.review.level")}</th>
                <th>{t("icm.review.hand")}</th>
                <th>{t("icm.review.stack")}</th>
                <th>{t("icm.review.played")}</th>
                <th>{t("icm.review.best")}</th>
                <th className="text-right">{t("icm.review.loss")}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {batch.map((b) => (
                <tr key={b.handId} className="border-t border-border" data-testid="batch-row">
                  <td className="py-1">{b.level ?? "—"}</td>
                  <td className="font-mono">{b.review.heroCards}</td>
                  <td>{b.review.effectiveBb.toFixed(1)} bb</td>
                  <td>{t(`icm.review.choice.${b.review.played}`)}</td>
                  <td className={b.review.played === b.review.best ? "text-success" : "text-warning"}>{t(`icm.review.choice.${b.review.best}`)}</td>
                  <td className="text-right tabular-nums">{b.review.lossMoney !== null ? `${b.review.lossMoney.toFixed(3)} €` : `${Math.round(b.review.lossChips)}`}</td>
                  <td className="text-right">
                    <Link className="text-primary underline" to={`/m/icm/review?hand=${b.handId}`}>
                      {t("icm.review.open")}
                    </Link>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}
    </div>
  );
}
