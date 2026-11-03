import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

export function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: ReactNode }) {
  const { t } = useTranslation();
  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-black/50 p-d4" role="dialog" aria-modal aria-label={title}>
      <div className="w-full max-w-4xl rounded-lg border border-border bg-surface p-d4 shadow-xl">
        <div className="mb-d3 flex items-center justify-between">
          <h2 className="text-lg font-semibold">{title}</h2>
          <Button onClick={onClose} data-testid="modal-done">
            {t("range.done")}
          </Button>
        </div>
        {children}
      </div>
    </div>
  );
}
