/**
 * The overlay drawn over one poker table (a transparent, always-on-top,
 * click-through window). It only shows **historical** stats from imported
 * hands — never advice, equity or ranges for the current hand (fair-play).
 * Outside edit mode only the panels take the mouse (the backend follows the
 * cursor over the rectangles reported here): hover = detail popup,
 * double-click = the player's sheet in the app, right-click = a small menu.
 * In edit mode panels can be dragged and resized (corner handle), per table size.
 */
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
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
  scale,
  onMove,
  onResize,
  onMenu,
}: {
  seat: HudSeat;
  data: OverlayData;
  pos: [number, number];
  scale: number;
  onMove: (x: number, y: number) => void;
  /** Live (done = false) then final (done = true) panel scale. */
  onResize: (factor: number, done: boolean) => void;
  onMenu: (seat: HudSeat, x: number, y: number) => void;
}) {
  const { t } = useTranslation();
  const [drag, setDrag] = useState<{ dx: number; dy: number } | null>(null);
  const [live, setLive] = useState<[number, number] | null>(null);
  const [hover, setHover] = useState(false);
  const [resize, setResize] = useState<{ x: number; w: number } | null>(null);
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
      data-hud-hit
      style={{
        left: `${x * 100}%`,
        top: `${y * 100}%`,
        background: `rgba(10,12,16,${L.opacity ?? 0.85})`,
        fontSize: `${11 * scale}px`,
        outline: data.edit ? "1px dashed #3987e5" : undefined,
        cursor: data.edit ? "move" : "default",
      }}
      data-testid="hud-panel"
      data-slot={seat.slot}
      onPointerEnter={() => setHover(true)}
      onPointerLeave={() => setHover(false)}
      onDoubleClick={() => {
        if (!data.edit) void getBackend().then((b) => b.hudOpenPlayer(seat.name));
      }}
      onContextMenu={(e) => {
        e.preventDefault();
        onMenu(seat, e.clientX, e.clientY);
      }}
      onPointerDown={(e) => {
        if (!data.edit || !ref.current || e.button !== 0) return;
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
      {data.edit && (
        <div
          className="absolute -bottom-1 -right-1 h-3 w-3 cursor-nwse-resize rounded-sm bg-[#3987e5]"
          data-testid="hud-resize"
          onPointerDown={(e) => {
            e.stopPropagation();
            if (!ref.current) return;
            setResize({ x: e.clientX, w: ref.current.getBoundingClientRect().width });
            e.currentTarget.setPointerCapture(e.pointerId);
          }}
          onPointerMove={(e) => {
            if (resize) onResize(Math.max(0.4, (resize.w + e.clientX - resize.x) / resize.w), false);
          }}
          onPointerUp={(e) => {
            if (resize) onResize(Math.max(0.4, (resize.w + e.clientX - resize.x) / resize.w), true);
            setResize(null);
          }}
        />
      )}
      {hover && !drag && !resize && (
        <div
          className="absolute left-full top-0 z-10 ml-1 min-w-48 rounded bg-[rgba(10,12,16,0.95)] p-1.5 font-sans text-[11px]"
          data-testid="hud-popup"
          data-hud-hit
        >
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
          {!data.edit && <div className="mt-1 text-[10px] text-[#898781]">{t("hud.popup_help")}</div>}
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

  // Report the clickable areas (panels, popup, menu, edit bar) after each render.
  const root = useRef<HTMLDivElement>(null);
  const lastRects = useRef("");
  useLayoutEffect(() => {
    if (!root.current) return;
    const rects = Array.from(root.current.querySelectorAll("[data-hud-hit]")).map((el) => {
      const r = el.getBoundingClientRect();
      return [Math.round(r.left), Math.round(r.top), Math.round(r.width), Math.round(r.height)] as [number, number, number, number];
    });
    const key = JSON.stringify(rects);
    if (key === lastRects.current) return;
    lastRects.current = key;
    void getBackend().then((b) => b.hudPanelRects(windowId, rects));
  });
  const [menu, setMenu] = useState<{ seat: HudSeat; x: number; y: number } | null>(null);
  const [liveScale, setLiveScale] = useState<number | null>(null);
  const [width, setWidth] = useState(() => window.innerWidth);
  useEffect(() => {
    const on = () => setWidth(window.innerWidth);
    window.addEventListener("resize", on);
    return () => window.removeEventListener("resize", on);
  }, []);

  if (!data?.table) return null;
  const n = data.table.maxSeats;
  const L = data.layout;
  const positions = L.seats[String(n)] ?? [];
  const sizeScale = liveScale ?? L.panel_scale?.[String(n)] ?? 1;
  // Auto: panels follow the table window (reference 900 px wide).
  const auto = L.auto_scale === false ? 1 : Math.min(1.6, Math.max(0.6, width / 900));
  const scale = (L.font_scale ?? 1) * sizeScale * auto;
  const save = (slot: number) => (x: number, y: number) =>
    void getBackend()
      .then((b) => b.hudSaveSeat(n, slot, x, y))
      .then(load);
  const saveScale = (v: number) =>
    void getBackend()
      .then((b) => b.hudSaveScale(n, Math.min(2.5, Math.max(0.5, v))))
      .then(load)
      .then(() => setLiveScale(null));
  const onResize = (factor: number, done: boolean) => {
    const base = L.panel_scale?.[String(n)] ?? 1;
    const v = Math.min(2.5, Math.max(0.5, Math.round(base * factor * 100) / 100));
    if (done) saveScale(v);
    else setLiveScale(v);
  };
  const hideTable = () => void getBackend().then((b) => b.hudSetTableHidden(windowId, true));
  const btn = "rounded bg-white px-2 py-0.5 font-semibold text-[#1a1a19]";
  return (
    <div
      ref={root}
      className="fixed inset-0"
      data-testid="hud-overlay"
      style={{ outline: data.edit ? "2px solid #3987e5" : undefined }}
      onPointerDown={(e) => {
        if (menu && !(e.target as HTMLElement).closest("[data-hud-menu]")) setMenu(null);
      }}
    >
      {data.table.seats
        .filter((s) => !s.isHero || L.show_hero)
        .map((s) => (
          <Panel
            key={s.seat}
            seat={s}
            data={data}
            scale={scale}
            pos={positions[s.slot] ?? defaultPos(s.slot, n)}
            onMove={save(s.slot)}
            onResize={onResize}
            onMenu={(seat, x, y) => setMenu({ seat, x, y })}
          />
        ))}
      {menu && (
        <div
          className="absolute z-20 flex min-w-44 flex-col rounded bg-[rgba(10,12,16,0.97)] py-1 font-sans text-xs text-white shadow"
          style={{ left: Math.min(menu.x, width - 180), top: menu.y }}
          data-hud-hit
          data-hud-menu
          data-testid="hud-menu"
          onPointerLeave={() => setMenu(null)}
        >
          <div className="px-2 py-1 font-semibold">{menu.seat.name}</div>
          {[
            ["sheet", t("hud.menu_sheet"), () => void getBackend().then((b) => b.hudOpenPlayer(menu.seat.name))],
            ["edit", t("hud.table_edit"), () => void getBackend().then((b) => b.hudSetEditTable(windowId))],
            ["hide", t("hud.menu_hide"), hideTable],
          ].map(([k, label, run]) => (
            <button
              key={k as string}
              className="px-2 py-1 text-left hover:bg-[#3987e5]"
              data-testid={`hud-menu-${k as string}`}
              onClick={() => {
                setMenu(null);
                (run as () => void)();
              }}
            >
              {label as string}
            </button>
          ))}
        </div>
      )}
      {data.edit && (
        <div
          className="absolute left-1/2 top-1 flex -translate-x-1/2 flex-wrap items-center gap-2 rounded bg-[#3987e5] px-2 py-1 text-xs text-white shadow"
          data-testid="hud-edit-bar"
          data-hud-hit
        >
          {t("hud.edit_bar")}
          <label className="flex items-center gap-1">
            {t("hud.format")}
            <select
              className="rounded px-1 text-[#1a1a19]"
              value={data.seatsOverride ?? ""}
              data-testid="hud-format"
              onChange={(e) => {
                const v = e.target.value ? Number(e.target.value) : null;
                void getBackend().then((b) => b.hudSetTableSeats(windowId, v));
              }}
            >
              <option value="">{t("hud.format_auto")}</option>
              {[2, 3, 4, 5, 6, 7, 8, 9, 10].map((k) => (
                <option key={k} value={k}>
                  {t("hud.max", { n: k })}
                </option>
              ))}
            </select>
          </label>
          <span className="flex items-center gap-1">
            {t("hud.size")}
            <button className={btn} data-testid="hud-smaller" onClick={() => saveScale(sizeScale - 0.1)}>
              −
            </button>
            <span className="tabular-nums" data-testid="hud-scale">
              {Math.round(sizeScale * 100)} %
            </span>
            <button className={btn} data-testid="hud-bigger" onClick={() => saveScale(sizeScale + 0.1)}>
              +
            </button>
          </span>
          <button className={btn} data-testid="hud-edit-hide" onClick={hideTable}>
            {t("hud.table_hide")}
          </button>
          <button className={btn} data-testid="hud-edit-done" onClick={() => void getBackend().then((b) => b.hudSetEditTable(null))}>
            {t("hud.table_done")}
          </button>
        </div>
      )}
    </div>
  );
}
