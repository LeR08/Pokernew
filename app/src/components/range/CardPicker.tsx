/** Cards as text ("AhKd7c") plus a clickable 52-card grid. */
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PlayingCard } from "@/components/PlayingCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { cardStr, makeCard, parseCards, type Card } from "@/lib/range";

export function CardPicker({
  label,
  value,
  onChange,
  max,
  blocked = [],
  testId,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  max: number;
  /** Cards used elsewhere (other players, board/dead). */
  blocked?: Card[];
  testId: string;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const cards = parseCards(value);
  const invalid = cards === null || cards.length > max || new Set(cards).size !== cards.length;
  const set = new Set(cards ?? []);
  const toggle = (c: Card) => {
    const cur = cards ?? [];
    const next = set.has(c) ? cur.filter((x) => x !== c) : cur.length < max ? [...cur, c] : cur;
    onChange(next.map(cardStr).join(""));
  };
  return (
    <div className="flex flex-col gap-1 text-sm">
      <span>{label}</span>
      <div className="flex items-center gap-d2">
        <Input
          className="w-40 font-mono"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          aria-invalid={invalid}
          placeholder={max === 5 ? "Ah7c2d" : "—"}
          data-testid={testId}
          aria-label={label}
        />
        <Button size="sm" variant="outline" onClick={() => setOpen(!open)} data-testid={`${testId}-pick`}>
          {open ? t("range.close") : t("cards.pick")}
        </Button>
        <span className="flex gap-0.5">
          {(cards ?? []).map((c) => (
            <PlayingCard key={c} card={cardStr(c)} size="sm" />
          ))}
        </span>
      </div>
      {invalid && <span className="text-xs text-danger">{t("cards.invalid", { max })}</span>}
      {open && (
        <div className="grid w-fit gap-0.5 rounded-md border border-border p-1" style={{ gridTemplateColumns: "repeat(13, 1.75rem)" }}>
          {[3, 2, 1, 0].flatMap((suit) =>
            Array.from({ length: 13 }, (_, i) => {
              const c = makeCard(12 - i, suit);
              const used = blocked.includes(c) && !set.has(c);
              return (
                <button
                  key={c}
                  disabled={used}
                  onClick={() => toggle(c)}
                  data-testid="card-option"
                  data-card={cardStr(c)}
                  aria-pressed={set.has(c)}
                  className={cn("rounded p-0.5 disabled:opacity-25", set.has(c) && "ring-2 ring-primary")}
                >
                  <PlayingCard card={cardStr(c)} size="sm" />
                </button>
              );
            }),
          )}
        </div>
      )}
    </div>
  );
}
