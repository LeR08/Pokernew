import { useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { AlertTriangle, CheckCircle2, Search } from "lucide-react";
import { Card } from "@/components/ui/card";
import { Cards, splitCards } from "@/components/PlayingCard";
import { formatDate } from "@/lib/importApi";
import { bb, euros, formatStat, int, resultClass } from "@/lib/format";
import type { Finding, Session } from "@/lib/tracker";
import { useTracker } from "@/stores/tracker";
import { cn } from "@/lib/utils";
import { useFilter, useSessionReview, useSessions } from "./api";
import { StudyLock } from "@/components/StudyLock";
import { usePlayMode } from "@/stores/playMode";

function duration(s: Session): string {
  const m = Math.max(1, Math.round((s.end - s.start) / 60));
  return m >= 60 ? `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, "0")}` : `${m} min`;
}

function FindingCard({ f }: { f: Finding }) {
  const { t, i18n } = useTranslation();
  const good = f.kind === "good";
  const nf = new Intl.NumberFormat(i18n.language === "fr" ? "fr-FR" : "en-GB", { maximumFractionDigits: 1 });
  const detail = Object.fromEntries(Object.entries(f.detail).map(([k, v]) => [k, typeof v === "number" ? nf.format(v) : v]));
  const Icon = good ? CheckCircle2 : AlertTriangle;
  return (
    <li className={cn("flex gap-d3 rounded-lg border p-d3", good ? "border-success/50" : "border-warning/50")} data-testid={`finding-${f.rule}`}>
      <Icon className={cn("mt-0.5 h-5 w-5 shrink-0", good ? "text-success" : "text-warning")} aria-hidden />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-d2">
          <span className="font-medium">{t(`review.rules.${f.rule}.title`, detail)}</span>
          <span className="text-xs text-muted">
            {formatDate(f.playedAt, i18n.language)} · {f.position ?? "—"} · {int(f.stackBb, i18n.language)} bb
          </span>
        </div>
        <p className="mt-1 text-sm text-muted">{t(`review.rules.${f.rule}.why`, detail)}</p>
      </div>
      <div className="flex flex-col items-end gap-1">
        <Cards cards={splitCards(f.holeCards)} size="sm" />
        <span className={cn("text-sm tabular-nums", resultClass(f.netBb))}>{bb(f.netBb, i18n.language)}</span>
        <Link className="text-xs text-primary underline" to={`/m/replayer/${f.handId}`}>
          {t("review.replay")}
        </Link>
      </div>
    </li>
  );
}

function Review({ s }: { s: Session }) {
  const playing = usePlayMode((p) => p.on);
  if (playing) return <StudyLock />;
  return <ReviewBody s={s} />;
}

function ReviewBody({ s }: { s: Session }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const filter = useFilter();
  const r = useSessionReview(filter, s.start, s.end).data;
  if (!r) return <p className="text-sm text-muted">…</p>;
  const reviews = r.findings.filter((f) => f.kind === "review");
  const goods = r.findings.filter((f) => f.kind === "good");
  return (
    <div className="flex flex-col gap-d4" data-testid="session-review">
      <Card className="flex flex-wrap gap-d6 p-d4 text-sm">
        <div>
          <div className="text-xs text-muted">{t("review.result")}</div>
          <div className={cn("text-2xl font-semibold tabular-nums", resultClass(s.netBb))}>{bb(s.netBb, lang)}</div>
        </div>
        <div>
          <div className="text-xs text-muted">{t("review.vpip_pfr")}</div>
          <div className="text-lg tabular-nums">
            {formatStat(r.vpip, "pct0", lang)} / {formatStat(r.pfr, "pct0", lang)}
          </div>
          <div className="text-xs text-muted">
            {t("review.usual", { vpip: formatStat(r.vpipOverall, "pct0", lang), pfr: formatStat(r.pfrOverall, "pct0", lang) })}
          </div>
        </div>
        <div>
          <div className="text-xs text-muted">{t("review.counts")}</div>
          <div className="text-lg">
            <span className="text-warning">{t("review.to_review", { count: reviews.length })}</span> ·{" "}
            <span className="text-success">{t("review.good_plays", { count: goods.length })}</span>
          </div>
        </div>
      </Card>
      {r.tiltWarning && (
        <div role="alert" className="rounded-lg border border-warning p-d3 text-sm text-warning" data-testid="tilt-warning">
          {t("review.tilt")}
        </div>
      )}
      <p className="text-xs text-muted">{t("review.disclaimer")}</p>
      <section>
        <h3 className="mb-d2 font-semibold">{t("review.section_review")}</h3>
        {reviews.length ? (
          <ul className="flex flex-col gap-d2">
            {reviews.map((f, i) => (
              <FindingCard key={`${f.handId}-${f.rule}-${i}`} f={f} />
            ))}
          </ul>
        ) : (
          <p className="text-sm text-success">{t("review.nothing_to_review")}</p>
        )}
      </section>
      <section>
        <h3 className="mb-d2 font-semibold">{t("review.section_good")}</h3>
        {goods.length ? (
          <ul className="flex flex-col gap-d2">
            {goods.map((f, i) => (
              <FindingCard key={`${f.handId}-${f.rule}-${i}`} f={f} />
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted">{t("review.no_good")}</p>
        )}
      </section>
    </div>
  );
}

/** Sessions (30-minute gaps) and the post-session review. */
export function Sessions() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const euro = useTracker((s) => s.euros);
  const sessions = useSessions(useFilter()).data ?? [];
  const [sel, setSel] = useState<number | null>(null);
  const current = sessions.find((s) => s.start === sel) ?? sessions[0];
  if (sessions.length === 0) return <p className="text-muted">{t("tracker.no_hands")}</p>;
  return (
    <div className="grid grid-cols-1 gap-d4 lg:grid-cols-[20rem_1fr]">
      <Card className="flex max-h-[40rem] flex-col gap-1 overflow-y-auto p-d2" data-testid="session-list">
        {sessions.map((s) => (
          <button
            key={s.start}
            className={cn("rounded px-d3 py-d2 text-left text-sm hover:bg-surface-alt", current?.start === s.start && "bg-surface-alt")}
            onClick={() => setSel(s.start)}
          >
            <div className="flex justify-between gap-d2">
              <span className="font-medium">{formatDate(s.start, lang)}</span>
              <span className={cn("tabular-nums", resultClass(euro && s.netCashCents ? s.netCashCents : s.netBb))}>
                {euro && s.netCashCents ? euros(s.netCashCents, lang) : bb(s.netBb, lang)}
              </span>
            </div>
            <div className="text-xs text-muted">
              {duration(s)} · {t("review.hands", { n: s.hands })}
              {s.tournaments > 0 && ` · ${t("review.tournaments", { count: s.tournaments })}`}
            </div>
          </button>
        ))}
      </Card>
      <div className="flex flex-col gap-d3">
        <h2 className="flex items-center gap-d2 text-lg font-semibold">
          <Search className="h-5 w-5 text-primary" aria-hidden />
          {t("review.title")}
        </h2>
        {current && <Review s={current} />}
      </div>
    </div>
  );
}
