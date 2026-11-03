import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Trash2 } from "lucide-react";
import { getBackend } from "@/lib/backend";
import { formatDate } from "@/lib/importApi";
import { euros, int, resultClass } from "@/lib/format";
import type { Tx } from "@/lib/tracker";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { LineChart } from "../tracker/LineChart";
import type { ModuleManifest } from "../types";

const KINDS: Tx["kind"][] = ["deposit", "withdrawal", "bonus", "adjustment"];

export function BankrollPage({ manifest }: { manifest: ModuleManifest }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const qc = useQueryClient();
  const b = useQuery({ queryKey: ["db", "bankroll"], queryFn: async () => (await getBackend()).bankrollGet() }).data;
  const txs = useQuery({ queryKey: ["db", "bankroll-tx"], queryFn: async () => (await getBackend()).bankrollTxList() }).data ?? [];
  const [kind, setKind] = useState<Tx["kind"]>("deposit");
  const [amount, setAmount] = useState("");
  const [date, setDate] = useState(() => new Date().toISOString().slice(0, 10));
  const [note, setNote] = useState("");
  const cents = Math.round(Number(amount.replace(",", ".")) * 100);
  const valid = Number.isFinite(cents) && cents > 0;

  const add = async () => {
    if (!valid) return;
    await (await getBackend()).bankrollTxAdd({ room: "winamax", at: Math.floor(new Date(date).getTime() / 1000), kind, amountCents: cents, note });
    setAmount("");
    setNote("");
    await qc.invalidateQueries({ queryKey: ["db"] });
  };
  const del = async (id: number) => {
    await (await getBackend()).bankrollTxDelete(id);
    await qc.invalidateQueries({ queryKey: ["db"] });
  };

  const fmt = useMemo(() => (v: number) => euros(Math.round(v * 100), lang, false), [lang]);
  const lines = useMemo(() => (b ? [{ name: t("bankroll.balance"), data: b.balanceEur }] : []), [b, t]);
  const x = useMemo(() => (b ? b.t.map((_, i) => i + 1) : []), [b]);
  const buyIns = b?.avgBuyInCents ? b.balanceCents / b.avgBuyInCents : null;

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-d4 p-d6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>
        <Link className="text-sm text-primary underline" to="/m/tracker/sessions">
          {t("bankroll.see_sessions")}
        </Link>
      </div>
      {b && (
        <div className="grid grid-cols-2 gap-d3 md:grid-cols-5" data-testid="bankroll-totals">
          {[
            [t("bankroll.balance"), euros(b.balanceCents, lang, false), "text-fg", "bankroll-balance"],
            [t("bankroll.deposits"), euros(b.depositsCents, lang, false), "", ""],
            [t("bankroll.withdrawals"), euros(b.withdrawalsCents, lang, false), "", ""],
            [t("bankroll.cash"), euros(b.cashCents, lang), resultClass(b.cashCents), ""],
            [t("bankroll.tournaments"), euros(b.tournamentsCents, lang), resultClass(b.tournamentsCents), ""],
          ].map(([label, value, cls, id]) => (
            <Card key={label} className="flex flex-col gap-d1 p-d3">
              <span className="text-xs text-muted">{label}</span>
              <span className={`text-xl font-semibold tabular-nums ${cls}`} data-testid={id || undefined}>
                {value}
              </span>
            </Card>
          ))}
        </div>
      )}
      {buyIns !== null && (
        <p className={`text-sm ${buyIns < 30 ? "text-warning" : "text-muted"}`} data-testid="buyin-rule">
          {t(buyIns < 30 ? "bankroll.rule_low" : "bankroll.rule_ok", { n: int(buyIns, lang), avg: euros(b!.avgBuyInCents!, lang, false) })}
        </p>
      )}
      {b && b.t.length > 1 && (
        <Card className="p-d4">
          <h2 className="mb-d2 font-semibold">{t("bankroll.curve")}</h2>
          <LineChart x={x} lines={lines} xLabel={t("bankroll.events")} format={fmt} testId="bankroll-graph" />
        </Card>
      )}
      <Card className="flex flex-col gap-d3 p-d4">
        <h2 className="font-semibold">{t("bankroll.movements")}</h2>
        <div className="flex flex-wrap items-end gap-d2" data-testid="tx-form">
          <label className="flex flex-col gap-1 text-sm">
            {t("bankroll.kind")}
            <Select value={kind} onChange={(e) => setKind(e.target.value as Tx["kind"])} className="w-40">
              {KINDS.map((k) => (
                <option key={k} value={k}>
                  {t(`bankroll.kinds.${k}`)}
                </option>
              ))}
            </Select>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t("bankroll.amount")}
            <Input className="w-32" inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value)} placeholder="50" data-testid="tx-amount" />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t("bankroll.date")}
            <Input className="w-40" type="date" value={date} onChange={(e) => setDate(e.target.value)} />
          </label>
          <label className="flex flex-1 flex-col gap-1 text-sm">
            {t("bankroll.note")}
            <Input value={note} onChange={(e) => setNote(e.target.value)} />
          </label>
          <Button onClick={() => void add()} disabled={!valid} data-testid="tx-add">
            <Plus className="h-4 w-4" />
            {t("bankroll.add")}
          </Button>
        </div>
        {txs.length === 0 ? (
          <p className="text-sm text-muted">{t("bankroll.none")}</p>
        ) : (
          <ul className="flex flex-col gap-1 text-sm" data-testid="tx-list">
            {txs.map((x) => (
              <li key={x.id} className="flex items-center gap-d3 rounded border border-border px-d3 py-d2">
                <span className="w-40 text-muted">{formatDate(x.at, lang)}</span>
                <span className="w-32">{t(`bankroll.kinds.${x.kind}`)}</span>
                <span className={`w-28 text-right tabular-nums ${x.kind === "withdrawal" ? "text-danger" : "text-success"}`}>
                  {x.kind === "withdrawal" ? "−" : "+"}
                  {euros(x.amountCents, lang, false)}
                </span>
                <span className="flex-1 truncate text-muted">{x.note}</span>
                <Button variant="ghost" size="icon" aria-label={t("settings.delete")} onClick={() => void del(x.id!)}>
                  <Trash2 className="h-4 w-4" />
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}
