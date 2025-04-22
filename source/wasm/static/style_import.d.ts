// Rust methods callable from plugins

declare type Node_ = { t: "f"; v: string } | { t: "v"; v: any };
declare type TreeNode =
  | { type: "scalar"; value: Node_ }
  | { type: "array"; value: TreeNode[] }
  | { type: "record"; value: { [s: string]: TreeNode } };

declare type PlaylistEntryMediaType = "audio" | "video" | "image";
declare type PlaylistEntry = {
  name?: string;
  album?: string;
  artist?: string;
  cover?: string;
  file: string;
  media_type: PlaylistEntryMediaType;
};

declare type Sunwet = {
  query(id: string, data: { [s: string]: TreeNode }): Promise<TreeNode[]>;
  fileUrl(file: string): string;
  editUrl(node: Node_): string;
  setPlaylist(playlist: PlaylistEntry[]): void;
  togglePlay(index: number): void;
};

declare interface Window {
  sunwet: Sunwet;
}
