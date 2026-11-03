import { lazy, Suspense, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { BackendError, type Family, type Issue, type JsonObject, type JsonValue } from "@/lib/backend";
import { useConfig, useConfigList, useSaveValue, useSchema } from "@/lib/config";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { SchemaForm, stripNulls, type FormContext } from "./SchemaForm";

// CodeMirror is heavy: load it only when the raw editor is opened (faster start-up).
const RawEditor = lazy(() => import("./RawEditor").then((m) => ({ default: m.RawEditor })));

export function useFormContext(): FormContext {
  const themes = useConfigList("themes").data ?? [];
  const profiles = useConfigList("profiles").data ?? [];
  return { refIds: (f) => (f === "themes" ? themes : f === "profiles" ? profiles : []) };
}

/** Form / raw-text editor for one config file. */
export function FileEditor({ family, id }: { family: Family; id: string }) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<"form" | "raw">("form");
  const schema = useSchema(family).data;
  const current = useConfig(family, id).data;
  const save = useSaveValue(family, id);
  const ctx = useFormContext();
  const [draft, setDraft] = useState<JsonValue | undefined>(undefined);
  const [issues, setIssues] = useState<Issue[]>([]);
  const [saved, setSaved] = useState(false);

  // Reset the draft whenever the file changes (including edits on disk).
  useEffect(() => {
    setDraft(current);
    setIssues([]);
  }, [current]);

  const dirty = JSON.stringify(draft) !== JSON.stringify(current);
  const onSave = () => {
    if (!draft) return;
    save.mutate(stripNulls(draft) as JsonObject, {
      onSuccess: () => {
        setSaved(true);
        setIssues([]);
      },
      onError: (e) => setIssues(e instanceof BackendError ? e.issues : [{ family, id, pointer: "", message: String(e) }]),
    });
  };

  return (
    <div className="flex flex-col gap-d4" data-testid={`editor-${family}-${id}`}>
      <div role="tablist" className="inline-flex w-fit rounded-md border border-border p-0.5 text-sm">
        {(["form", "raw"] as const).map((m) => (
          <button
            key={m}
            role="tab"
            aria-selected={mode === m}
            className={cn("rounded px-d3 py-1", mode === m ? "bg-surface-alt font-medium" : "text-muted")}
            onClick={() => setMode(m)}
          >
            {t(`settings.mode.${m}`)}
          </button>
        ))}
      </div>
      {mode === "raw" ? (
        <Suspense fallback={null}>
          <RawEditor family={family} id={id} />
        </Suspense>
      ) : (
        schema &&
        draft !== undefined && (
          <>
            <SchemaForm
              schema={schema}
              value={draft}
              onChange={(v) => {
                setDraft(v);
                setSaved(false);
              }}
              issues={issues}
              ctx={ctx}
            />
            <div className="flex items-center gap-d3">
              <Button onClick={onSave} disabled={!dirty || save.isPending} data-testid="form-save">
                {t("settings.save")}
              </Button>
              <Button variant="outline" onClick={() => setDraft(current)} disabled={!dirty}>
                {t("settings.revert")}
              </Button>
              {saved && !dirty && <span className="text-sm text-success">{t("settings.saved")}</span>}
              {issues.some((i) => i.pointer === "") && (
                <span className="text-sm text-danger" role="alert">
                  {issues.find((i) => i.pointer === "")!.message}
                </span>
              )}
            </div>
          </>
        )
      )}
    </div>
  );
}
