import { cn } from "@/lib/utils";

const SUIT: Record<string, { sym: string; cls: string; name: string }> = {
  s: { sym: "♠", cls: "bg-slate-800 text-white", name: "spades" },
  h: { sym: "♥", cls: "bg-red-600 text-white", name: "hearts" },
  d: { sym: "♦", cls: "bg-blue-600 text-white", name: "diamonds" },
  c: { sym: "♣", cls: "bg-green-700 text-white", name: "clubs" },
};

/** Four-colour card ("Ah"); `hidden` shows the back. */
export function PlayingCard({ card, hidden, size = "md" }: { card?: string; hidden?: boolean; size?: "sm" | "md" }) {
  const dims = size === "sm" ? "h-7 w-5 text-xs" : "h-11 w-8 text-sm";
  if (hidden || !card) {
    return <span className={cn("inline-block rounded border border-border bg-surface-alt", dims)} aria-label="?" />;
  }
  const rank = card[0] === "T" ? "10" : card[0];
  const s = SUIT[card[1]] ?? SUIT.s;
  return (
    <span
      className={cn("inline-flex flex-col items-center justify-center rounded font-bold leading-none shadow", dims, s.cls)}
      aria-label={`${rank} ${s.name}`}
      data-card={card}
    >
      <span>{rank}</span>
      <span>{s.sym}</span>
    </span>
  );
}

/** "AhKd" -> ["Ah", "Kd"] */
export const splitCards = (s: string): string[] => s.match(/.{2}/g) ?? [];

export function Cards({ cards, size }: { cards: string[]; size?: "sm" | "md" }) {
  return (
    <span className="inline-flex gap-0.5">
      {cards.map((c) => (
        <PlayingCard key={c} card={c} size={size} />
      ))}
    </span>
  );
}
