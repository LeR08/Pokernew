import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { ExternalLink, Lock } from "lucide-react";
import { DndContext, closestCenter, PointerSensor, KeyboardSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, arrayMove, rectSortingStrategy, sortableKeyboardCoordinates, useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { orderedModules } from "@/modules/registry";
import type { ModuleManifest } from "@/modules/types";
import { useSaveValue, useSettings } from "@/lib/config";
import { getBackend, type JsonObject } from "@/lib/backend";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

function Tile({ m }: { m: ModuleManifest }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: m.id });
  const openWindow = () => void getBackend().then((b) => b.openModuleWindow(m.id));
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={isDragging ? "z-10 opacity-80" : undefined}
    >
      <Card
        data-testid="module-tile"
        data-module={m.id}
        {...attributes}
        {...listeners}
        onClick={() => navigate(`/m/${m.id}`)}
        onKeyDown={(e) => {
          if (e.key === "Enter") navigate(`/m/${m.id}`);
          listeners?.onKeyDown?.(e);
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          openWindow();
        }}
        className="group relative flex h-full cursor-pointer flex-col gap-d2 p-d4 transition hover:border-primary"
      >
        <div className="flex items-start justify-between">
          <m.icon className="h-7 w-7 text-primary" aria-hidden />
          <Button
            variant="ghost"
            size="icon"
            aria-label={t("launcher.open_window")}
            title={t("launcher.open_window")}
            className="opacity-0 group-hover:opacity-100"
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              openWindow();
            }}
          >
            <ExternalLink className="h-4 w-4" />
          </Button>
        </div>
        <div className="font-semibold">{t(m.name)}</div>
        <div className="flex-1 text-sm text-muted">{t(m.description)}</div>
        <div className="flex items-center gap-d2 text-xs text-muted">
          <span className="rounded bg-surface-alt px-d2 py-0.5">{t("launcher.phase", { phase: m.phase })}</span>
          {m.studyOnly && (
            <span className="inline-flex items-center gap-1 rounded bg-surface-alt px-d2 py-0.5">
              <Lock className="h-3 w-3" aria-hidden />
              {t("module.study_only")}
            </span>
          )}
        </div>
      </Card>
    </div>
  );
}

export function Launcher() {
  const { t } = useTranslation();
  const settings = useSettings().data;
  const save = useSaveValue("settings", "settings");
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  if (!settings) return null;
  const modules = orderedModules(settings.modules.order, settings.modules.disabled);
  const ids = modules.map((m) => m.id);

  const onDragEnd = (e: DragEndEvent) => {
    if (!e.over || e.active.id === e.over.id) return;
    const all = orderedModules(settings.modules.order, settings.modules.disabled, true).map((m) => m.id);
    const order = arrayMove(all, all.indexOf(String(e.active.id)), all.indexOf(String(e.over.id)));
    save.mutate({ ...settings, modules: { ...settings.modules, order } } as unknown as JsonObject);
  };

  return (
    <div className="p-d6">
      <h1 className="text-2xl font-semibold">{t("launcher.title")}</h1>
      <p className="mb-d6 text-sm text-muted">{t("launcher.subtitle")}</p>
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
        <SortableContext items={ids} strategy={rectSortingStrategy}>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-d4" data-testid="launcher-grid">
            {modules.map((m) => (
              <Tile key={m.id} m={m} />
            ))}
          </div>
        </SortableContext>
      </DndContext>
    </div>
  );
}
