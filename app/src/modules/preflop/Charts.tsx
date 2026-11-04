/** Chart library: folders of ranges per format and situation, edit, import / export. */
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { RangeEditor } from "@/components/range/RangeEditor";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { comboCount, tryParseRange } from "@/lib/range";
import { cn } from "@/lib/utils";

/** Suggested chart folders: empty situations to fill (nothing proprietary bundled). */
const FORMATS: { id: string; positions: string[] }[] = [
  { id: "cash 6max 100bb", positions: ["UTG", "HJ", "CO", "BTN", "SB", "BB"] },
  ...[10, 15, 20, 25, 40, 60, 100].map((d) => ({ id: `MTT ${d}bb`, positions: ["UTG", "HJ", "CO", "BTN", "SB", "BB"] })),
  { id: "Spin 3max 25bb", positions: ["BTN", "SB", "BB"] },
];

export function situations(positions: string[]): string[] {
  const out: string[] = [];
  positions.forEach((p, i) => {
    if (p !== "BB") out.push(`${p}/open`);
    for (const v of positions.slice(0, i)) {
      out.push(`${p} vs ${v}/3bet`, `${p} vs ${v}/call`);
    }
    if (p !== "BB") out.push(`${p} vs 3bet/4bet`, `${p} vs 3bet/call`);
  });
  if (positions.includes("SB") && positions.includes("BB")) out.push("BB vs limp/raise");
  return out;
}

export function Charts() {
  const { t } = useTranslation();
  const [names, setNames] = useState<string[]>([]);
  const [folder, setFolder] = useState<string>("");
  const [sel, setSel] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [bundle, setBundle] = useState("");
  const [importPrefix, setImportPrefix] = useState("");
  const [overwrite, setOverwrite] = useState(false);

  const reload = () => void getBackend().then((b) => b.rangesList()).then(setNames);
  useEffect(reload, []);

  const folders = useMemo(() => {
    const s = new Set<string>();
    for (const n of names) {
      const parts = n.split("/");
      if (parts.length > 1) s.add(parts[0]);
    }
    return [...s].sort();
  }, [names]);
  const inFolder = names.filter((n) => (folder ? n.startsWith(`${folder}/`) : !n.includes("/")));

  const open = (name: string) =>
    void getBackend()
      .then((b) => b.rangeLoad(name))
      .then((x) => {
        setSel(name);
        setText(x.trim());
        setMsg(null);
      });
  const save = () =>
    sel &&
    void getBackend()
      .then((b) => b.rangeSave(sel, text))
      .then(() => setMsg(t("preflop.saved", { name: sel })))
      .catch((e: Error) => setMsg(e.message));
  const create = (id: string) => {
    const f = FORMATS.find((x) => x.id === id);
    if (!f) return;
    const b = ["# Local Poker Suite ranges v1", ...situations(f.positions).map((s) => `[${s}]\n`)].join("\n");
    void getBackend()
      .then((be) => be.rangesImport(b, id, false))
      .then((k) => {
        setMsg(t("preflop.created", { n: k, folder: id }));
        setFolder(id);
        reload();
      });
  };

  return (
    <div className="grid grid-cols-1 gap-d4 lg:grid-cols-[18rem_1fr]">
      <Card className="flex flex-col gap-d3 p-d3 text-sm">
        <label className="flex flex-col gap-1">
          {t("preflop.new_format")}
          <Select value="" onChange={(e) => create(e.target.value)} data-testid="charts-create">
            <option value="">{t("preflop.pick_format")}</option>
            {FORMATS.map((f) => (
              <option key={f.id} value={f.id}>
                {f.id}
              </option>
            ))}
          </Select>
        </label>
        <ul className="flex flex-col gap-0.5" data-testid="charts-folders">
          {[...folders, ""].map((f) => (
            <li key={f || "_root"}>
              <button className={cn("w-full rounded px-d2 py-1 text-left hover:bg-surface-alt", folder === f && "bg-surface-alt font-medium")} onClick={() => setFolder(f)}>
                {f || t("preflop.root")}
              </button>
            </li>
          ))}
        </ul>
        <ul className="flex max-h-[28rem] flex-col gap-0.5 overflow-y-auto border-t border-border pt-d2" data-testid="charts-list">
          {inFolder.length === 0 && <li className="text-muted">{t("preflop.empty_folder")}</li>}
          {inFolder.map((n) => (
            <li key={n}>
              <button className={cn("w-full truncate rounded px-d2 py-0.5 text-left text-xs hover:bg-surface-alt", sel === n && "bg-primary text-primary-fg")} onClick={() => open(n)} data-testid="chart-item">
                {folder ? n.slice(folder.length + 1) : n}
              </button>
            </li>
          ))}
        </ul>
      </Card>
      <div className="flex flex-col gap-d4">
        <Card className="flex flex-col gap-d3 p-d4">
          {sel ? (
            <>
              <div className="flex flex-wrap items-center gap-d2">
                <h2 className="font-semibold" data-testid="chart-name">
                  {sel}
                </h2>
                <span className="text-sm text-muted">{t("range.combos", { n: comboCount(tryParseRange(text) ?? new Float32Array(1326)).toFixed(0) })}</span>
                <Button size="sm" onClick={save} disabled={!tryParseRange(text)} data-testid="chart-save">
                  {t("preflop.save")}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() =>
                    void getBackend()
                      .then((b) => b.rangeDelete(sel))
                      .then(() => {
                        setSel(null);
                        reload();
                      })
                  }
                >
                  {t("range.lib_delete")}
                </Button>
              </div>
              <RangeEditor value={text} onChange={setText} testId="chart-editor" />
            </>
          ) : (
            <p className="text-sm text-muted">{t("preflop.pick_chart")}</p>
          )}
          {msg && (
            <p className="text-sm text-muted" data-testid="charts-msg">
              {msg}
            </p>
          )}
        </Card>
        <Card className="flex flex-col gap-d3 p-d4 text-sm">
          <h2 className="font-semibold">{t("preflop.transfer")}</h2>
          <div className="flex flex-wrap gap-d2">
            <Button size="sm" variant="outline" onClick={() => void getBackend().then((b) => b.rangesExport(folder)).then(setBundle)} data-testid="charts-export">
              {t("preflop.export", { folder: folder || t("preflop.root") })}
            </Button>
            <Button size="sm" variant="outline" onClick={() => void getBackend().then((b) => b.rangesExportFile(folder)).then((p) => setMsg(t("preflop.exported", { path: p })))}>
              {t("preflop.export_file")}
            </Button>
            <label className="inline-flex cursor-pointer items-center rounded-md border border-border px-d3 text-sm hover:bg-surface-alt">
              {t("preflop.open_file")}
              <input
                type="file"
                accept=".lpsranges,.txt"
                className="hidden"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f) void f.text().then(setBundle);
                }}
              />
            </label>
          </div>
          <textarea
            className="min-h-32 w-full rounded-md border border-border bg-bg p-d2 font-mono text-xs"
            value={bundle}
            onChange={(e) => setBundle(e.target.value)}
            placeholder={"# Local Poker Suite ranges v1\n[BTN/open]\n22+,A2s+,K9s+\n[BB vs BTN/3bet]\nQQ+,AKs,A5s:0.5"}
            data-testid="charts-bundle"
          />
          <div className="flex flex-wrap items-end gap-d2">
            <label className="flex flex-col gap-1">
              {t("preflop.import_into")}
              <Input className="w-64" value={importPrefix} onChange={(e) => setImportPrefix(e.target.value)} placeholder={t("preflop.root")} data-testid="charts-prefix" />
            </label>
            <label className="flex items-center gap-1">
              <input type="checkbox" checked={overwrite} onChange={(e) => setOverwrite(e.target.checked)} />
              {t("preflop.overwrite")}
            </label>
            <Button
              size="sm"
              disabled={!bundle.trim()}
              onClick={() =>
                void getBackend()
                  .then((b) => b.rangesImport(bundle, importPrefix, overwrite))
                  .then((k) => {
                    setMsg(t("preflop.imported", { n: k }));
                    reload();
                  })
                  .catch((e: Error) => setMsg(e.message))
              }
              data-testid="charts-import"
            >
              {t("preflop.import")}
            </Button>
          </div>
          <p className="text-xs text-muted">{t("preflop.transfer_help")}</p>
        </Card>
      </div>
    </div>
  );
}
