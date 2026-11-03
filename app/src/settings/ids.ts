/** Same rule as the Rust side (crates/config/src/family.rs). */
export const isValidId = (id: string): boolean => /^[A-Za-z0-9_-]{1,64}$/.test(id);
