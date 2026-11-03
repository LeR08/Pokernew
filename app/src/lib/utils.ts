import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

/** Reads a nested value by JSON pointer ("/a/b"). */
export function getAt(obj: unknown, pointer: string): unknown {
  if (!pointer) return obj;
  return pointer
    .split("/")
    .slice(1)
    .reduce<unknown>((o, k) => (o && typeof o === "object" ? (o as Record<string, unknown>)[k] : undefined), obj);
}
