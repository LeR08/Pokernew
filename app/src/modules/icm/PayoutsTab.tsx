/** Payout structures, entered by the user (payouts/*.toml). */
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend, type JsonObject } from "@/lib/backend";
import type { Payouts } from "@/lib/icm";
import { parsePlaces, slug, usePayouts } from "./payouts";

const empty = (): Payouts => ({ name: "", unit: "money", places: [0, 0, 0] });

export function PayoutsTab() {
  const { t } = useTranslation();
  const { list } = usePayouts();
  const [id, setId] = useState<string | null>(null);
  const [p, setP] = useState<Payouts>(empty());
  const [csv, setCsv] = useState("");
  const [msg, setMsg] = useState<string | null>(null);

  const edit = (x: { id: string; p: Payouts }) => {
    setId(x.id);
    setP({ ...x.p, places: [...x.p.places] });
    setMsg(null);
  };
  const setPlace = (i: number, v: number) => setP({ ...p, places: p.places.map((x, j) => (j === i ? v : x)) });
  const total = p.places.reduce((a, b) => a + b, 0);

  const save = () => {
    const target = id ?? slug(p.name);
    const value: Record<string, unknown> = { name: p.name.trim(), unit: p.unit, places: p.places };
    if (p.players) value.players = p.players;
    if (p.unit === "percent" && p.prize_pool) value.prize_pool = p.prize_pool;
    void getBackend()
      .then((b) => b.configSaveValue("payouts", target, value as JsonObject))
      .then(() => {
        setId(target);
        setMsg(t("icm.payouts.saved", { name: p.name }));
      })
      .catch((e: Error) => setMsg(e.message));
  };
  const remove = () => {
    if (!id) return;
    void getBackend()
      .then((b) => b.configDelete("payouts", id))
      .then(() => {
        setId(null);
        setP(empty());
      });
  };

  return (
    <div className="grid grid-cols-1 gap-d4 lg:grid-cols-[16rem_1fr]">
      <Card className="flex flex-col gap-d2 p-d3">
        <Button size="sm" variant="outline" onClick={() => (setId(null), setP(empty()), setMsg(null))} data-testid="payouts-new">
          <Plus className="h-4 w-4" />
          {t("icm.payouts.new")}
        </Button>
        {list.length === 0 && <p className="text-sm text-muted">{t("icm.payouts.none")}</p>}
        <ul className="flex flex-col gap-1 text-sm" data-testid="payouts-list">
          {list.map((x) => (
            <li key={x.id}>
              <button className={`w-full rounded px-d2 py-1 text-left hover:bg-surface-alt ${x.id === id ? "bg-surface-alt font-medium" : ""}`} onClick={() => edit(x)}>
                {x.p.name}
                <span className="block text-xs text-muted">{t("icm.payouts.places_n", { n: x.p.places.length })}</span>
              </button>
            </li>
          ))}
        </ul>
      </Card>
      <Card className="flex flex-col gap-d3 p-d4">
        <p className="text-sm text-muted">{t("icm.payouts.help")}</p>
        <div className="flex flex-wrap gap-d3">
          <label className="flex min-w-64 flex-1 flex-col gap-1 text-sm">
            {t("schema.payouts.name")}
            <Input value={p.name} onChange={(e) => setP({ ...p, name: e.target.value })} data-testid="payouts-name" placeholder="Space KO 1 164 joueurs" />
          </label>
          <label className="flex w-36 flex-col gap-1 text-sm">
            {t("schema.payouts.players")}
            <Input type="number" min={2} value={p.players ?? ""} onChange={(e) => setP({ ...p, players: e.target.value ? Number(e.target.value) : undefined })} />
          </label>
          <label className="flex w-40 flex-col gap-1 text-sm">
            {t("schema.payouts.unit")}
            <Select value={p.unit} onChange={(e) => setP({ ...p, unit: e.target.value as Payouts["unit"] })} data-testid="payouts-unit">
              <option value="money">{t("icm.payouts.money")}</option>
              <option value="percent">{t("icm.payouts.percent")}</option>
            </Select>
          </label>
          {p.unit === "percent" && (
            <label className="flex w-36 flex-col gap-1 text-sm">
              {t("schema.payouts.prize_pool")}
              <Input type="number" min={0} value={p.prize_pool ?? ""} onChange={(e) => setP({ ...p, prize_pool: e.target.value ? Number(e.target.value) : undefined })} />
            </label>
          )}
        </div>
        <table className="w-full max-w-md text-sm" data-testid="payouts-places">
          <thead className="text-left text-xs text-muted">
            <tr>
              <th className="py-1">{t("icm.payouts.place")}</th>
              <th>{p.unit === "money" ? t("icm.payouts.prize") : "%"}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {p.places.map((x, i) => (
              <tr key={i}>
                <td className="py-0.5">{i + 1}</td>
                <td>
                  <Input
                    type="number"
                    min={0}
                    step="any"
                    value={x}
                    onChange={(e) => setPlace(i, Number(e.target.value))}
                    className="w-36"
                    data-testid="payouts-place"
                    aria-label={t("icm.payouts.place_n", { n: i + 1 })}
                  />
                </td>
                <td>
                  <Button size="icon" variant="ghost" aria-label={t("icm.payouts.remove")} onClick={() => setP({ ...p, places: p.places.filter((_, j) => j !== i) })} disabled={p.places.length <= 1}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <div className="flex flex-wrap items-center gap-d3 text-sm">
          <Button size="sm" variant="outline" onClick={() => setP({ ...p, places: [...p.places, 0] })} data-testid="payouts-add">
            <Plus className="h-4 w-4" />
            {t("icm.payouts.add")}
          </Button>
          <span className="text-muted">
            {t("icm.payouts.total")} : {p.unit === "money" ? `${total.toFixed(2)} €` : `${total.toFixed(1)} %`}
          </span>
          {p.unit === "percent" && Math.abs(total - 100) > 0.05 && <span className="text-warning">{t("icm.payouts.not_100")}</span>}
        </div>
        <details className="text-sm">
          <summary className="cursor-pointer">{t("icm.payouts.import")}</summary>
          <textarea
            className="mt-d2 min-h-24 w-full rounded-md border border-border bg-bg p-d2 font-mono text-xs"
            value={csv}
            onChange={(e) => setCsv(e.target.value)}
            placeholder={"place;gain\n1;150\n2;100\n3;70"}
            data-testid="payouts-csv"
          />
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              const xs = parsePlaces(csv);
              if (xs) setP({ ...p, places: xs });
              else setMsg(t("icm.payouts.csv_error"));
            }}
            data-testid="payouts-csv-apply"
          >
            {t("icm.payouts.csv_apply")}
          </Button>
        </details>
        <div className="flex gap-d2">
          <Button onClick={save} disabled={!p.name.trim() || p.places.length === 0} data-testid="payouts-save">
            {t("icm.payouts.save")}
          </Button>
          {id && (
            <Button variant="ghost" onClick={remove}>
              {t("icm.payouts.delete")}
            </Button>
          )}
        </div>
        {msg && (
          <p className="text-sm text-muted" data-testid="payouts-msg">
            {msg}
          </p>
        )}
      </Card>
    </div>
  );
}
