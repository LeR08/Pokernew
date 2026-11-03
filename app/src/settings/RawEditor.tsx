import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { EditorView, basicSetup } from "codemirror";
import { StreamLanguage } from "@codemirror/language";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { oneDark } from "@codemirror/theme-one-dark";
import { getBackend, BackendError, type Family, type Issue } from "@/lib/backend";
import { useRaw, useSaveText } from "@/lib/config";
import { Button } from "@/components/ui/button";

/** Raw TOML editor with live validation against the family's JSON Schema. */
export function RawEditor({ family, id }: { family: Family; id: string }) {
  const { t } = useTranslation();
  const raw = useRaw(family, id).data;
  const save = useSaveText(family);
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const [text, setText] = useState<string | null>(null);
  const [issues, setIssues] = useState<Issue[]>([]);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!host.current || raw === undefined) return;
    view.current?.destroy();
    view.current = new EditorView({
      parent: host.current,
      doc: raw,
      extensions: [
        basicSetup,
        StreamLanguage.define(toml),
        ...(document.documentElement.dataset.theme === "light" ? [] : [oneDark]),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) {
            setText(u.state.doc.toString());
            setSaved(false);
          }
        }),
      ],
    });
    setText(raw);
    return () => {
      view.current?.destroy();
      view.current = null;
    };
  }, [raw]);

  useEffect(() => {
    if (text === null) return;
    const h = setTimeout(() => {
      void getBackend()
        .then((b) => b.configValidate(family, id, text))
        .then(setIssues);
    }, 250);
    return () => clearTimeout(h);
  }, [text, family, id]);

  const dirty = text !== null && text !== raw;
  const onSave = () => {
    if (text === null) return;
    save.mutate(
      { id, text },
      {
        onSuccess: () => setSaved(true),
        onError: (e) => setIssues(e instanceof BackendError ? e.issues : []),
      },
    );
  };

  return (
    <div className="flex flex-col gap-d3">
      <div ref={host} data-testid="raw-editor" className="overflow-hidden rounded-md border border-border bg-surface text-sm" />
      <div className="flex items-center gap-d3">
        <Button onClick={onSave} disabled={!dirty || issues.length > 0} data-testid="raw-save">
          {t("settings.save")}
        </Button>
        {saved && !dirty && <span className="text-sm text-success">{t("settings.saved")}</span>}
        {issues.length === 0 ? (
          <span className="text-sm text-success" data-testid="raw-status">
            {t("settings.raw_valid")}
          </span>
        ) : (
          <ul className="text-sm text-danger" data-testid="raw-issues" role="alert">
            {issues.map((i, n) => (
              <li key={n}>
                <code>{i.pointer || "/"}</code> — {i.message}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
