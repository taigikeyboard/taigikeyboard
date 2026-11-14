import type { SettingsDocument } from "../shared.d.ts";

export const STORAGE_KEY: "taigi.settings";
export function emptyDocument(): SettingsDocument & { revision: number };
export function parseStored(text: unknown): SettingsDocument;
