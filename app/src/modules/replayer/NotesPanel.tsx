import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Trash2 } from "lucide-react";
import { getBackend } from "@/lib/backend";
import { formatDate } from "@/lib/importApi";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Select } from "@/components/ui/input";

export const COLOURS = ["", "red", "orange", "yellow", "green", "blue", "violet"] as const;
export const COLOUR_HEX: Record<string, string> = {
  red: "#e34948",
  orange: "#eb6834",
  yellow: "#eda100",
  green: "#1baf7a",
  blue: "#2a78d6",
  violet: "#6250d6",
};

/** Manual notes and colour label of a player (shown later in HUD popups). */
export function NotesPanel({ room, name }: { room: string; name: string }) {
  const { t, i18n } = useTranslation();
  const qc = useQueryClient();
  const key = ["db", "notes", room, name];
  const data = useQuery({ queryKey: key, queryFn: async () => (await getBackend()).notesGet(room, name) }).data;
  const [text, setText] = useState("");
  const refresh = () => qc.invalidateQueries({ queryKey: key });
  return (
    <Card className="flex flex-col gap-d2 p-d3 text-sm" data-testid="notes-panel">
      <div className="flex items-center justify-between gap-d2">
        <span className="font-semibold">{t("notes.title", { name })}</span>
        <Select
          className="w-32"
          aria-label={t("notes.colour")}
          value={data?.colour ?? ""}
          onChange={(e) => void getBackend().then((b) => b.playerColour(room, name, e.target.value || null)).then(refresh)}
        >
          {COLOURS.map((c) => (
            <option key={c} value={c}>
              {t(`notes.colours.${c || "none"}`)}
            </option>
          ))}
        </Select>
      </div>
      <textarea
        className="min-h-16 rounded-md border border-border bg-bg p-d2 text-sm"
        placeholder={t("notes.placeholder")}
        value={text}
        onChange={(e) => setText(e.target.value)}
        data-testid="note-text"
      />
      <Button
        size="sm"
        className="w-fit"
        disabled={!text.trim()}
        data-testid="note-add"
        onClick={() =>
          void getBackend()
            .then((b) => b.noteAdd(room, name, text))
            .then(() => {
              setText("");
              return refresh();
            })
        }
      >
        {t("notes.add")}
      </Button>
      <ul className="flex flex-col gap-1" data-testid="notes-list">
        {data?.notes.map((n) => (
          <li key={n.id} className="flex items-start gap-d2 rounded bg-surface-alt px-d2 py-1">
            <span className="flex-1 whitespace-pre-wrap">{n.text}</span>
            <span className="text-xs text-muted">{formatDate(n.createdAt, i18n.language)}</span>
            <Button
              variant="ghost"
              size="icon"
              aria-label={t("settings.delete")}
              onClick={() => void getBackend().then((b) => b.noteDelete(n.id)).then(refresh)}
            >
              <Trash2 className="h-3.5 w-3.5" />
            </Button>
          </li>
        ))}
      </ul>
    </Card>
  );
}
