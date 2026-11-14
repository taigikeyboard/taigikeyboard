import { type InitOutput, Session } from "../../site/pkg/taigi_web.js";

export { Session };

export function initAsm(
  asmFunc: (imports: Record<string, unknown>) => WebAssembly.Exports,
): InitOutput;
