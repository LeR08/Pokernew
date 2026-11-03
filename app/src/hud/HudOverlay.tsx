/**
 * The overlay drawn over one poker table (a transparent, always-on-top,
 * click-through window). It only shows **historical** stats from imported
 * hands — never advice, equity or ranges for the current hand (fair-play).
 * In edit mode the window takes the mouse: panels can be dragged (positions
 * saved to the layout file) and hovering a panel opens its detail popup.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { getBackend } from "@/lib/backend";
import type { HudSeat, OverlayData, StatMeta } from "@/lib/tracker";
import { COLOUR_HEX } from "@/modules/replayer/NotesPanel";

function short(v: number | null | undefined, format: string): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return "-";
  if (format.startsWith("pct")) return String(Math.round(v * 100));
  if (format === "bb100") return v.toFixed(0);
  return v.toFixed(1);
}

function defaultPos(slot: number, n: number): [number, number] {
  const a = Math.PI / 2 + (2 * Math.PI * slot) / n;
  return [0.42 + 0.4 * Math.cos(a), 0.42 + 0.36 * Math.sin(a)];
}

function Panel({
  seat,
  data,
  pos,
  onMove,
}: {
  seat: HudSeat;
  data: OverlayData;
  pos: [number, number];
  onMove: (x: number, y: number) => void;
}) {
  const { t } = useTranslation();
  const [drag, setDrag] = useState<{ dx: number; dy: number } | null>(null);
  const [live, setLive] = useState<[number, number] | null>(null);
  const [hover, setHover] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const L = data.layout;
  const meta = (id: string): StatMeta | undefined => data.stats.find((s) => s.id === id);
  const value = (id: string) => seat.stats.find((s) => s.id === id);
  const dim = seat.hands < (L.min_hands ?? 0);
  const [x, y] = live ?? pos;

  return (
    <div
      ref={ref}
      className="absolute select-none rounded px-1 py-0.5 font-mono leading-tight text-white"
      style={{
        left: `${x * 100}%`,
        top: `${y * 100}%`,
        background: `rgba(10,12,16,${L.opacity ?? 0.85})`,
        fontSize: `${11 * (L.font_scale ?? 1)}px`,
        outline: data.edit ? "1px dashed #3987e5" : undefined,
        cursor: data.edit ? "move" : "default",
      }}
      data-testid="hud-panel"
      data-slot={seat.slot}
      onPointerEnter={() => setHover(true)}
      onPointerLeave={() => setHover(false)}
      onPointerDown={(e) => {
        if (!data.edit || !ref.current) return;
        const r = ref.current.getBoundingClientRect();
        setDrag({ dx: e.clientX - r.left, dy: e.clientY - r.top });
        ref.current.setPointerCapture(e.pointerId);
      }}
      onPointerMove={(e) => {
        if (!drag) return;
        setLive([(e.clientX - drag.dx) / window.innerWidth, (e.clientY - drag.dy) / window.innerHeight]);
      }}
      onPointerUp={() => {
        if (drag && live) onMove(Math.max(0, Math.min(1, live[0])), Math.max(0, Math.min(1, live[1])));
        setDrag(null);
      }}
    >
      <div className="flex items-center gap-1">
        {seat.colour && <span className="inline-block h-2 w-2 rounded-full" style={{ background: COLOUR_HEX[seat.colour] ?? seat.colour }} />}
        <span className="max-w-28 truncate font-sans font-semibold">{seat.name}</span>
        {seat.notes.length > 0 && <span title={seat.notes.join("\n")}>✎</span>}
      </div>
      <div className={dim ? "opacity-60" : undefined}>
        {L.rows.map((row, i) => (
          <div key={i} className="flex gap-1.5">
            {row.map((id) =>
              id === "hands" ? (
                <span key={id} className="text-[#c3c2b7]" title={t("hud.hands")}>
                  {seat.hands}
                </span>
              ) : (
                <span key={id} title={meta(id)?.name ?? id}>
                  {short(value(id)?.value, meta(id)?.format ?? "pct0")}
                </span>
              ),
            )}
          </div>
        ))}
      </div>
      {hover && data.edit && !drag && (
        <div className="absolute left-full top-0 z-10 ml-1 min-w-48 rounded bg-[rgba(10,12,16,0.95)] p-1.5 font-sans text-[11px]" data-testid="hud-popup">
          <div className="mb-1 font-semibold">
            {seat.name} · {t("hud.hands_n", { n: seat.hands })}
          </div>
          <table className="w-full">
            <tbody>
              {(L.popup ?? []).map((id) => (
                <tr key={id}>
                  <td className="pr-2 text-[#c3c2b7]">{meta(id)?.name ?? id}</td>
                  <td className="text-right font-mono">{short(value(id)?.value, meta(id)?.format ?? "pct0")}</td>
                  <td className="pl-1 text-right text-[#898781]">{value(id)?.sample ?? 0}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {seat.notes.map((n, i) => (
            <div key={i} className="mt-1 border-t border-white/10 pt-1">
              {n}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export function HudOverlay() {
  const { t } = useTranslation();
  const { id = "0" } = useParams();
  const windowId = Number(id);
  const [data, setData] = useState<OverlayData | null>(null);

  const load = useCallback(() => {
    void getBackend()
      .then((b) => b.hudOverlay(windowId))
      .then(setData)
      .catch(() => undefined);
  }, [windowId]);

  useEffect(() => {
    // The overlay window is transparent: no page background.
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    load();
    let off: (() => void) | undefined;
    void getBackend().then((b) => {
      const offHud = b.onHudEvent((k) => {
        if (k === "hud-refresh" || k === "hud-edit") load();
      });
      // Layout edited (settings, HUD page, text editor): redraw.
      const offCfg = b.onConfigEvent((e) => {
        if (e.kind === "changed" && (e.family === "hud" || e.family === "stats" || e.family === "profiles")) load();
      });
      off = () => {
        offHud();
        offCfg();
      };
    });
    return () => off?.();
  }, [load]);

  if (!data?.table) return null;
  const n = data.table.maxSeats;
  const positions = data.layout.seats[String(n)] ?? [];
  const save = (slot: number) => (x: number, y: number) =>
    void getBackend()
      .then((b) => b.hudSaveSeat(n, slot, x, y))
      .then(load);
  return (
    <div className="fixed inset-0" data-testid="hud-overlay" style={{ outline: data.edit ? "2px solid #3987e5" : undefined }}>
      {data.table.seats
        .filter((s) => !s.isHero || data.layout.show_hero)
        .map((s) => (
          <Panel key={s.seat} seat={s} data={data} pos={positions[s.slot] ?? defaultPos(s.slot, n)} onMove={save(s.slot)} />
        ))}
      {data.edit && (
        <div className="absolute left-1/2 top-1 flex -translate-x-1/2 items-center gap-2 rounded bg-[#3987e5] px-2 py-1 text-xs text-white shadow" data-testid="hud-edit-bar">
          {t("hud.edit_bar")}
          <button
            className="rounded bg-white px-2 py-0.5 font-semibold text-[#1a1a19]"
            data-testid="hud-edit-done"
            onClick={() => void getBackend().then((b) => b.hudSetEditTable(null))}
          >
            {t("hud.table_done")}
          </button>
        </div>
      )}
    </div>
  );
}
