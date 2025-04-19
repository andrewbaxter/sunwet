declare type Direction = "up" | "down" | "left" | "right";
declare type Orientation =
  | "up_left"
  | "up_right"
  | "down_left"
  | "down_right"
  | "left_up"
  | "left_down"
  | "right_up"
  | "right_down";
declare type Align = "start" | "end";
declare interface Window {
  _wongus: {
    stream_cbs: Map<number, (line: string) => void>;
    responses: Map<number, (body: any) => void>;
    external_ipc: (id: number, args: any) => void;
  };
  ipc: {
    postMessage: (message: string) => void;
  };
}
