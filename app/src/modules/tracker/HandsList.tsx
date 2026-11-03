import { useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Cards, splitCards } from "@/components/PlayingCard";
import { formatDate } from "@/lib/importApi";
import { bb, euros, int, resultClass } from "@/lib/format";
import { useTracker } from "@/stores/tracker";
import { useFilter, useHands } from "./api";

const PAGE = 50;

export function HandsList() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const base = useFilter();
  const tournament = params.get("tournament");
  const filter = tournament ? { ...base, tournament: Number(tournament) } : base;
  const euro = useTracker((s) => s.euros);
  const [page, setPage] = useState(0);
  const data = useHands(filter, PAGE, page * PAGE).data;
  if (!data) return null;
  const pages = Math.max(1, Math.ceil(data.total / PAGE));
  return (
    <div className="flex flex-col gap-d3">
      <div className="flex items-center justify-between text-sm text-muted">
        <span>{t("tracker.hands_count", { n: int(data.total, lang) })}</span>
        <div className="flex items-center gap-d2">
          <Button variant="outline" size="sm" disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
            ←
          </Button>
          <span>
            {page + 1} / {pages}
          </span>
          <Button variant="outline" size="sm" disabled={page + 1 >= pages} onClick={() => setPage((p) => p + 1)}>
            →
          </Button>
        </div>
      </div>
      <div className="overflow-x-auto rounded-lg border border-border">
        <table className="w-full text-sm tabular-nums" data-testid="hands-table">
          <thead className="bg-surface-alt text-left text-xs text-muted">
            <tr>
              <th className="px-d3 py-d2 font-medium">{t("tracker.col.date")}</th>
              <th className="px-d3 py-d2 font-medium">{t("tracker.col.table")}</th>
              <th className="px-d3 py-d2 font-medium">{t("tracker.col.position")}</th>
              <th className="px-d3 py-d2 font-medium">{t("tracker.col.cards")}</th>
              <th className="px-d3 py-d2 font-medium">{t("tracker.col.board")}</th>
              <th className="px-d3 py-d2 text-right font-medium">{t("tracker.col.result")}</th>
            </tr>
          </thead>
          <tbody>
            {data.rows.map((h) => (
              <tr
                key={h.handId}
                className="cursor-pointer border-t border-border hover:bg-surface-alt"
                onClick={() => navigate(`/m/replayer/${h.handId}`)}
                data-testid="hand-row"
              >
                <td className="px-d3 py-d2 whitespace-nowrap">{formatDate(h.playedAt, lang)}</td>
                <td className="max-w-56 truncate px-d3 py-d2">{h.tableName}</td>
                <td className="px-d3 py-d2">{h.position ?? "—"}</td>
                <td className="px-d3 py-d2">
                  <Cards cards={splitCards(h.holeCards)} size="sm" />
                </td>
                <td className="px-d3 py-d2">
                  <Cards cards={splitCards(h.board)} size="sm" />
                </td>
                <td className={`px-d3 py-d2 text-right ${resultClass(h.netWon)}`}>
                  {euro && h.unit === "cents" ? euros(h.netWon, lang) : bb(h.netWonBb, lang)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
