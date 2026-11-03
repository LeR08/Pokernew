import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowDown, ArrowUp, FolderOpen, X } from "lucide-react";
import { getBackend } from "@/lib/backend";
import { Button } from "@/components/ui/button";
import { Input, Switch } from "@/components/ui/input";
import { formatEvent } from "@/hotkeys/keys";
import { allModules, orderedModules } from "@/modules/registry";

export function ColorWidget({ id, value, onChange }: { id: string; value: string; onChange: (v: string) => void }) {
  return (
    <div className="flex items-center gap-d2">
      <input
        type="color"
        aria-label={id}
        value={/^#[0-9a-fA-F]{6}$/.test(value) ? value : "#000000"}
        onChange={(e) => onChange(e.target.value)}
        className="h-9 w-12 cursor-pointer rounded border border-border bg-transparent"
      />
      <Input id={id} value={value} onChange={(e) => onChange(e.target.value)} className="w-32 font-mono" />
    </div>
  );
}

export function HotkeyWidget({ id, value, onChange }: { id: string; value: string; onChange: (v: string) => void }) {
  const { t } = useTranslation();
  const [recording, setRecording] = useState(false);
  return (
    <div className="flex items-center gap-d2">
      <kbd id={id} className="min-w-24 rounded border border-border bg-surface-alt px-d2 py-1 font-mono text-sm">
        {value}
      </kbd>
      <Button
        variant="outline"
        size="sm"
        data-hotkey-capture
        onClick={() => setRecording(true)}
        onBlur={() => setRecording(false)}
        onKeyDown={(e) => {
          if (!recording) return;
          e.preventDefault();
          e.stopPropagation();
          if (e.key === "Escape") return setRecording(false);
          const combo = formatEvent(e.nativeEvent);
          if (combo) {
            onChange(combo);
            setRecording(false);
          }
        }}
      >
        {recording ? t("widget.hotkey_press") : t("widget.hotkey_record")}
      </Button>
    </div>
  );
}

/** settings.modules.order: up/down list (drag-and-drop lives on the launcher). */
export function ModuleOrderWidget({ value, onChange }: { value: string[]; onChange: (v: string[]) => void }) {
  const { t } = useTranslation();
  const ids = orderedModules(value, [], true).map((m) => m.id);
  const move = (i: number, d: number) => {
    const next = [...ids];
    [next[i], next[i + d]] = [next[i + d], next[i]];
    onChange(next);
  };
  return (
    <ol className="flex flex-col gap-1" data-testid="module-order">
      {ids.map((id, i) => {
        const m = allModules.find((x) => x.id === id)!;
        return (
          <li key={id} className="flex items-center gap-d2 rounded border border-border px-d2 py-1 text-sm">
            <m.icon className="h-4 w-4 text-muted" aria-hidden />
            <span className="flex-1">{t(m.name)}</span>
            <Button variant="ghost" size="icon" aria-label={t("widget.move_up")} disabled={i === 0} onClick={() => move(i, -1)}>
              <ArrowUp className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label={t("widget.move_down")}
              disabled={i === ids.length - 1}
              onClick={() => move(i, 1)}
            >
              <ArrowDown className="h-4 w-4" />
            </Button>
          </li>
        );
      })}
    </ol>
  );
}

/** settings.modules.disabled shown as one "enabled" switch per module. */
export function ModuleTogglesWidget({ value, onChange }: { value: string[]; onChange: (v: string[]) => void }) {
  const { t } = useTranslation();
  return (
    <ul className="grid grid-cols-1 gap-1 sm:grid-cols-2">
      {allModules.map((m) => (
        <li key={m.id} className="flex items-center justify-between gap-d2 rounded border border-border px-d2 py-1 text-sm">
          <span>{t(m.name)}</span>
          <Switch
            label={t(m.name)}
            checked={!value.includes(m.id)}
            onChange={(on) => onChange(on ? value.filter((x) => x !== m.id) : [...value, m.id])}
          />
        </li>
      ))}
    </ul>
  );
}

/** List of folders with a native "add folder" picker. */
export function FolderListWidget({ value, onChange }: { value: string[]; onChange: (v: string[]) => void }) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1" data-testid="folder-list">
      {value.map((f) => (
        <div key={f} className="flex items-center gap-d2 rounded border border-border px-d2 py-1 text-sm">
          <code className="flex-1 truncate">{f}</code>
          <Button variant="ghost" size="icon" aria-label={t("widget.remove")} onClick={() => onChange(value.filter((x) => x !== f))}>
            <X className="h-4 w-4" />
          </Button>
        </div>
      ))}
      <Button
        variant="outline"
        size="sm"
        className="w-fit"
        onClick={() =>
          void getBackend()
            .then((b) => b.pickPaths({ directory: true, multiple: false, title: t("widget.add_folder") }))
            .then((p) => p && onChange([...new Set([...value, ...p])]))
        }
      >
        <FolderOpen className="h-4 w-4" />
        {t("widget.add_folder")}
      </Button>
    </div>
  );
}
