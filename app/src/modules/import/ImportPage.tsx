import { useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, ClipboardCopy, Database, FileUp, FolderOpen, FolderSearch, RotateCcw, Save } from "lucide-react";
import { getBackend } from "@/lib/backend";
import {
  formatDate,
  ik,
  testCaseText,
  useBackups,
  useDbStats,
  useImportErrors,
  useImportEvents,
  useImportStatus,
} from "@/lib/importApi";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import type { ModuleManifest } from "../types";

function Stat({ label, value, testId }: { label: string; value: number | string; testId?: string }) {
  return (
    <Card className="flex flex-col gap-d1 p-d4">
      <span className="text-xs text-muted">{label}</span>
      <span className="text-2xl font-semibold tabular-nums" data-testid={testId}>
        {typeof value === "number" ? value.toLocaleString() : value}
      </span>
    </Card>
  );
}

function Section({ title, icon: Icon, children, actions }: { title: string; icon: typeof Database; children: React.ReactNode; actions?: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-d3">
      <div className="flex flex-wrap items-center justify-between gap-d2">
        <h2 className="flex items-center gap-d2 text-lg font-semibold">
          <Icon className="h-5 w-5 text-primary" aria-hidden />
          {title}
        </h2>
        <div className="flex flex-wrap gap-d2">{actions}</div>
      </div>
      {children}
    </section>
  );
}

export function ImportPage({ manifest }: { manifest: ModuleManifest }) {
  const { t, i18n } = useTranslation();
  const qc = useQueryClient();
  const last = useImportEvents();
  const stats = useDbStats().data;
  const status = useImportStatus().data;
  const errors = useImportErrors().data ?? [];
  const backups = useBackups().data ?? [];
  const [copied, setCopied] = useState<number | null>(null);
  const [restoreAsked, setRestoreAsked] = useState<string | null>(null);

  const pick = async (directory: boolean) => {
    const b = await getBackend();
    const paths = await b.pickPaths({
      directory,
      multiple: !directory,
      title: t(directory ? "import.pick_folder" : "import.pick_files"),
    });
    if (paths?.length) await b.importPaths(paths);
  };

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-d6 p-d6">
      <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>

      {last && last.report.files_imported > 0 && (
        <div role="status" data-testid="import-toast" className="rounded-md border border-success bg-surface p-d3 text-sm">
          {t("import.toast", {
            count: last.report.hands_inserted,
            dups: last.report.duplicates,
            errors: last.report.errors,
          })}
        </div>
      )}

      <div className="grid grid-cols-2 gap-d3 md:grid-cols-4">
        <Stat label={t("import.stats.hands")} value={stats?.hands ?? 0} testId="stat-hands" />
        <Stat label={t("import.stats.tournaments")} value={stats?.tournaments ?? 0} />
        <Stat label={t("import.stats.players")} value={stats?.players ?? 0} />
        <Stat label={t("import.stats.errors")} value={stats?.errors ?? 0} testId="stat-errors" />
      </div>
      {stats && stats.hands > 0 && (
        <p className="-mt-d3 text-sm text-muted">
          {t("import.range", { from: formatDate(stats.firstHandAt, i18n.language), to: formatDate(stats.lastHandAt, i18n.language) })}
        </p>
      )}

      <Section
        title={t("import.folders")}
        icon={FolderSearch}
        actions={
          <>
            <Button variant="outline" size="sm" onClick={() => void pick(true)} data-testid="import-folder">
              <FolderOpen className="h-4 w-4" />
              {t("import.pick_folder")}
            </Button>
            <Button variant="outline" size="sm" onClick={() => void pick(false)}>
              <FileUp className="h-4 w-4" />
              {t("import.pick_files")}
            </Button>
          </>
        }
      >
        {status && status.folders.length === 0 ? (
          <p className="text-sm text-muted">
            {t("import.no_folders")}{" "}
            <Link className="text-primary underline" to="/settings/rooms">
              {t("import.configure_rooms")}
            </Link>
          </p>
        ) : (
          <ul className="flex flex-col gap-1 text-sm" data-testid="watched-folders">
            {status?.folders.map((f) => (
              <li key={f.path} className="flex items-center gap-d2 rounded border border-border px-d3 py-d2">
                <span className="rounded bg-surface-alt px-d2 py-0.5 text-xs">{f.room}</span>
                <code className="flex-1 truncate">{f.path}</code>
                <span className={f.exists ? "text-success" : "text-danger"}>
                  {t(f.exists ? "import.watching" : "import.missing")}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Section>

      <Section title={t("import.history")} icon={Database}>
        {status?.history.length ? (
          <div className="overflow-x-auto">
            <table className="w-full text-sm" data-testid="import-history">
              <thead className="text-left text-muted">
                <tr>
                  <th className="py-1 pr-d3 font-normal">{t("import.col.when")}</th>
                  <th className="py-1 pr-d3 font-normal">{t("import.col.trigger")}</th>
                  <th className="py-1 pr-d3 text-right font-normal">{t("import.col.new")}</th>
                  <th className="py-1 pr-d3 text-right font-normal">{t("import.col.dups")}</th>
                  <th className="py-1 pr-d3 text-right font-normal">{t("import.col.errors")}</th>
                  <th className="py-1 text-right font-normal">{t("import.col.speed")}</th>
                </tr>
              </thead>
              <tbody className="tabular-nums">
                {status.history.map((h, i) => (
                  <tr key={i} className="border-t border-border">
                    <td className="py-1 pr-d3">{formatDate(h.at, i18n.language)}</td>
                    <td className="py-1 pr-d3">{t(`import.trigger.${h.trigger}`)}</td>
                    <td className="py-1 pr-d3 text-right">{h.report.hands_inserted}</td>
                    <td className="py-1 pr-d3 text-right">{h.report.duplicates}</td>
                    <td className="py-1 pr-d3 text-right">{h.report.errors}</td>
                    <td className="py-1 text-right">
                      {h.report.elapsed_ms > 0 ? Math.round((h.report.hands_parsed * 1000) / h.report.elapsed_ms) : "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="text-sm text-muted">{t("import.no_history")}</p>
        )}
      </Section>

      <Section
        title={t("import.errors")}
        icon={AlertTriangle}
        actions={
          errors.length > 0 && (
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                void getBackend()
                  .then((b) => b.importRetryErrors())
                  .then(() => qc.invalidateQueries({ queryKey: ["db"] }))
              }
            >
              <RotateCcw className="h-4 w-4" />
              {t("import.retry")}
            </Button>
          )
        }
      >
        {errors.length === 0 ? (
          <p className="text-sm text-success">{t("import.no_errors")}</p>
        ) : (
          <ul className="flex flex-col gap-d2" data-testid="import-errors">
            {errors.map((e) => (
              <li key={e.id} className="rounded border border-border p-d3 text-sm">
                <div className="flex flex-wrap items-start justify-between gap-d2">
                  <div className="min-w-0 flex-1">
                    <div className="font-medium text-danger">{e.message}</div>
                    <code className="block truncate text-xs text-muted">
                      {e.file} · {t("import.line", { n: e.line_no })}
                    </code>
                    <code className="mt-1 block truncate">{e.line}</code>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      void navigator.clipboard.writeText(testCaseText(e)).then(() => {
                        setCopied(e.id);
                        setTimeout(() => setCopied(null), 2000);
                      })
                    }
                  >
                    <ClipboardCopy className="h-4 w-4" />
                    {copied === e.id ? t("import.copied") : t("import.copy_test_case")}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </Section>

      <Section
        title={t("import.backups")}
        icon={Save}
        actions={
          <Button
            variant="outline"
            size="sm"
            data-testid="backup-now"
            onClick={() =>
              void getBackend()
                .then((b) => b.backupNow())
                .then(() => qc.invalidateQueries({ queryKey: ik.backups }))
            }
          >
            <Save className="h-4 w-4" />
            {t("import.backup_now")}
          </Button>
        }
      >
        {backups.length === 0 ? (
          <p className="text-sm text-muted">{t("import.no_backups")}</p>
        ) : (
          <ul className="flex flex-col gap-1 text-sm" data-testid="backups">
            {backups.map((b) => (
              <li key={b.name} className="flex items-center gap-d2 rounded border border-border px-d3 py-d2">
                <code className="flex-1 truncate">{b.name}</code>
                <span className="text-muted">{(b.size / 1_048_576).toFixed(1)} Mo</span>
                {restoreAsked === b.name ? (
                  <Button
                    variant="danger"
                    size="sm"
                    onClick={() =>
                      void getBackend().then(async (be) => {
                        await be.backupRestore(b.name);
                        await be.appRestart();
                      })
                    }
                  >
                    {t("import.restore_confirm")}
                  </Button>
                ) : (
                  <Button variant="ghost" size="sm" onClick={() => setRestoreAsked(b.name)}>
                    {t("import.restore")}
                  </Button>
                )}
              </li>
            ))}
          </ul>
        )}
        <p className="text-xs text-muted">{t("import.db_path", { path: status?.dbPath ?? "" })}</p>
      </Section>
    </div>
  );
}
