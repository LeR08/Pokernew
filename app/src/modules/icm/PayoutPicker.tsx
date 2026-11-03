/** Prizes as text ("50, 30, 20") or from a saved structure. */
import { useTranslation } from "react-i18next";
import { Input, Select } from "@/components/ui/input";
import { prizes } from "@/lib/icm";
import { parsePlaces, usePayouts } from "./payouts";

export function PayoutPicker({ value, onChange, testId }: { value: string; onChange: (v: string) => void; testId: string }) {
  const { t } = useTranslation();
  const { list } = usePayouts();
  const bad = value.trim() !== "" && parsePlaces(value) === null;
  return (
    <div className="flex flex-wrap items-end gap-d2 text-sm">
      <label className="flex min-w-56 flex-1 flex-col gap-1">
        {t("icm.prizes")}
        <Input value={value} onChange={(e) => onChange(e.target.value)} aria-invalid={bad} placeholder="50, 30, 20" data-testid={testId} />
      </label>
      {list.length > 0 && (
        <Select
          className="w-56"
          value=""
          aria-label={t("icm.from_saved")}
          onChange={(e) => {
            const x = list.find((l) => l.id === e.target.value);
            if (x) onChange(prizes(x.p).map((v) => +v.toFixed(2)).join(", "));
          }}
          data-testid={`${testId}-saved`}
        >
          <option value="">{t("icm.from_saved")}</option>
          {list.map((l) => (
            <option key={l.id} value={l.id}>
              {l.p.name}
            </option>
          ))}
        </Select>
      )}
    </div>
  );
}
