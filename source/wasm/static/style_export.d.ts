declare type Presentation = {
  ///////////////////////////////////////////////////////////////////////////////
  // xx Utility, globals
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

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: all

  leafAsyncBlock: (cb: () => Promise<HTMLElement>) => { root: HTMLElement };

  leafErrBlock: (data: Error) => { root: HTMLElement };

  contTitle: (args: { left: HTMLElement; right?: HTMLElement }) => {
    root: HTMLElement;
  };

  leafTitle: (text: string) => { root: HTMLElement };

  contBar: (
    extraStyles: string[],
    leftChildren: HTMLElement[],
    leftMidChildren: HTMLElement[],
    midChildren: HTMLElement[],
    rightMidChildren: HTMLElement[],
    rightChildren: HTMLElement[]
  ) => { root: HTMLElement };

  contBarMainForm: (
    leftChildren: HTMLElement[],
    leftMidChildren: HTMLElement[],
    midChildren: HTMLElement[],
    rightMidChildren: HTMLElement[],
    rightChildren: HTMLElement[]
  ) => { root: HTMLElement };

  contBarMenu: (children: HTMLElement[]) => { root: HTMLElement };

  contSpinner: () => { root: HTMLElement };

  leafSpace: () => { root: HTMLElement };

  leafButton: (
    title: string,
    text: string,
    extraStyles: string[],
    onclick?: () => void
  ) => { root: HTMLElement; button: HTMLElement };

  leafBarButtonBig: (
    title: string,
    icon: string
  ) => { root: HTMLElement; button: HTMLElement };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, form + edit

  leafInputPair: (
    label: string,
    inputId: string,
    input: HTMLElement
  ) => { root: HTMLElement };

  leafInputText: (
    id: string,
    title: string,
    value: string
  ) => { root: HTMLElement; input: HTMLInputElement };

  leafInputPairText: (
    id: string,
    title: string,
    value: string
  ) => { root: HTMLElement; input: HTMLInputElement };

  leafInputSelect: (
    id: string,
    children: HTMLElement[]
  ) => { root: HTMLElement; input: HTMLSelectElement };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, view

  contPageView: (entries: HTMLElement[]) => { root: HTMLElement };

  contBarViewTransport: () => {
    root: HTMLElement;
    buttonShare: HTMLElement;
    buttonPrev: HTMLElement;
    buttonNext: HTMLElement;
    buttonPlay: HTMLElement;
    seekbar: HTMLElement;
    seekbarFill: HTMLElement;
  };

  contMediaFullscreen: (media: HTMLElement) => {
    root: HTMLElement;
    buttonClose: HTMLElement;
  };

  contModal: (
    title: string,
    child: HTMLElement
  ) => { root: HTMLElement; buttonClose: HTMLElement };

  leafTransportButton: (
    title: string,
    icon: string
  ) => { root: HTMLElement; button: HTMLElement };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, form

  contPageForm: (entries: HTMLElement[]) => { root: HTMLElement };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, edit

  contPageEdit: (children: HTMLElement[]) => { root: HTMLElement };

  contPageEditSectionRel: (children: HTMLElement[]) => { root: HTMLElement };

  leafButtonEditFree: (
    icon: string,
    hint: string
  ) => { root: HTMLElement; button: HTMLElement };

  leafEditNode: (
    id: string,
    nodeHint: string,
    nodeType: string,
    node: string
  ) => {
    root: HTMLElement;
    inputType: HTMLSelectElement;
    inputValue: HTMLInputElement;
    buttonDelete: HTMLElement;
    buttonRevert: HTMLElement;
  };

  leafEditPredicate: (id: string, value: string) => { root: HTMLElement };

  leafEditRowIncoming: (children: HTMLElement[]) => { root: HTMLElement };

  leafEditRowOutgoing: (children: HTMLElement[]) => { root: HTMLElement };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu

  contBodyMenu: () => { root: HTMLElement };

  contMenuGroup: (
    title: string,
    children: HTMLElement[]
  ) => { root: HTMLElement };

  leafMenuLink: (title: string, href: string) => { root: HTMLElement };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: Main

  appMain: (children: HTMLElement[]) => { root: HTMLElement };

  ///////////////////////////////////////////////////////////////////////////////
  // xx PLUGINS: View

  buildView: (
    pluginId: string,
    arguments: any
  ) => Promise<{ root: HTMLElement }>;
};

declare interface Window {
  sunwet_presentation: Presentation;
}
