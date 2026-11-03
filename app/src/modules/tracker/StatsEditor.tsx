import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Plus, Save, Trash2 } from "lucide-react";
import { getBackend, type JsonObject } from "@/lib/backend";
import { formatStat } from "@/lib/format";
import type { Preview, StatDef, StatMeta } from "@/lib/tracker";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { useFilter, useReport } from "./api";

const CUSTOM = "custom";
const FORMATS = ["pct0", "pct1", "dec1", "dec2", "int", "bb100"];
const COUNTERS_HELP =
  "cnt_hands, cnt_vpip, cnt_pfr, cnt_limp, opp_3bet, cnt_3bet, opp_fold_to_3bet, cnt_fold_to_3bet, opp_steal, cnt_steal, opp_fold_to_steal, cnt_fold_to_steal, cnt_saw_flop, opp_cbet_flop, cnt_cbet_flop, opp_fold_to_cbet_flop, cnt_fold_to_cbet_flop, cnt_wtsd, cnt_wsd, cnt_wwsf, cnt_postflop_bets_raises, cnt_postflop_calls, cnt_postflop_folds, cnt_allin_preflop, cnt_won_hand, net_won_bb";

interface Draft {
  file: string;
  id: string;
  isNew: boolean;
  def: StatDef;
}

/** Create and edit stats; the formula is validated and computed live. */
export function StatsEditor() {
  const { t, i18n } = useTranslation();
  const qc = useQueryClient();
  const filter = useFilter();
  const report = useReport(filter, "none").data;
  const [draft, setDraft] = useState<Draft | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const open = async (m: StatMeta) => {
    const file = (await (await getBackend()).configGet("stats", m.file)) as unknown as { stat: Record<string, StatDef> };
    setDraft({ file: m.file, id: m.id, isNew: false, def: { ...file.stat[m.id] } });
    setSaved(false);
  };
  const create = () => {
    setDraft({ file: CUSTOM, id: "", isNew: true, def: { name: "", formula: "", where: "", format: "pct1" } });
    setSaved(false);
  };

  useEffect(() => {
    if (!draft) return;
    const h = setTimeout(() => {
      const def = { ...draft.def, where: draft.def.where?.trim() ? draft.def.where : undefined };
      void getBackend()
        .then((b) => b.statPreview(def, filter))
        .then((p) => {
          setPreview(p);
          setError(null);
        })
        .catch((e: Error) => {
          setPreview(null);
          setError(e.message);
        });
    }, 250);
    return () => clearTimeout(h);
  }, [draft, filter]);

  const set = (patch: Partial<StatDef>) => draft && setDraft({ ...draft, def: { ...draft.def, ...patch } });
  const validId = draft && /^[a-z][a-z0-9_]{0,40}$/.test(draft.id);

  const save = async () => {
    if (!draft || !validId || error) return;
    const b = await getBackend();
    const files = await b.configList("stats");
    if (!files.includes(draft.file)) {
      await b.configSaveText("stats", draft.file, `# ${t("tracker.editor.custom_file_header")}\n\n[stat]\n`);
    }
    const file = (await b.configGet("stats", draft.file)) as unknown as { stat?: Record<string, StatDef> };
    const def: StatDef = { ...draft.def };
    if (!def.where?.trim()) delete def.where;
    if (!def.description) delete def.description;
    await b.configSaveValue("stats", draft.file, { ...file, stat: { ...(file.stat ?? {}), [draft.id]: def } } as unknown as JsonObject);
    await qc.invalidateQueries({ queryKey: ["db"] });
    setDraft({ ...draft, isNew: false });
    setSaved(true);
  };

  const remove = async () => {
    if (!draft || draft.isNew) return;
    const b = await getBackend();
    const file = (await b.configGet("stats", draft.file)) as unknown as { stat: Record<string, StatDef> };
    const { [draft.id]: _gone, ...rest } = file.stat;
    await b.configSaveValue("stats", draft.file, { ...file, stat: rest } as unknown as JsonObject);
    await qc.invalidateQueries({ queryKey: ["db"] });
    setDraft(null);
  };

  const lang = i18n.language;
  return (
    <div className="grid grid-cols-1 gap-d4 lg:grid-cols-[16rem_1fr]">
      <Card className="flex flex-col gap-1 p-d2" data-testid="stat-list">
        <Button size="sm" onClick={create} className="mb-d2" data-testid="new-stat">
          <Plus className="h-4 w-4" />
          {t("tracker.editor.new")}
        </Button>
        {report?.stats.map((m) => (
          <button
            key={m.id}
            className={cn("rounded px-d2 py-1 text-left text-sm hover:bg-surface-alt", draft?.id === m.id && "bg-surface-alt font-medium")}
            onClick={() => void open(m)}
          >
            {m.name}
            <span className="block text-xs text-muted">stats/{m.file}.toml</span>
          </button>
        ))}
      </Card>
      {draft ? (
        <Card className="flex flex-col gap-d3 p-d4" data-testid="stat-editor">
          <div className="grid grid-cols-1 gap-d3 md:grid-cols-2">
            <label className="flex flex-col gap-1 text-sm">
              {t("tracker.editor.id")}
              <Input
                value={draft.id}
                disabled={!draft.isNew}
                aria-invalid={!validId}
                placeholder="vpip_btn"
                onChange={(e) => setDraft({ ...draft, id: e.target.value })}
                data-testid="stat-id"
              />
            </label>
            <label className="flex flex-col gap-1 text-sm">
              {t("tracker.editor.name")}
              <Input value={draft.def.name} onChange={(e) => set({ name: e.target.value })} data-testid="stat-name" />
            </label>
          </div>
          <label className="flex flex-col gap-1 text-sm">
            {t("tracker.editor.formula")}
            <Input
              className="font-mono"
              value={draft.def.formula}
              aria-invalid={!!error}
              placeholder="cnt_vpip / cnt_hands"
              onChange={(e) => set({ formula: e.target.value })}
              data-testid="stat-formula"
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t("tracker.editor.where")}
            <Input
              className="font-mono"
              value={draft.def.where ?? ""}
              placeholder="position = 'BTN'"
              onChange={(e) => set({ where: e.target.value })}
              data-testid="stat-where"
            />
          </label>
          <label className="flex w-48 flex-col gap-1 text-sm">
            {t("tracker.editor.format")}
            <Select value={draft.def.format ?? "pct1"} onChange={(e) => set({ format: e.target.value })}>
              {FORMATS.map((f) => (
                <option key={f} value={f}>
                  {t(`tracker.editor.formats.${f}`)}
                </option>
              ))}
            </Select>
          </label>
          <div className="rounded border border-border p-d3 text-sm" data-testid="stat-preview" role="status">
            {error ? (
              <span className="text-danger">{error}</span>
            ) : preview ? (
              <span>
                {t("tracker.editor.preview")} : <strong>{formatStat(preview.value, draft.def.format ?? "pct1", lang)}</strong>{" "}
                <span className="text-muted">{t("tracker.sample", { n: preview.sample })}</span>
              </span>
            ) : (
              <span className="text-muted">…</span>
            )}
          </div>
          <details className="text-xs text-muted">
            <summary className="cursor-pointer">{t("tracker.editor.help_title")}</summary>
            <p className="mt-1">{t("tracker.editor.help")}</p>
            <p className="mt-1 font-mono">{COUNTERS_HELP}</p>
            <p className="mt-1">{t("tracker.editor.help_where")}</p>
          </details>
          <div className="flex items-center gap-d2">
            <Button onClick={() => void save()} disabled={!validId || !!error || !draft.def.name} data-testid="stat-save">
              <Save className="h-4 w-4" />
              {t("settings.save")}
            </Button>
            {!draft.isNew && (
              <Button variant="outline" onClick={() => void remove()}>
                <Trash2 className="h-4 w-4" />
                {t("settings.delete")}
              </Button>
            )}
            {saved && <span className="text-sm text-success">{t("tracker.editor.saved", { file: draft.file })}</span>}
          </div>
        </Card>
      ) : (
        <p className="text-sm text-muted">{t("tracker.editor.pick")}</p>
      )}
    </div>
  );
}
