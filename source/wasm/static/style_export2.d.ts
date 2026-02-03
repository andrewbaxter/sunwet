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
declare type OrientationType2 = "grid" | "flex";
declare type TransAlign = "start" | "middle" | "end";
declare type TextSizeMode = "wrap" | "ellipsize";
// Merge, separated to avoid issues with rust generation
declare interface Window {
  sunwetPresentation: Presentation;
}
