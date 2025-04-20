declare type BuildFn = (arguments: any) => HTMLElement;

declare type Node_ = { t: "f"; v: string } | { t: "v"; v: any };
declare type TreeNode =
  | { type: "scalar"; value: Node_ }
  | { type: "array"; value: TreeNode[] }
  | { type: "record"; value: { [s: string]: TreeNode } };
declare type QueryRes = Map<string, TreeNode>;
