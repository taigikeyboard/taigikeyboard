interface NavigatorUAData {
  readonly platform: string;
}

interface Navigator {
  readonly userAgentData?: NavigatorUAData;
}

interface CaretPosition {
  readonly offsetNode: Node;
  readonly offset: number;
}

interface Document {
  caretPositionFromPoint?(x: number, y: number): CaretPosition | null;
}

interface EditContextTextUpdateEvent extends Event {
  readonly updateRangeStart: number;
  readonly updateRangeEnd: number;
  readonly text: string;
}

interface EditContext {
  addEventListener(
    type: "textupdate",
    listener: (event: EditContextTextUpdateEvent) => void,
  ): void;
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject | null,
    options?: boolean | AddEventListenerOptions,
  ): void;
}

interface HTMLElement {
  editContext?: EditContext | null;
}

interface Window {
  EditContext?: {
    new (): EditContext;
  };
}

declare var taigiMeasureCells: ((runsJson: string) => Float32Array) | undefined;

declare var taigiKeyboard: boolean | undefined;
