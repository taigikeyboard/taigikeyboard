import type { SettingsDocument } from "../shared.d.ts";

export const STORAGE_KEY: "taigi.settings";
export const SHORTCUTS_KEY: "taigi.shortcuts";
export function emptyDocument(): SettingsDocument & { revision: number };
export function parseStored(text: unknown): SettingsDocument;
