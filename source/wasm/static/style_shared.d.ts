// Style/presentation stuff used by plugins too

declare type PresentationShared = {
  setOnPlaylistStateChanged: (
    cb?: (playing: boolean, index: number) => void
  ) => void;

  ///////////////////////////////////////////////////////////////////////////////
  // xx Utility, globals
  textIconPlay: string;
  textIconPause: string;
  contGroupStyle: string;
  contVboxStyle: string;
  contHboxStyle: string;
  contStackStyle: string;

  notnull: <T>(x: T | null | undefined) => T;

  uniq: (...args: string[]) => string;

  uniqn: (...args: string[]) => string;

  e: <N extends keyof HTMLElementTagNameMap>(
    name: N,
    args: Partial<HTMLElementTagNameMap[N]>,
    args2: {
      styles_?: string[];
      children_?: HTMLElement[];
    }
  ) => HTMLElementTagNameMap[N];

  et: (
    markup: string,
    args?: {
      styles_?: string[];
      children_?: HTMLElement[];
    }
  ) => HTMLElement;

  v: (v: string) => string;

  vs: (light: string, dark: string) => string;

  s: (
    id: string,
    f: { [s: string]: (r: CSSStyleDeclaration) => void }
  ) => string;

  ss: (
    id: string,
    f: { [s: string]: (r: CSSStyleDeclaration) => void }
  ) => string;
};

declare interface Window {
  sunwet_presentation_shared: PresentationShared;
}
