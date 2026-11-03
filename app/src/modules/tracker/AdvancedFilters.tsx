import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SlidersHorizontal, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input, Select } from "@/components/ui/input";
import { ALL_CLASSES, HAND_GROUPS, parseClasses, TEXTURE } from "@/lib/tracker";
import { activeCount, useTracker, type Advanced } from "@/stores/tracker";
import { cn } from "@/lib/utils";

const POTS = ["limped", "srp", "3bet", "4bet"] as const;
const TEXTURES = Object.keys(TEXTURE) as (keyof typeof TEXTURE)[];

function num(v: string): number | null {
  const n = Number(v.replace(",", "."));
  return v.trim() === "" || !Number.isFinite(n) ? null : n;
}

/** "More filters": cards, pot type, stack depth, street, flop texture, showdown… */
export function AdvancedFilters() {
  const { t } = useTranslation();
  const adv = useTracker((s) => s.adv);
  const setAdv = useTracker((s) => s.setAdv);
  const reset = useTracker((s) => s.resetAdv);
  const [open, setOpen] = useState(false);
  const [classes, setClasses] = useState(adv.handClasses.join(", "));
  const n = activeCount(adv);
  const toggle = <K extends "potTypes">(key: K, v: string) =>
    setAdv({ [key]: adv[key].includes(v) ? adv[key].filter((x) => x !== v) : [...adv[key], v] } as Partial<Advanced>);
  const addGroup = (g: string) => {
    const next = Array.from(new Set([...adv.handClasses, ...ALL_CLASSES.filter(HAND_GROUPS[g])]));
    setAdv({ handClasses: next });
    setClasses(next.join(", "));
  };

  return (
    <div className="flex flex-col gap-d2">
      <div className="flex flex-wrap items-center gap-d2">
        <Button variant="outline" size="sm" onClick={() => setOpen((o) => !o)} aria-expanded={open} data-testid="more-filters">
          <SlidersHorizontal className="h-4 w-4" />
          {t("filters.more")}
          {n > 0 && <span className="rounded-full bg-primary px-1.5 text-xs text-primary-fg">{n}</span>}
        </Button>
        <Select
          className="w-52"
          value={adv.who}
          onChange={(e) => setAdv({ who: e.target.value as Advanced["who"] })}
          data-testid="filter-who"
          aria-label={t("filters.who")}
        >
          <option value="hero">{t("filters.who_hero")}</option>
          <option value="opponents">{t("filters.who_opponents")}</option>
        </Select>
        {n > 0 && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              reset();
              setClasses("");
            }}
          >
            <X className="h-4 w-4" />
            {t("filters.reset")}
          </Button>
        )}
      </div>
      {open && (
        <div className="grid grid-cols-1 gap-d3 rounded-lg border border-border bg-surface p-d3 text-sm md:grid-cols-2 xl:grid-cols-3" data-testid="advanced-panel">
          <fieldset className="flex flex-col gap-d1">
            <legend className="mb-1 font-medium">{t("filters.cards")}</legend>
            <Input
              placeholder="AKs, QQ, A5s"
              value={classes}
              onChange={(e) => setClasses(e.target.value)}
              onBlur={() => setAdv({ handClasses: parseClasses(classes) })}
              data-testid="filter-classes"
            />
            <div className="flex flex-wrap gap-1">
              {Object.keys(HAND_GROUPS).map((g) => (
                <button key={g} className="rounded border border-border px-1.5 py-0.5 text-xs hover:bg-surface-alt" onClick={() => addGroup(g)}>
                  + {t(`filters.groups.${g}`)}
                </button>
              ))}
            </div>
            {adv.handClasses.length > 0 && <span className="text-xs text-muted">{t("filters.classes_count", { n: adv.handClasses.length })}</span>}
          </fieldset>
          <fieldset className="flex flex-col gap-d1">
            <legend className="mb-1 font-medium">{t("filters.pot")}</legend>
            {POTS.map((p) => (
              <label key={p} className="flex items-center gap-d2">
                <input type="checkbox" checked={adv.potTypes.includes(p)} onChange={() => toggle("potTypes", p)} data-testid={`pot-${p}`} />
                {t(`filters.pots.${p}`)}
              </label>
            ))}
          </fieldset>
          <fieldset className="flex flex-col gap-d1">
            <legend className="mb-1 font-medium">{t("filters.stack")}</legend>
            <div className="flex items-center gap-d2">
              <Input className="w-20" inputMode="decimal" placeholder="min" defaultValue={adv.stackMin ?? ""} onBlur={(e) => setAdv({ stackMin: num(e.target.value) })} />
              —
              <Input className="w-20" inputMode="decimal" placeholder="max" defaultValue={adv.stackMax ?? ""} onBlur={(e) => setAdv({ stackMax: num(e.target.value) })} data-testid="stack-max" />
              {t("filters.bb")}
            </div>
            <legend className="mb-1 mt-d2 font-medium">{t("filters.buy_in")}</legend>
            <div className="flex items-center gap-d2">
              <Input className="w-20" inputMode="decimal" placeholder="min" defaultValue={adv.buyInMin ?? ""} onBlur={(e) => setAdv({ buyInMin: num(e.target.value) })} />
              —
              <Input className="w-20" inputMode="decimal" placeholder="max" defaultValue={adv.buyInMax ?? ""} onBlur={(e) => setAdv({ buyInMax: num(e.target.value) })} />
              {t("filters.eur")}
            </div>
          </fieldset>
          <fieldset className="flex flex-col gap-d1">
            <legend className="mb-1 font-medium">{t("filters.street")}</legend>
            <Select value={adv.street} onChange={(e) => setAdv({ street: e.target.value as Advanced["street"] })}>
              <option value="">{t("filters.any")}</option>
              <option value="flop">{t("filters.streets.flop")}</option>
              <option value="turn">{t("filters.streets.turn")}</option>
              <option value="river">{t("filters.streets.river")}</option>
            </Select>
          </fieldset>
          <fieldset className="flex flex-col gap-d1">
            <legend className="mb-1 font-medium">{t("filters.texture")}</legend>
            <div className="flex flex-wrap gap-1">
              {TEXTURES.map((k) => {
                const on = (adv.texture & TEXTURE[k]) !== 0;
                return (
                  <button
                    key={k}
                    className={cn("rounded border px-1.5 py-0.5 text-xs", on ? "border-primary bg-primary text-primary-fg" : "border-border hover:bg-surface-alt")}
                    onClick={() => setAdv({ texture: adv.texture ^ TEXTURE[k] })}
                    aria-pressed={on}
                  >
                    {t(`filters.textures.${k}`)}
                  </button>
                );
              })}
            </div>
            <span className="text-xs text-muted">{t("filters.texture_help")}</span>
          </fieldset>
          <fieldset className="flex flex-col gap-d1">
            <legend className="mb-1 font-medium">{t("filters.other")}</legend>
            <label className="flex items-center justify-between gap-d2">
              {t("filters.showdown")}
              <Select className="w-28" value={adv.showdown} onChange={(e) => setAdv({ showdown: e.target.value as Advanced["showdown"] })}>
                <option value="">{t("filters.any")}</option>
                <option value="yes">{t("filters.yes")}</option>
                <option value="no">{t("filters.no")}</option>
              </Select>
            </label>
            <label className="flex items-center justify-between gap-d2">
              {t("filters.allin_pre")}
              <Select className="w-28" value={adv.allinPreflop} onChange={(e) => setAdv({ allinPreflop: e.target.value as Advanced["allinPreflop"] })}>
                <option value="">{t("filters.any")}</option>
                <option value="yes">{t("filters.yes")}</option>
                <option value="no">{t("filters.no")}</option>
              </Select>
            </label>
          </fieldset>
        </div>
      )}
    </div>
  );
}
