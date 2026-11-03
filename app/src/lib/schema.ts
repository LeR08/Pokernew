/** Minimal JSON Schema typing + helpers used by the form generator. */
export interface JSONSchema {
  $id?: string;
  $ref?: string;
  $defs?: Record<string, JSONSchema>;
  title?: string;
  description?: string;
  type?: "object" | "array" | "string" | "number" | "integer" | "boolean";
  properties?: Record<string, JSONSchema>;
  required?: string[];
  additionalProperties?: boolean | JSONSchema;
  items?: JSONSchema;
  enum?: (string | number)[];
  minimum?: number;
  maximum?: number;
  multipleOf?: number;
  minLength?: number;
  pattern?: string;
  uniqueItems?: boolean;
  default?: unknown;
  /** UI widget override: color, hotkey, module-order, module-toggles, hidden. */
  "x-widget"?: string;
  /** The value is the id of a file of another config family. */
  "x-ref-family"?: string;
}

/** Resolves a local `$ref` (`#/$defs/x`) against `root`, merging sibling keys. */
export function resolveRef(node: JSONSchema, root: JSONSchema): JSONSchema {
  if (!node.$ref) return node;
  const m = /^#\/\$defs\/(.+)$/.exec(node.$ref);
  const target = m ? root.$defs?.[m[1]] : undefined;
  if (!target) return node;
  const { $ref: _ignored, ...rest } = node;
  return { ...resolveRef(target, root), ...rest };
}
