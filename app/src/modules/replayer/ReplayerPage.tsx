import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { ChevronFirst, ChevronLast, ChevronLeft, ChevronRight, Pause, Play } from "lucide-react";
import { getBackend } from "@/lib/backend";
import { formatDate } from "@/lib/importApi";
import type { Hand } from "@/lib/tracker";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { PlayingCard } from "@/components/PlayingCard";
import { cn } from "@/lib/utils";
import { HandsList } from "../tracker/HandsList";
import { FilterBar } from "../tracker/FilterBar";
import type { ModuleManifest } from "../types";
import { stateAt, totalSteps } from "./replay";
import { NotesPanel } from "./NotesPanel";
import { usePlayMode } from "@/stores/playMode";

/**
 * Equity Lab pre-filled with the hand as played, whatever the replay step: hero's cards, the players still
 * in at the end (their cards when shown, otherwise a random hand) and the full board.
 */
function equityLink(hand: Hand, heroIdx: number, board: (string | undefined)[], seats: { folded?: boolean }[]): string {
  const players = [hand.players[heroIdx].hole_cards.join("")];
  hand.players.forEach((p, i) => {
    if (i !== heroIdx && !seats[i]?.folded) players.push(p.hole_cards.length === 2 ? p.hole_cards.join("") : "random");
  });
  if (players.length < 2) players.push("random");
  const q = new URLSearchParams({ players: players.slice(0, 10).join("|"), board: board.filter(Boolean).join("") });
  return `/m/equity/calc?${q.toString()}`;
}

function useAmount(h: Hand | undefined, inBb: boolean, lang: string) {
  return useMemo(() => {
    const nf = new Intl.NumberFormat(lang === "fr" ? "fr-FR" : "en-GB", { maximumFractionDigits: 1 });
    return (v: number) => {
      if (!h) return "";
      if (inBb) return `${nf.format(v / h.big_blind)} bb`;
      if (h.unit === "cents") return `${(v / 100).toFixed(2).replace(".", lang === "fr" ? "," : ".")} €`;
      return nf.format(v);
    };
  }, [h, inBb, lang]);
}

/** Seat coordinates on an ellipse, hero at the bottom. */
function seatPos(i: number, n: number): { left: string; top: string } {
  const angle = Math.PI / 2 + (2 * Math.PI * i) / n;
  return { left: `${50 + 42 * Math.cos(angle)}%`, top: `${50 + 38 * Math.sin(angle)}%` };
}

function Replayer({ hand }: { hand: Hand }) {
  const { t, i18n } = useTranslation();
  const [step, setStep] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [inBb, setInBb] = useState(true);
  const [notesFor, setNotesFor] = useState<string | null>(null);
  const fmt = useAmount(hand, inBb, i18n.language);
  const last = totalSteps(hand);
  const st = stateAt(hand, step);
  // Fair play: no equity link while a table is open.
  const playMode = usePlayMode((x) => x.on);

  useEffect(() => {
    setStep(0);
    setPlaying(false);
  }, [hand]);
  useEffect(() => {
    if (!playing) return;
    if (step >= last) {
      setPlaying(false);
      return;
    }
    const h = setTimeout(() => setStep((s) => s + 1), 700);
    return () => clearTimeout(h);
  }, [playing, step, last]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight") setStep((s) => Math.min(last, s + 1));
      if (e.key === "ArrowLeft") setStep((s) => Math.max(0, s - 1));
      if (e.key === " ") {
        e.preventDefault();
        setPlaying((p) => !p);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [last]);

  // Hero at the bottom: rotate seats so that the hero gets slot 0.
  const heroIdx = Math.max(0, hand.players.findIndex((p) => p.is_hero));
  const slots = Math.max(hand.max_seats, hand.players.length);
  const heroSeat = hand.players[heroIdx]?.seat ?? 1;
  const slotOf = (seat: number) => (((seat - heroSeat) % slots) + slots) % slots;

  const blinds = `${fmt(hand.small_blind)} / ${fmt(hand.big_blind)}${hand.ante ? ` (ante ${fmt(hand.ante)})` : ""}`;
  return (
    <div className="grid grid-cols-1 gap-d4 xl:grid-cols-[1fr_20rem]">
      <div className="flex flex-col gap-d3">
        <div className="flex flex-wrap items-center justify-between gap-d2 text-sm">
          <div>
            <div className="font-semibold">{hand.tournament?.name ?? hand.table_name}</div>
            <div className="text-muted">
              {formatDate(hand.played_at, i18n.language)} · {blinds}
              {hand.level != null && ` · ${t("replayer.level", { n: hand.level })}`} · #{hand.hand_no}
            </div>
          </div>
          <label className="flex items-center gap-d2">
            <input type="checkbox" checked={inBb} onChange={(e) => setInBb(e.target.checked)} />
            {t("replayer.in_bb")}
          </label>
        </div>
        <div
          className="relative aspect-[16/9] w-full rounded-[50%] border-8 border-[#5b3a1a] bg-[#0f5b3a] shadow-inner"
          data-testid="replayer-table"
        >
          <div className="absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-d2">
            <div className="flex gap-1" data-testid="board">
              {[0, 1, 2, 3, 4].map((i) => (
                <PlayingCard key={i} card={st.board[i]} hidden={!st.board[i]} />
              ))}
            </div>
            <div className="rounded-full bg-black/40 px-d3 py-1 text-sm text-white" data-testid="pot">
              {t("replayer.pot")} : {fmt(st.finished ? hand.total_pot - hand.rake : st.pot + st.seats.reduce((a, s) => a + s.bet, 0))}
            </div>
          </div>
          {hand.players.map((p, i) => {
            const s = st.seats[i];
            const pos = seatPos(slotOf(p.seat), slots);
            const acting = !st.finished && st.current >= 0 && hand.actions[st.current]?.player === i;
            return (
              <div
                key={p.seat}
                className="absolute flex -translate-x-1/2 -translate-y-1/2 cursor-pointer flex-col items-center gap-1"
                style={pos}
                data-testid="seat"
                title={t("notes.open")}
                onClick={() => setNotesFor(p.name)}
              >
                <div className="flex gap-0.5">
                  {[0, 1].map((k) => (
                    <PlayingCard key={k} card={p.hole_cards[k]} hidden={!s.showCards || !p.hole_cards[k]} size="sm" />
                  ))}
                </div>
                <div
                  className={cn(
                    "min-w-28 rounded-md border px-d2 py-1 text-center text-xs",
                    p.is_hero ? "border-primary" : "border-border",
                    acting ? "bg-primary text-primary-fg" : "bg-surface text-fg",
                    s.folded && "opacity-40",
                  )}
                >
                  <div className="truncate font-medium">
                    {p.name}
                    {hand.button_seat === p.seat && <span className="ml-1 rounded-full bg-white px-1 text-black">D</span>}
                  </div>
                  <div className="tabular-nums">
                    {p.position} · {fmt(s.stack)}
                  </div>
                  {s.last && <div className="text-[10px] uppercase opacity-80">{t(`replayer.action.${s.last}`)}</div>}
                  {st.finished && s.collected > 0 && (
                    <div className="font-semibold text-success">+{fmt(s.collected)}</div>
                  )}
                </div>
                {s.bet > 0 && <div className="rounded-full bg-yellow-400 px-2 text-xs font-semibold text-black">{fmt(s.bet)}</div>}
              </div>
            );
          })}
        </div>
        <div className="flex items-center justify-center gap-d2" data-testid="replayer-controls">
          <Button variant="outline" size="icon" aria-label={t("replayer.first")} onClick={() => setStep(0)}>
            <ChevronFirst className="h-4 w-4" />
          </Button>
          <Button variant="outline" size="icon" aria-label={t("replayer.prev")} onClick={() => setStep((s) => Math.max(0, s - 1))}>
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <Button size="icon" aria-label={playing ? t("replayer.pause") : t("replayer.play")} onClick={() => setPlaying((p) => !p)}>
            {playing ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
          </Button>
          <Button variant="outline" size="icon" aria-label={t("replayer.next")} onClick={() => setStep((s) => Math.min(last, s + 1))} data-testid="step-next">
            <ChevronRight className="h-4 w-4" />
          </Button>
          <Button variant="outline" size="icon" aria-label={t("replayer.last")} onClick={() => setStep(last)} data-testid="step-last">
            <ChevronLast className="h-4 w-4" />
          </Button>
          <span className="ml-d2 text-sm tabular-nums text-muted" data-testid="step">
            {step} / {last}
          </span>
          {!playMode && hand.players[heroIdx]?.hole_cards.length === 2 && (
            <Link className="ml-d3 text-sm text-primary underline" to={equityLink(hand, heroIdx, hand.board, stateAt(hand, last).seats)} data-testid="to-equity">
              {t("replayer.to_equity")}
            </Link>
          )}
        </div>
      </div>
      <div className="flex flex-col gap-d3">
      {notesFor && <NotesPanel room={hand.room} name={notesFor} />}
      <Card className="max-h-[36rem] overflow-y-auto p-d3 text-sm" data-testid="action-log">
        <ol className="flex flex-col gap-0.5">
          {hand.actions.map((a, i) => (
            <li
              key={i}
              className={cn("cursor-pointer rounded px-d2 py-0.5", i === st.current && "bg-primary text-primary-fg", i > st.current && "text-muted")}
              onClick={() => setStep(i + 1)}
            >
              {(i === 0 || hand.actions[i - 1].street !== a.street) && (
                <div className="mt-1 text-xs font-semibold uppercase text-muted">{t(`replayer.street.${a.street}`)}</div>
              )}
              {hand.players[a.player].name} {t(`replayer.action.${a.kind}`)}
              {a.amount > 0 && ` ${fmt(a.raise_to ?? a.amount)}`}
              {a.is_all_in && ` (${t("replayer.all_in")})`}
            </li>
          ))}
          <li
            className={cn("mt-1 cursor-pointer rounded px-d2 py-0.5 font-semibold", st.finished && "bg-primary text-primary-fg")}
            onClick={() => setStep(last)}
          >
            {t("replayer.result")}
          </li>
        </ol>
      </Card>
      </div>
    </div>
  );
}

export function ReplayerPage({ manifest }: { manifest: ModuleManifest }) {
  const { t } = useTranslation();
  const { handId } = useParams();
  const id = handId ? Number(handId) : null;
  const hand = useQuery({
    queryKey: ["hand", id],
    queryFn: async () => (await getBackend()).handGet(id!),
    enabled: id !== null,
  });
  return (
    <div className="mx-auto flex max-w-7xl flex-col gap-d4 p-d6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>
        {id !== null && (
          <Link className="text-sm text-primary underline" to="/m/replayer">
            {t("replayer.pick_other")}
          </Link>
        )}
      </div>
      {id === null ? (
        <>
          <p className="text-sm text-muted">{t("replayer.pick")}</p>
          <FilterBar />
          <HandsList />
        </>
      ) : hand.isError ? (
        <p className="text-danger">{String(hand.error)}</p>
      ) : hand.data ? (
        <Replayer hand={hand.data} />
      ) : null}
    </div>
  );
}
