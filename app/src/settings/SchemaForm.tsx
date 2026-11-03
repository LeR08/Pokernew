/**
 * Renders a form from a JSON Schema (the subset used by our config files):
 * objects become fieldsets, enums selects, numbers number inputs, booleans
 * switches; `x-widget` picks a custom widget and `x-ref-family` a select of
 * the ids of another config family. Labels are i18n keys (schema `title`).
 */
import { useTranslation } from "react-i18next";
import type { Issue, JsonValue } from "@/lib/backend";
import { resolveRef, type JSONSchema } from "@/lib/schema";
import { Input, Select, Switch } from "@/components/ui/input";
import { ColorWidget, HotkeyWidget, ModuleOrderWidget, ModuleTogglesWidget } from "./widgets";

export interface FormContext {
  /** Ids available for `x-ref-family` selects. */
  refIds: (family: string) => string[];
}

interface FieldProps {
  root: JSONSchema;
  node: JSONSchema;
  name: string;
  pointer: string;
  value: JsonValue | undefined;
  onChange: (v: JsonValue) => void;
  issues: Issue[];
  ctx: FormContext;
}

const fieldId = (pointer: string) => `f${pointer.replace(/\//g, "-")}`;

function Field(props: FieldProps) {
  const { t } = useTranslation();
  const node = resolveRef(props.node, props.root);
  const { pointer, value, onChange, ctx } = props;
  const label = node.title ? t(node.title, { defaultValue: props.name }) : props.name;
  const help = node.description ? t(node.description) : undefined;
  const own = props.issues.filter((i) => i.pointer === pointer);
  const id = fieldId(pointer);

  if (node["x-widget"] === "hidden") return null;

  if (node.type === "object") {
    if (!node.properties) return null; // free-form maps are edited as raw text
    const obj = (value && typeof value === "object" && !Array.isArray(value) ? value : {}) as Record<string, JsonValue>;
    const body = Object.entries(node.properties).map(([k, child]) => (
      <Field
        key={k}
        {...props}
        node={child}
        name={k}
        pointer={`${pointer}/${k}`}
        value={obj[k]}
        onChange={(v) => onChange({ ...obj, [k]: v })}
      />
    ));
    if (pointer === "") return <div className="flex flex-col gap-d4">{body}</div>;
    return (
      <fieldset className="flex flex-col gap-d3 rounded-lg border border-border p-d4" data-pointer={pointer}>
        <legend className="px-d1 text-sm font-semibold">{label}</legend>
        {body}
      </fieldset>
    );
  }

  let control: JSX.Element;
  const widget = node["x-widget"];
  if (widget === "module-order") {
    control = <ModuleOrderWidget value={(value as string[]) ?? []} onChange={onChange} />;
  } else if (widget === "module-toggles") {
    control = <ModuleTogglesWidget value={(value as string[]) ?? []} onChange={onChange} />;
  } else if (widget === "color") {
    control = <ColorWidget id={id} value={String(value ?? "")} onChange={onChange} />;
  } else if (widget === "hotkey") {
    control = <HotkeyWidget id={id} value={String(value ?? "")} onChange={onChange} />;
  } else if (node["x-ref-family"]) {
    const ids = ctx.refIds(node["x-ref-family"]);
    const optional = !value;
    control = (
      <Select id={id} value={String(value ?? "")} onChange={(e) => onChange(e.target.value || null)}>
        {optional && <option value="">{t("widget.none")}</option>}
        {ids.map((x) => (
          <option key={x} value={x}>
            {x}
          </option>
        ))}
      </Select>
    );
  } else if (node.enum) {
    control = (
      <Select
        id={id}
        value={String(value ?? "")}
        onChange={(e) => {
          const raw = e.target.value;
          const match = node.enum!.find((x) => String(x) === raw);
          onChange(match ?? raw);
        }}
      >
        {node.enum.map((x) => (
          <option key={String(x)} value={String(x)}>
            {t(`enum.${x}`, { defaultValue: String(x) })}
          </option>
        ))}
      </Select>
    );
  } else if (node.type === "boolean") {
    control = <Switch id={id} label={label} checked={value === true} onChange={onChange} />;
  } else if (node.type === "number" || node.type === "integer") {
    control = (
      <Input
        id={id}
        type="number"
        className="w-40"
        min={node.minimum}
        max={node.maximum}
        step={node.type === "integer" ? 1 : (node.multipleOf ?? "any")}
        value={typeof value === "number" ? value : ""}
        aria-invalid={own.length > 0}
        onChange={(e) => {
          const n = e.target.valueAsNumber;
          onChange(Number.isNaN(n) ? null : n);
        }}
      />
    );
  } else if (node.type === "array") {
    const arr = Array.isArray(value) ? value : [];
    control = (
      <Input
        id={id}
        value={arr.join(", ")}
        onChange={(e) =>
          onChange(
            e.target.value
              .split(",")
              .map((s) => s.trim())
              .filter(Boolean),
          )
        }
      />
    );
  } else {
    control = (
      <Input
        id={id}
        value={String(value ?? "")}
        aria-invalid={own.length > 0}
        onChange={(e) => onChange(e.target.value)}
      />
    );
  }

  return (
    <div className="grid grid-cols-1 gap-d1 md:grid-cols-[14rem_1fr] md:items-start" data-pointer={pointer}>
      <label htmlFor={id} className="pt-2 text-sm">
        {label}
      </label>
      <div className="flex flex-col gap-d1">
        {control}
        {help && <p className="text-xs text-muted">{help}</p>}
        {own.map((i, n) => (
          <p key={n} className="text-xs text-danger" role="alert">
            {i.message}
          </p>
        ))}
      </div>
    </div>
  );
}

export function SchemaForm({
  schema,
  value,
  onChange,
  issues = [],
  ctx,
}: {
  schema: JSONSchema;
  value: JsonValue;
  onChange: (v: JsonValue) => void;
  issues?: Issue[];
  ctx: FormContext;
}) {
  return (
    <form onSubmit={(e) => e.preventDefault()} data-testid="schema-form">
      <Field root={schema} node={schema} name="" pointer="" value={value} onChange={onChange} issues={issues} ctx={ctx} />
    </form>
  );
}

/** Removes nulls (cleared optional fields) before saving. */
export function stripNulls(v: JsonValue): JsonValue {
  if (Array.isArray(v)) return v.filter((x) => x !== null).map(stripNulls);
  if (v && typeof v === "object") {
    return Object.fromEntries(
      Object.entries(v)
        .filter(([, x]) => x !== null)
        .map(([k, x]) => [k, stripNulls(x)]),
    );
  }
  return v;
}
