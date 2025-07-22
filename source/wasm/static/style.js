/// <reference path="style_export.d.ts" />
/// <reference path="style_export2.d.ts" />
{
  const presentation = {};

  ///////////////////////////////////////////////////////////////////////////////
  // xx Utility, globals

  const notnull = /** @type {<T>(x: T | null | undefined) => T} */ (x) => {
    if (x == null) {
      throw Error();
    }
    return x;
  };

  const uniq = /** @type {(...args: string[]) => string} */ (...args) => {
    const lines = [];
    for (const e of notnull(new Error().stack).matchAll(/(\d+):\d+/g)) {
      lines.push(`${e[1]}`);
    }
    let uniq = [lines[1]];
    uniq.push(...args);
    return `r${uniq.join("_")}`;
  };

  const e = /** @type {
    <N extends keyof HTMLElementTagNameMap>(
      name: N,
      args: Partial<HTMLElementTagNameMap[N]>,
      args2: {
        styles_?: string[];
        children_?: Element[];
      }
    ) => HTMLElementTagNameMap[N]
  } */ (name, args1, args2) => {
    const out = document.createElement(name);
    if (args1 != null) {
      for (const [k, v] of Object.entries(args1)) {
        // @ts-ignore
        out[k] = v;
      }
    }
    if (args2.children_ != null) {
      for (const c of args2.children_) {
        out.appendChild(c);
      }
    }
    if (args2.styles_ != null) {
      for (const c of args2.styles_) {
        out.classList.add(c);
      }
    }
    return out;
  };

  const et = /** @type { 
      (
        markup: string,
        args?: {
          styles_?: string[];
          children_?: Element[];
        }
      ) => HTMLElement
    } */ (t, args) => {
    const out = /** @type {HTMLElement} */ (
      new DOMParser().parseFromString(t, "text/html").body.firstElementChild
    );
    if (args != null) {
      if (args.styles_ != null) {
        for (const c of args.styles_) {
          out.classList.add(c);
        }
      }
      if (args.children_ != null) {
        for (const c of args.children_) {
          out.appendChild(c);
        }
      }
    }
    return out;
  };

  const globalStyle = new CSSStyleSheet();
  document.adoptedStyleSheets.push(globalStyle);
  globalStyle.insertRule(`:root {}`);
  const globalStyleRoot = /** @type { CSSStyleRule } */ (
    globalStyle.cssRules[globalStyle.cssRules.length - 1]
  ).style;
  const globalStyleMediaLight =
    /** @type { CSSMediaRule } */
    (
      globalStyle.cssRules[
        globalStyle.insertRule("@media (prefers-color-scheme: light) {}")
      ]
    );
  const globalStyleLight =
    /** @type { CSSStyleRule} */
    (
      globalStyleMediaLight.cssRules[
        globalStyleMediaLight.insertRule(":root {}")
      ]
    ).style;
  const globalStyleMediaDark =
    /** @type { CSSMediaRule } */
    (
      globalStyle.cssRules[
        globalStyle.insertRule("@media not (prefers-color-scheme: light) {}")
      ]
    );
  const globalStyleDark =
    /** @type { CSSStyleRule} */
    (globalStyleMediaDark.cssRules[globalStyleMediaDark.insertRule(":root {}")])
      .style;

  const v = /** @type {(id: string, v: string) => string} */ (id, val) => {
    const name = `--${id}`;
    globalStyleRoot.setProperty(name, val);
    return `var(${name})`;
  };

  const vs = /** @type {(id:String, light: string, dark: string) => string} */ (
    id,
    light,
    dark
  ) => {
    const name = `--${id}`;
    globalStyleLight.setProperty(name, light);
    globalStyleDark.setProperty(name, dark);
    return `var(${name})`;
  };

  /** @type { Set<string> } */
  const staticStyles = new Set();
  // Static style - the id must be unique for every value closed on (i.e. put all the arguments in the id).
  const ss = /** @type {(
    id: string,
    f: { [s: string]: (r: CSSStyleDeclaration) => void }
  ) => string} */ (id, f) => {
    if (staticStyles.has(id)) {
      return id;
    }
    for (const [suffix, f1] of Object.entries(f)) {
      globalStyle.insertRule(`.${id}${suffix} {}`, 0);
      f1(/** @type { CSSStyleRule } */ (globalStyle.cssRules[0]).style);
    }
    staticStyles.add(id);
    return id;
  };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Constants

  const textIconPlay = "\ue037";
  const textIconPause = "\ue034";
  const textIconDelete = "\ue15b";
  const textIconRevert = "\ue166";
  const textIconAdd = "\ue145";
  const textIconNext = "\ue5cc";
  const textIconPrev = "\ue5cb";
  const textIconLink = "\ue157";
  const textIconCommit = "\ue161";
  const textIconSave = "\uf090";
  const textIconUnlink = "\ue16f";
  const textIconLogin = "\uea77";
  const textIconLogout = "\ue9ba";
  const textIconMenu = "\ue5d2";
  const textIconFoldClosed = "\ue316";
  const textIconFoldOpened = "\ue313";
  const textIconClose = "\ue5cd";
  const textIconFullscreen = "\ue5d0";
  const textIconRelIn = "\uf72d";
  const textIconRelOut = "\uf72e";
  const textIconEdit = "\ue3c9";
  const textIconView = "\ue8f4";
  const textIconHistory = "\ue889";
  const textIconCenter = "\ue39e";

  // xx Variables
  const varFNormal = "12pt";
  const varFCode = "12pt";
  const varFTitle = "24pt";
  const varFMenu = "20pt";
  const varFMenuIcon = "14pt";
  const varFPageTitle = "18pt";
  const varFModalTitle = "24pt";
  const varFModalIconClose = "20pt";
  const varFRelIcon = "32pt";
  const varFLinkBig = "14pt";

  const varSFullscreenIconClose = "1cm";
  const varSModalIconClose = "1.4cm";
  const varSNarrow = "20cm";
  const varSModalHeight = "20cm";
  const varSCol1Width = "min(0.8cm, 5dvw)";
  const varSCol3Width = "1.4cm";
  const varSTransportGutterRadius = "0.05cm";
  const varSRelIcon = "min(10dvw, 1.5cm)";
  const varSNodeButton = "min(10dvw, 0.8cm)";
  const varSHistPredObj = "min(10dvw, 1cm)";
  const varSColWidthRaw = `12cm`;
  const varSColWidth = `min(100%, ${varSColWidthRaw})`;
  const varSMenuIndent = "0.6cm";
  const varSLinkIcon = "3cm";
  const varSButtonSmallIcon = "0.7cm";
  const varSButtonBigIcon = "0.6cm";

  const varPSmall = "0.2cm";
  const varPBarBottom = "0.7cm";
  const varPPageBottom = "2.5cm";
  const varPViewRows = "0.8cm";
  const varPViewCol = "0.5cm";
  const varPViewList = "0.3cm";
  const varPViewHoriz = `max(0.3cm, min(${varSCol1Width}, 100dvw / 20))`;
  const varP05 = "0.5cm";
  const varPFormCommentTop = "0.3cm";
  const varPMenu = "0.5cm";
  const varPAppTitle = "0.3cm";
  const varPTitle = "0.4cm";
  const varPModalTitleLeft = "0.5cm";
  const varPNodeCenter = "min(2dvw, 0.4cm)";
  const varPHistoryMid = "min(3dvw, 0.5cm)";
  const varPHistoryBig = "min(5dvw, 0.8cm)";
  const varPLink = "0.5cm";
  const varPLinkGap = "0.3cm";
  const varPLinkText = "0.4cm";
  const varPButtonBig = "0.3cm";
  const varPButtonSmall = "0.1cm";

  const varLThin = "0.04cm";
  const varLMid = "0.06cm";
  const varLThick = "0.08cm";

  const varRMedia = "0.2cm";
  const varRModal = "0.2cm";
  const varRNode = "0.2cm";
  const varRNodeButton = "0.2cm";
  const varRLink = "0.5cm";

  const varWTransportBold = "300";
  const varWLight = "100";
  const varWRelIcon = "800";
  const varWNodeButton = "300";
  const varWLinkLogoText = "200";

  const varONoninteractive = "0.5";
  const varONoninteractiveLight = "0.3";
  const varONodePredicate = "0.5";
  const varOMenuBar = "0.5";

  const varCLinkDisplayBg = v(uniq("link_display_bg"), "rgb(35, 35, 36)");
  const varCLinkLogoText = vs(uniq("link_logo_text"), "#fefefe", "rgb(0,0,0)");
  const varCLinkBackground = v(uniq("link_background"), "rgb(45, 45, 46)");
  const varCLinkForegroundDark = v(
    uniq("foreground_dark"),
    "rgb(249, 248, 240)"
  );

  const varCSpinner = v(uniq("spinner"), "rgb(155, 178, 229)");

  const varCAppTitle = vs(
    uniq("app_title"),
    "rgb(93,113,134)",
    "rgb(164, 180, 200)"
  );
  const varCModalVeil = vs(
    uniq("modal_veil"),
    "rgba(0,0,0,0.3)",
    "rgb(0,0,0,0.3)"
  );
  const varCBackground = vs(
    uniq("background"),
    "rgb(230, 232, 238)",
    "rgb(70, 73, 77)"
  );
  const varCBackground2 = vs(
    uniq("node_center"),
    "rgb(215, 217, 225)",
    "rgb(82, 87, 94)"
  );
  const varCNodeCenterLine = vs(
    uniq("node_center_line"),
    "rgb(204, 207, 217)",
    "rgb(89, 95, 104)"
  );
  const varCBackgroundMenu = vs(
    uniq("background_menu"),
    "rgb(205, 208, 217)",
    "rgb(85, 87, 90)"
  );
  const varCBackgroundMenuBar = vs(
    uniq("background_menu_bar"),
    "rgb(219, 223, 232)",
    "rgb(99, 102, 104)"
  );
  const varCButtonHover = vs(
    uniq("button_hover"),
    "rgba(255, 255, 255, 0.7)",
    "rgb(56, 61, 64)"
  );
  const varCButtonClick = vs(
    uniq("button_click"),
    "rgba(255, 255, 255, 1)",
    "rgb(49, 50, 53)"
  );
  const varCSeekbarEmpty = vs(
    uniq("seekbar_empty"),
    "rgb(212, 216, 223)",
    "rgb(58, 57, 57)"
  );
  const varCForeground = vs(
    uniq("foreground"),
    "rgb(0, 0, 0)",
    "rgb(244, 255, 255)"
  );
  const varCForegroundFade = vs(
    uniq("foreground_fade"),
    "rgb(123, 123, 123)",
    "rgb(167, 177, 177)"
  );
  const varCForegroundError = vs(
    uniq("foreground_error"),
    "rgb(154, 60, 74)",
    "rgb(243, 69, 95)"
  );
  const varCModified = vs(
    uniq("border_modified"),
    "rgb(20, 194, 121)",
    "rgb(5, 136, 81)"
  );
  const varCSelected = vs(
    uniq("selected"),
    "rgb(66, 104, 219)",
    "rgb(111, 144, 245)"
  );
  const varCBorderError = vs(
    uniq("border_error"),
    "rgb(192, 61, 80)",
    "rgb(168, 72, 72)"
  );
  const varCInputUnderline = vs(
    uniq("input_underline"),
    "rgba(154, 157, 168, 0.5)",
    "rgba(128, 131, 145, 0.5)"
  );
  const varCHighlightBold = vs(
    uniq("highlight"),
    "rgb(140, 172, 245)",
    "rgb(78, 129, 183)"
  );
  const varCRemove = vs(uniq("remove"), "rgb(136, 136, 136)", "rgb(0,0,0)");
  const varCLogoWhite = vs(
    uniq("logo_white"),
    "rgb(254, 254, 254)",
    "rgb(96, 96, 96)"
  );
  const varCLogoBlue = vs(
    uniq("logo_blue"),
    "rgb(124,167,219)",
    "rgb(105, 140, 185)"
  );

  // xx State classes

  const attrState = "data-state";
  presentation.attrState = /** @type { Presentation["attrState"]} */ () => ({
    value: attrState,
  });
  const attrStatePlaying = "playing";
  presentation.attrStatePlaying =
    /** @type { Presentation["attrStatePlaying"]} */ () => ({
      value: attrStatePlaying,
    });

  const classMenuWantStateOpen = "want_state_open";
  presentation.classMenuWantStateOpen =
    /** @type { Presentation["classMenuWantStateOpen"]} */ () => ({
      value: classMenuWantStateOpen,
    });
  const classMenuStateOpen = "state_open";
  presentation.classMenuStateOpen =
    /** @type { Presentation["classMenuStateOpen"]} */ () => ({
      value: classMenuStateOpen,
    });
  const classStateHide = "hide";
  presentation.classStateHide =
    /** @type { Presentation["classStateHide"]} */ () => ({
      value: classStateHide,
    });
  const classInputStateInvalid = "invalid";
  presentation.classStateInvalid =
    /** @type { Presentation["classStateInvalid"]} */ () => ({
      value: classInputStateInvalid,
    });
  const classInputStateModified = "modified";
  presentation.classStateModified =
    /** @type { Presentation["classStateModified"]} */ () => ({
      value: classInputStateModified,
    });
  const classStateThinking = "thinking";
  presentation.classStateThinking =
    /** @type { Presentation["classStateThinking"]} */ () => ({
      value: classStateThinking,
    });
  const classStateDisabled = "disabled";
  presentation.classStateDisabled =
    /** @type { Presentation["classStateDisabled"]} */ () => ({
      value: classStateDisabled,
    });
  const classStatePressed = "pressed";
  presentation.classStatePressed =
    /** @type { Presentation["classStatePressed"]} */ () => ({
      value: classStatePressed,
    });
  const classStateDeleted = "deleted";
  presentation.classStateDeleted =
    /** @type { Presentation["classStateDeleted"]} */ () => ({
      value: classStateDeleted,
    });
  const classStateSharing = "sharing";
  presentation.classStateSharing =
    /** @type { Presentation["classStateSharing"]} */ () => ({
      value: classStateSharing,
    });
  const classStateSelected = "selected";
  presentation.classStateSelected =
    /** @type { Presentation["classStateSelected"]} */ () => ({
      value: classStateSelected,
    });

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: all

  const contGroupStyle = "group";
  const contVboxStyle = "vbox";
  const contHboxStyle = "hbox";
  const contStackStyle = "stack";

  presentation.contGroup = /** @type {Presentation["contGroup"]} */ (args) => ({
    root: e("div", {}, { styles_: [contGroupStyle], children_: args.children }),
  });

  presentation.contStack = /** @type {Presentation["contStack"]} */ (args) => ({
    root: e("div", {}, { styles_: [contStackStyle], children_: args.children }),
  });

  presentation.contVbox = /** @type {Presentation["contVbox"]} */ (args) => ({
    root: e("div", {}, { styles_: [contVboxStyle], children_: args.children }),
  });

  presentation.contRootStack = /** @type {Presentation["contRootStack"]} */ (
    args
  ) => ({
    root: e(
      "div",
      {},
      {
        styles_: [
          contStackStyle,
          ss(uniq("cont_root_stack"), {
            ">*": (s) => {
              s.width = "100dvw";
              s.height = "100dvh";
            },
          }),
        ],
        children_: args.children,
      }
    ),
  });

  const contTitleStyle = ss(uniq("cont_title"), {
    "": (s) => {
      s.gridColumn = "1/4";
      s.gridRow = "1";
      s.margin = `${varPTitle} 0`;
      s.alignItems = "center";
      s.display = "grid";
      s.gridTemplateColumns = "subgrid";
    },
  });
  presentation.contTitle = /** @type {Presentation["contTitle"]} */ (args) => {
    const children = [args.left];
    if (args.right != null) {
      children.push(args.right);
    }
    return {
      root: e(
        "div",
        {},
        {
          styles_: [contTitleStyle],
          children_: children,
        }
      ),
    };
  };

  const leafTitleStyle = ss(uniq("leaf_title"), {
    "": (s) => {
      s.fontSize = varFTitle;
      s.gridColumn = "2";
      s.gridRow = "1";
    },
  });
  presentation.leafTitle = /** @type {Presentation["leafTitle"]} */ (args) => ({
    root: e(
      "h1",
      {
        textContent: args.text,
      },
      {
        styles_: [leafTitleStyle],
      }
    ),
  });

  const leafLinkStyle = ss(uniq("leafLinkStyle"), {
    ":hover": (s) => {
      s.textDecoration = "underline";
    },
  });

  const leafIconStyle = ss(uniq("icon"), {
    "": (s) => {
      s.display = "inline-grid";
      s.fontFamily = "I";
      s.gridTemplateColumns = "1fr";
      s.gridTemplateRows = "1fr";
      s.justifyItems = "center";
      s.alignItems = "center";
    },
    ">*": (s) => {
      s.gridColumn = "1";
      s.gridRow = "1";
    },
  });
  const leafIcon =
    /** @type {(args: {text: string, extraStyles?: string[]})=>HTMLElement} */ (
      args
    ) =>
      et(
        `
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
          <g transform="translate(50 50)"><text fill="currentColor" style="
            text-anchor: middle;
            dominant-baseline: central;
            font-family: I;
            font-size: 90px;
          ">${args.text}</text></g>
        </svg>
      `,
        {
          styles_: args.extraStyles,
        }
      );

  presentation.contBar = /** @type {Presentation["contBar"]} */ (args) => {
    /** @type { (children: Element[]) => HTMLElement} */
    const newHbox = (children) =>
      e(
        "div",
        {},
        {
          styles_: [
            contHboxStyle,
            ss(uniq("cont_bar_hbox"), {
              "": (s) => {
                s.alignItems = "center";
                s.gap = varPSmall;
                s.margin = `0 ${varPSmall}`;
              },
            }),
          ],
          children_: children,
        }
      );

    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            ss(uniq("cont_bar"), {
              "": (s) => {
                s.zIndex = "2";
                s.display = "grid";
                s.gridTemplateColumns =
                  "minmax(min-content, 1fr) auto minmax(min-content, 1fr)";
              },
              ">*:nth-child(1)": (s) => {
                s.gridColumn = "1";
                s.justifySelf = "start";
              },
              ">*:nth-child(2)": (s) => {
                s.gridColumn = "2";
                s.justifySelf = "center";
              },
              ">*:nth-child(3)": (s) => {
                s.gridColumn = "3";
                s.justifySelf = "end";
              },
            }),
            ...args.extraStyles,
          ],
          children_: [
            newHbox(args.leftChildren),
            e(
              "div",
              {},
              {
                styles_: [
                  ss(uniq("cont_bar_middle"), {
                    "": (s) => {
                      s.minWidth = "0";
                      s.display = "grid";
                      s.gridColumn = "2";
                      s.gridTemplateColumns =
                        "minmax(min-content, 1fr) auto minmax(min-content, 1fr)";
                    },
                    ">*:nth-child(1)": (s) => {
                      s.gridColumn = "1";
                      s.justifySelf = "end";
                    },
                    ">*:nth-child(2)": (s) => {
                      s.gridColumn = "2";
                      s.justifySelf = "center";
                    },
                    ">*:nth-child(3)": (s) => {
                      s.gridColumn = "3";
                      s.justifySelf = "start";
                    },
                  }),
                ],
                children_: [
                  newHbox(args.leftMidChildren),
                  newHbox(args.midChildren),
                  newHbox(args.rightMidChildren),
                ],
              }
            ),
            newHbox(args.rightChildren),
          ],
        }
      ),
    };
  };

  const contBarMainStyle = ss(uniq("cont_bar_main"), {
    "": (s) => {
      s.width = "100%";
      s.position = "fixed";
      s.bottom = varPBarBottom;

      s.transition = "0.03s opacity";
      s.opacity = "1";
    },
    [`.${classMenuStateOpen}`]: (s) => {
      s.opacity = "0";
    },
  });

  presentation.leafSpinner = /** @type {Presentation["leafSpinner"]} */ (
    args
  ) => ({
    root: et(
      `
    <svg viewBox="0 0 1 1" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <path id="x" fill="none" stroke="currentColor" stroke-width="0.08" stroke-linecap="round"
          d="M 0.38 0 H 0.46" />
      </defs>
      <g transform-origin="0.5 0.5">
        <use href="#x" transform="translate(0.5 0.5) rotate(0)" />
        <use href="#x" transform="translate(0.5 0.5) rotate(45)" />
        <use href="#x" transform="translate(0.5 0.5) rotate(90)" />
        <use href="#x" transform="translate(0.5 0.5) rotate(135)" />
        <use href="#x" transform="translate(0.5 0.5) rotate(180)" />
        <use href="#x" transform="translate(0.5 0.5) rotate(225)" />
        <use href="#x" transform="translate(0.5 0.5) rotate(270)" />
        <use href="#x" transform="translate(0.5 0.5) rotate(315)" />
        <animateTransform attributeType="XML" attributeName="transform" type="rotate" values="0; -360;"
          dur="10s" repeatCount="indefinite" />
      </g>
    </svg>
    `,
      {
        styles_: [
          ss(uniq("leaf_spinner"), {
            "": (s) => {
              s.color = varCSpinner;
              const size = "1cm";
              s.width = size;
              s.height = size;
            },
          }),
          ...args.extraStyles,
        ],
      }
    ),
  });

  presentation.leafAsyncBlock = /** @type {Presentation["leafAsyncBlock"]} */ (
    args
  ) => {
    const inner = e(
      "div",
      {},
      {
        styles_: [
          contStackStyle,
          ss(uniq("leaf_async_block"), {
            "": (s) => {
              s.justifyItems = "center";
              s.alignItems = "center";
            },
          }),
        ],
        children_: [presentation.leafSpinner({ extraStyles: [] }).root],
      }
    );
    if (args.inRoot) {
      inner.style.gridColumn = "1/4";
      inner.style.gridRow = "2";
    }
    return {
      root: inner,
    };
  };

  presentation.leafErrBlock = /** @type {Presentation["leafErrBlock"]} */ (
    args
  ) => {
    const out = e(
      "div",
      {},
      {
        styles_: [
          contStackStyle,
          ss(uniq("err_block"), {
            "": (s) => {
              s.flexGrow = "1";
              s.justifyItems = "center";
              s.alignItems = "center";
              s.gridColumn = "1/-1";
            },
            ">span": (s) => {
              s.color = varCForegroundError;
              s.pointerEvents = "initial";
            },
          }),
        ],
        children_: [e("span", { textContent: args.data }, {})],
      }
    );
    if (args.inRoot) {
      out.style.gridColumn = "1/4";
      out.style.gridRow = "2";
    }
    return {
      root: out,
    };
  };

  const leafSpaceStyle = ss(uniq("leaf_space"), {
    "": (s) => {
      s.flexGrow = "1";
    },
  });
  presentation.leafSpace = /** @type {Presentation["leafSpace"]} */ () => ({
    root: e("div", {}, { styles_: [leafSpaceStyle] }),
  });
  const leafSpace = presentation.leafSpace;

  const leafButtonStyle = ss(uniq("leaf_button"), {
    "": (s) => {
      s.flexDirection = "row";
      s.gap = varPSmall;
      s.alignItems = "center";
    },
    [`.${classStateDisabled}`]: (s) => {
      s.opacity = varONoninteractive;
    },
    [`:not(.${classStateDisabled}):hover`]: (s) => {
      s.backgroundColor = varCButtonHover;
    },
    [`:not(.${classStateDisabled}):hover:active`]: (s) => {
      s.backgroundColor = varCButtonClick;
    },
    [`:not(.${classStateDisabled}).${classStateThinking}`]: (s) => {
      // TODO horiz line with marching dashes instead
      s.opacity = varONoninteractive;
    },
    ">span": (s) => {
      s.minWidth = "max-content";
    },
  });
  const leafButton = /** @type {
    (args: { title: string, icon?: string, text?: string, extraStyles: string[] }) => { root: HTMLElement }
  } */ (args) => {
    const children = [];
    if (args.icon != null) {
      children.push(
        leafIcon({ text: args.icon, extraStyles: [leafIconStyle] })
      );
    }
    if (args.text != null) {
      children.push(e("span", { textContent: args.text }, {}));
    }
    return {
      root: e(
        "button",
        {
          title: args.title,
        },
        {
          styles_: [leafButtonStyle, contHboxStyle, ...args.extraStyles],
          children_: children,
        }
      ),
    };
  };
  const leafButtonLinkStyle = ss(uniq("leaf_button_link"), {
    "": (s) => {
      s.display = "flex";
    },
    ":hover": (s) => {
      s.textDecoration = "underline";
    },
  });
  const leafButtonLink = /** @type {
    (args: { title: string, icon?: string, text?: string, extraStyles: string[], url: string, download?: boolean }) => { root: HTMLElement }
  } */ (args) => {
    const children = [];
    if (args.icon != null) {
      children.push(
        leafIcon({ text: args.icon, extraStyles: [leafIconStyle] })
      );
    }
    if (args.text != null) {
      children.push(e("span", { textContent: args.text }, {}));
    }
    const out = e(
      "a",
      {
        title: args.title,
        href: args.url,
      },
      {
        styles_: [
          leafButtonStyle,
          leafButtonLinkStyle,
          contHboxStyle,
          ...args.extraStyles,
        ],
        children_: children,
      }
    );
    if (args.download != null && args.download) {
      out.download = "";
    }
    return {
      root: out,
    };
  };

  presentation.leafButtonBig =
    /** @type { Presentation["leafButtonBig"] } */
    (args) =>
      leafButton({
        title: args.title,
        icon: args.icon,
        text: args.text,
        extraStyles: [
          ss(uniq("leaf_text_button_big"), {
            "": (s) => {
              s.padding = `0 ${varPButtonBig}`;
            },
            ">span": (s) => {
              s.padding = `${varPButtonBig} 0`;
            },
            ">svg": (s) => {
              s.width = varSButtonBigIcon;
              s.minWidth = varSButtonBigIcon;
              s.height = varSButtonBigIcon;
              s.minHeight = varSButtonBigIcon;
            },
          }),
          ...args.extraStyles,
        ],
      });
  presentation.leafButtonBigView =
    /** @type { Presentation["leafButtonBigView"] } */
    (args) =>
      presentation.leafButtonBig({
        title: "View",
        icon: textIconView,
        text: "View",
        extraStyles: [],
      });
  presentation.leafButtonBigCommit =
    /** @type { Presentation["leafButtonBigCommit"] } */
    (args) =>
      presentation.leafButtonBig({
        title: "Commit",
        icon: textIconCommit,
        text: "Commit",
        extraStyles: [],
      });
  const leafButtonLinkSmall =
    /** @type {(args: { title: string, icon?: string, text?: string, url: string }) => { root: HTMLElement }} */ (
      args
    ) =>
      leafButtonLink({
        url: args.url,
        title: args.title,
        icon: args.icon,
        text: args.text,
        extraStyles: [
          ss(uniq("leaf_text_button_small"), {
            "": (s) => {
              s.padding = `${varPButtonSmall} ${varPButtonBig}`;
              s.color = varCForegroundFade;
            },
            ">svg": (s) => {
              s.width = varSButtonSmallIcon;
              s.minWidth = varSButtonSmallIcon;
              s.height = varSButtonSmallIcon;
              s.minHeight = varSButtonSmallIcon;
            },
          }),
        ],
      });

  const contBodyStyle = ss(uniq("cont_body"), {
    "": (s) => {
      s.gridRow = "2";
      s.gridColumn = "1/4";

      s.marginBottom = varPPageBottom;
    },
    [`.${classMenuStateOpen}`]: (s) => {
      s.visibility = "hidden";
    },
  });
  const contBodyNarrowStyle = ss(uniq("cont_body_narrow"), {
    "": (s) => {
      s.width = `min(${varSNarrow}, 100% - ${varSCol1Width} * 2)`;
      s.marginLeft = "auto";
      s.marginRight = "auto";
    },
  });

  const leafMediaStyle = ss(uniq("leaf_input_file_media"), {
    "": (s) => {
      s.pointerEvents = "initial";
    },
  });
  presentation.leafMediaImg = /** @type {Presentation["leafMediaImg"]} */ (
    args
  ) => ({
    root: e("img", { src: args.src, loading: "lazy" }, {}),
  });
  presentation.leafMediaAudio = /** @type {Presentation["leafMediaAudio"]} */ (
    args
  ) => ({
    root: e(
      "audio",
      {
        src: args.src,
        controls: true,
      },
      { styles_: [leafMediaStyle] }
    ),
  });
  presentation.leafMediaVideo = /** @type {Presentation["leafMediaVideo"]} */ (
    args
  ) => ({
    root: e(
      "video",
      { src: args.src, controls: true },
      { styles_: [leafMediaStyle] }
    ),
  });

  const pageButtonsStyle = ss(uniq("page_buttons_style"), {
    "": (s) => {
      s.justifyContent = "end";
      s.paddingLeft = varPSmall;
      s.paddingRight = varPSmall;
      s.paddingBottom = varPSmall;
    },
  });

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: home
  const leafLogoText =
    /** @type {(args: {extraStyles?:string[]})=>HTMLElement} */ (args) =>
      et(
        `
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 210.77">
              <path class="white" d="M187.6 100.09a69.05 69.05 0 0 0-68.8-63.7A69.05 69.05 0 0 0 57 74.7c4.35-.17 8.86-.06 13.54.37a56.99 56.99 0 0 1 105.12 26.33 72.7 72.7 0 0 0 11.94-1.31zm-9.93 41.27c-4.6.16-9.37.03-14.31-.45a56.91 56.91 0 0 1-44.56 21.47 57.06 57.06 0 0 1-56.25-47.73c-4.14-.1-8.12.2-12.01.83a69 69 0 0 0 127.14 25.88Z"/>
              <path class="blue" fill="none" stroke-linecap="round" stroke-width="10" d="M5 110.87c20.49-9.6 40.98-19.2 68-15.39 27.02 3.81 60.58 21.04 88 25 27.42 3.97 48.71-5.32 70-14.6"/>
              <path class="white2" fill="none" stroke-linecap="square" stroke-width="10" d="m34.52 44.15 12.13 8.81M86.6 6.3l4.64 14.27M151 6.3l-4.64 14.27m56.72 23.58-12.13 8.81m12.13 113.66-12.13-8.82M151 204.46l-4.64-14.26M86.6 204.46l4.64-14.26m-56.72-23.58 12.13-8.82"/>
              <text x="286" y="50%" style="font-size: 140pt;" dominant-baseline="middle">sunwet</text>
            </svg>
          `,
        {
          styles_: [
            ss(uniq("leaf_logo_text"), {
              "": (s) => {
                s.width = "min(100%, 12cm)";
                s.padding = "0.5cm";
              },
              ">*.white": (s) => {
                s.fill = varCLogoWhite;
              },
              ">*.white2": (s) => {
                s.stroke = varCLogoWhite;
              },
              ">*.blue": (s) => {
                s.stroke = varCLogoBlue;
              },
            }),
            ...(args.extraStyles || []),
          ],
        }
      );

  presentation.contPageHome = /** @type {Presentation["contPageHome"]} */ (
    args
  ) => ({
    root: e(
      "div",
      {},
      {
        styles_: [
          classMenuWantStateOpen,
          contHboxStyle,
          ss(uniq("cont_page_home"), {
            "": (s) => {
              s.gridColumn = "1/4";
              s.gridRow = "1/3";
              s.justifyContent = "center";
              s.alignItems = "center";
            },
            [`.${classMenuStateOpen}`]: (s) => {
              s.display = "none";
            },
          }),
        ],
        children_: [
          leafLogoText({
            extraStyles: [
              ss(uniq("cont_page_home_logo"), {
                ">text": (s) => {
                  s.fill = varCLogoWhite;
                },
              }),
            ],
          }),
        ],
      }
    ),
  });
  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, form + edit
  const leafInputPairStyle = ss(uniq("leaf_form_input_pair"), {
    "": (s) => {
      s.display = "grid";
      s.gridTemplateColumns = "subgrid";
      s.gridColumn = "1 / 3";
      s.alignItems = "first baseline";
    },
    ">*": (s) => {},
    ">*:nth-child(1)": (s) => {
      s.gridColumn = "1";
    },
    ">*:nth-child(2)": (s) => {
      s.gridColumn = "2";
    },
  });
  presentation.leafInputPair = /** @type {Presentation["leafInputPair"]} */ (
    args
  ) => ({
    root: e(
      "div",
      {},
      {
        styles_: [leafInputPairStyle],
        children_: [
          e(
            "label",
            {
              textContent: args.label,
              htmlFor: args.inputId,
            },
            {}
          ),
          args.input,
        ],
      }
    ),
  });

  const varSInputPad = v(uniq(), "0.1cm");
  const leafInputBorderStyle = uniq("leaf_input_border");
  ss(leafInputBorderStyle, {
    "": (s) => {
      s.borderBottom = `${varLThin} solid ${varCInputUnderline}`;
    },
    [` *.${leafInputBorderStyle}`]: (s) => {
      s.borderBottom = "none";
    },
    [`.${classInputStateModified}`]: (s) => {
      s.borderColor = varCModified;
      s.borderBottomWidth = varLThin;
    },
    [`.${classInputStateInvalid}`]: (s) => {
      s.borderColor = varCBorderError;
      s.borderBottomStyle = "dashed";
      s.borderBottomWidth = varLMid;
    },
  });
  const leafInputStyle = ss(uniq("leaf_input"), {
    "": (s) => {
      s.padding = varSInputPad;
      s.maxWidth = "9999cm";
    },
  });

  presentation.leafInputText = /** @type {Presentation["leafInputText"]} */ (
    args
  ) => {
    const out = e(
      "span",
      {
        contentEditable: "plaintext-only",
        title: args.title,
        textContent: args.value,
      },
      {
        styles_: [
          leafInputStyle,
          leafInputBorderStyle,
          ss(uniq("leaf_input_text"), {
            "": (s) => {
              s.whiteSpace = "pre-wrap";
              s.overflowWrap = "anywhere";
            },
            ":empty::before": (s) => {
              s.whiteSpace = "pre-wrap";
              s.overflowWrap = "anywhere";
              s.opacity = varONoninteractive;
            },
          }),
          ss(uniq("leaf_input_text_", args.title || " "), {
            ":empty::before": (s) => {
              s.content = JSON.stringify(args.title || " ");
            },
          }),
        ],
      }
    );
    if (args.id != null) {
      out.id = args.id;
    }
    return {
      root: out,
    };
  };

  presentation.leafInputNumber =
    /** @type {Presentation["leafInputNumber"]} */ (args) => {
      const out =
        /** @type {HTMLInputElement} */
        (
          e(
            "input",
            {
              type: "number",
              placeholder: args.title,
              title: args.title,
              name: args.id,
              value: args.value,
            },
            {
              styles_: [leafInputStyle, leafInputBorderStyle],
            }
          )
        );
      return { root: out };
    };
  presentation.leafInputBool = /** @type {Presentation["leafInputBool"]} */ (
    args
  ) => {
    const out =
      /** @type {HTMLInputElement} */
      (
        e(
          "input",
          {
            type: "checkbox",
            placeholder: args.title,
            title: args.title,
            name: args.id,
          },
          {
            styles_: [leafInputStyle, leafInputBorderStyle],
          }
        )
      );
    out.checked = args.value;
    if (args.id != null) {
      out.id = args.id;
    }
    return { root: out };
  };
  presentation.leafInputDate = /** @type {Presentation["leafInputDate"]} */ (
    args
  ) => {
    const out =
      /** @type {HTMLInputElement} */
      (
        e(
          "input",
          {
            type: "date",
            placeholder: args.title,
            title: args.title,
            name: args.id,
            value: args.value,
          },
          {
            styles_: [leafInputStyle, leafInputBorderStyle],
          }
        )
      );
    if (args.id != null) {
      out.id = args.id;
    }
    return { root: out };
  };
  presentation.leafInputTime = /** @type {Presentation["leafInputTime"]} */ (
    args
  ) => {
    const out =
      /** @type {HTMLInputElement} */
      (
        e(
          "input",
          {
            type: "time",
            placeholder: args.title,
            title: args.title,
            name: args.id,
            value: args.value,
          },
          {
            styles_: [leafInputStyle, leafInputBorderStyle],
          }
        )
      );
    if (args.id != null) {
      out.id = args.id;
    }
    return { root: out };
  };
  presentation.leafInputDatetime =
    /** @type {Presentation["leafInputDatetime"]} */ (args) => {
      const d = new Date(args.value);
      const initialValue = `${d.getFullYear()}-${(1 + d.getMonth())
        .toString()
        .padStart(2, "0")}-${d.getDate().toString().padStart(2, "0")}T${d
        .getHours()
        .toString()
        .padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d
        .getSeconds()
        .toString()
        .padStart(2, "0")}.${d.getMilliseconds().toString().padStart(3, "0")}`;
      const out =
        /** @type {HTMLInputElement} */
        (
          e(
            "input",
            {
              type: "datetime-local",
              placeholder: args.title,
              title: args.title,
              name: args.id,
              value: initialValue,
            },
            {
              styles_: [leafInputStyle, leafInputBorderStyle],
            }
          )
        );
      if (args.id != null) {
        out.id = args.id;
      }
      return { root: out };
    };
  presentation.leafInputColor = /** @type {Presentation["leafInputColor"]} */ (
    args
  ) => {
    const out =
      /** @type {HTMLInputElement} */
      (
        e(
          "input",
          {
            type: "color",
            placeholder: args.title,
            title: args.title,
            name: args.id,
            value: args.value,
          },
          {
            styles_: [leafInputStyle, leafInputBorderStyle],
          }
        )
      );
    if (args.id != null) {
      out.id = args.id;
    }
    return { root: out };
  };
  presentation.leafInputEnum = /** @type {Presentation["leafInputEnum"]} */ (
    args
  ) => {
    const children = [];
    for (const [k, v] of Object.entries(args.options)) {
      children.push(e("option", { textContent: v, value: k }, {}));
    }
    const out = e(
      "select",
      {
        title: args.title,
        name: args.id,
      },
      {
        styles_: [leafInputStyle, leafInputBorderStyle],
        children_: children,
      }
    );
    if (args.id != null) {
      out.id = args.id;
    }
    out.value = args.value;
    return { root: out };
  };
  presentation.leafInputTextMedia =
    /** @type {Presentation["leafInputTextMedia"]} */ (args) => {
      const media = e("div", {}, { styles_: [contGroupStyle] });
      const input = e(
        "input",
        {
          type: "text",
          title: args.title,
          value: args.value,
        },
        {
          styles_: [leafInputStyle],
        }
      );
      if (args.id != null) {
        input.id = args.id;
        input.name = args.id;
      }
      return {
        root: e(
          "div",
          {},
          {
            styles_: [contVboxStyle, leafInputBorderStyle],
            children_: [media, input],
          }
        ),
        input: input,
        media: media,
      };
    };
  presentation.leafInputFile = /** @type {Presentation["leafInputFile"]} */ (
    args
  ) => {
    const media = e("div", {}, { styles_: [contGroupStyle] });
    const input = e(
      "input",
      {
        type: "file",
        title: args.title,
      },
      {
        styles_: [leafInputStyle],
      }
    );
    if (args.id != null) {
      input.id = args.id;
      input.name = args.id;
    }
    input.addEventListener("input", () => {
      media.innerHTML = "";
      const r = new FileReader();
      const f = /** @type { File } */ (input.files?.item(0));
      r.addEventListener("load", (_) => {
        const url = /** @type {string} */ (r.result);
        switch (f.type.split("/")[0]) {
          case "image":
            {
              media.appendChild(presentation.leafMediaImg({ src: url }).root);
            }
            break;
          case "video":
            {
              media.appendChild(presentation.leafMediaVideo({ src: url }).root);
            }
            break;
          case "audio":
            {
              media.appendChild(presentation.leafMediaAudio({ src: url }).root);
            }
            break;
        }
      });
      r.readAsDataURL(f);
    });
    return {
      root: e(
        "div",
        {},
        {
          styles_: [contVboxStyle, leafInputBorderStyle],
          children_: [media, input],
        }
      ),
      input: input,
    };
  };

  presentation.leafInputPairText =
    /** @type {Presentation["leafInputPairText"]} */ (args) => {
      const input = presentation.leafInputText({
        id: args.id,
        title: args.title,
        value: args.value,
      });
      return {
        root: presentation.leafInputPair({
          label: args.title,
          inputId: args.id,
          input: input.root,
        }).root,
        input: input.root,
      };
    };
  presentation.leafInputPairBool =
    /** @type {Presentation["leafInputPairBool"]} */ (args) => {
      const input = presentation.leafInputBool({
        id: args.id,
        title: args.title,
        value: args.value,
      });
      return {
        root: presentation.leafInputPair({
          label: args.title,
          inputId: args.id,
          input: input.root,
        }).root,
        input: input.root,
      };
    };
  presentation.leafInputPairNumber =
    /** @type {Presentation["leafInputPairNumber"]} */ (args) => {
      const input = presentation.leafInputNumber({
        id: args.id,
        title: args.title,
        value: args.value,
      });
      return {
        root: presentation.leafInputPair({
          label: args.title,
          inputId: args.id,
          input: input.root,
        }).root,
        input: input.root,
      };
    };
  presentation.leafInputPairDate =
    /** @type {Presentation["leafInputPairDate"]} */ (args) => {
      const input = presentation.leafInputDate({
        id: args.id,
        title: args.title,
        value: args.value,
      });
      return {
        root: presentation.leafInputPair({
          label: args.title,
          inputId: args.id,
          input: input.root,
        }).root,
        input: input.root,
      };
    };
  presentation.leafInputPairTime =
    /** @type {Presentation["leafInputPairTime"]} */ (args) => {
      const input = presentation.leafInputTime({
        id: args.id,
        title: args.title,
        value: args.value,
      });
      return {
        root: presentation.leafInputPair({
          label: args.title,
          inputId: args.id,
          input: input.root,
        }).root,
        input: input.root,
      };
    };
  presentation.leafInputPairDatetime =
    /** @type {Presentation["leafInputPairDatetime"]} */ (args) => {
      const input = presentation.leafInputDatetime({
        id: args.id,
        title: args.title,
        value: args.value,
      });
      return {
        root: presentation.leafInputPair({
          label: args.title,
          inputId: args.id,
          input: input.root,
        }).root,
        input: input.root,
      };
    };
  presentation.leafInputPairColor =
    /** @type {Presentation["leafInputPairColor"]} */ (args) => {
      const input = presentation.leafInputColor({
        id: args.id,
        title: args.title,
        value: args.value,
      });
      return {
        root: presentation.leafInputPair({
          label: args.title,
          inputId: args.id,
          input: input.root,
        }).root,
        input: input.root,
      };
    };
  presentation.leafInputPairEnum =
    /** @type {Presentation["leafInputPairEnum"]} */ (args) => {
      const input = presentation.leafInputEnum({
        id: args.id,
        title: args.title,
        options: args.options,
        value: args.value,
      });
      return {
        root: presentation.leafInputPair({
          label: args.title,
          inputId: args.id,
          input: input.root,
        }).root,
        input: input.root,
      };
    };
  presentation.leafInputPairFile =
    /** @type {Presentation["leafInputPairFile"]} */ (args) => {
      const input = presentation.leafInputFile({
        id: args.id,
        title: args.title,
      });
      return {
        root: presentation.leafInputPair({
          label: args.title,
          inputId: args.id,
          input: input.root,
        }).root,
        input: input.input,
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, view

  presentation.contBarViewTransport =
    /** @type {Presentation["contBarViewTransport"]} */ () => {
      const buttonShare = leafTransportButton({
        title: "Share",
        icons: { "": textIconLink },
        extraStyles: [
          ss(uniq("cont_bar_view_transport_share_button"), {
            [`.${classStateSharing} text`]: (s) => {
              s.color = varCHighlightBold;
              s.fontWeight = varWTransportBold;
            },
          }),
        ],
      });
      const buttonPrev = leafTransportButton({
        title: "Previous",
        icons: { "": textIconPrev },
      });
      const seekbarFill = e(
        "div",
        {},
        {
          styles_: [
            ss(uniq("transport_gutter_fill"), {
              "": (s) => {
                s.height = "100%";
                s.width = "30%";
                s.borderRadius = varSTransportGutterRadius;
                s.backgroundColor = varCHighlightBold;
              },
            }),
          ],
        }
      );
      const seekbarLabel = e(
        "span",
        {
          textContent: "00:00",
        },
        {
          styles_: [
            ss(uniq("transport_time"), {
              "": (s) => {
                s.opacity = varONoninteractive;
                s.justifySelf = "end";
                s.margin = `0 ${varPSmall}`;
              },
            }),
          ],
        }
      );
      const seekbar = e(
        "div",
        {},
        {
          styles_: [
            contStackStyle,
            ss(uniq("transport_seekbar"), {
              "": (s) => {
                // Hack around https://github.com/w3c/csswg-drafts/issues/12081 to
                // set a default size without affecting min-content
                s.gridTemplateColumns = "minmax(min-content, 8cm)";
                s.flexBasis = "0";
                s.flexGrow = "1";

                s.pointerEvents = "initial";

                s.alignItems = "center";
              },
            }),
          ],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [
                  ss(uniq("transport_gutter"), {
                    "": (s) => {
                      s.height = "0.15cm";
                      s.borderRadius = varSTransportGutterRadius;
                      s.backgroundColor = varCSeekbarEmpty;
                    },
                  }),
                ],
                children_: [seekbarFill],
              }
            ),
            seekbarLabel,
          ],
        }
      );
      const buttonNext = leafTransportButton({
        title: "Next",
        icons: { "": textIconNext },
      });
      const buttonPlay = leafTransportButton({
        title: "Play",
        icons: { "": textIconPlay, [attrStatePlaying]: textIconPause },
      });
      const buttonCenter = leafTransportButton({
        title: "Scroll to",
        icons: { "": textIconCenter },
      });
      buttonCenter.root.addEventListener("click", (_) => {
        for (const b of document.getElementsByClassName(classStateSelected)) {
          b.scrollIntoView({ block: "center", inline: "center" });
          break;
        }
      });
      return {
        root: presentation.contBarMain({
          leftChildren: [buttonShare.root],
          leftMidChildren: [buttonPrev.root],
          midChildren: [seekbar, buttonPlay.root],
          rightMidChildren: [buttonNext.root],
          rightChildren: [buttonCenter.root],
        }).root,
        buttonShare: buttonShare.root,
        buttonNext: buttonNext.root,
        buttonPlay: buttonPlay.root,
        buttonPrev: buttonPrev.root,
        seekbar: seekbar,
        seekbarFill: seekbarFill,
        seekbarLabel: seekbarLabel,
      };
    };

  presentation.contPageView = /** @type {Presentation["contPageView"]} */ (
    args
  ) => {
    const children = [];
    if (args.transport != null) {
      children.push(args.transport);
    }
    if (args.params.length > 0) {
      children.push(
        e(
          "div",
          {},
          {
            styles_: [
              ss(uniq("view_list_params"), {
                "": (s) => {
                  s.display = "grid";
                  s.gridTemplateColumns = "auto 1fr";
                  s.alignItems = "first baseline";
                  s.padding = `0 ${varPViewHoriz}`;
                  s.paddingBottom = varP05;
                  s.columnGap = varPSmall;
                },
              }),
            ],
            children_: args.params,
          }
        )
      );
    }
    children.push(
      e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            ss(uniq("view_list_body"), {
              "": (s) => {
                s.padding = `0 ${varPViewHoriz}`;
              },
            }),
          ],
          children_: [args.rows],
        }
      )
    );
    return {
      root: e(
        "div",
        {},
        {
          styles_: [classMenuWantStateOpen, contVboxStyle, contBodyStyle],
          children_: children,
        }
      ),
    };
  };

  presentation.contViewRootRows =
    /** @type {Presentation["contViewRootRows"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            ss(uniq("cont_view_root_rows"), {
              "": (s) => {
                s.flexGrow = "1";
                s.gap = varPViewRows;
              },
            }),
          ],
          children_: args.rows,
        }
      ),
    });

  presentation.contViewRow = /** @type {Presentation["contViewRow"]} */ (
    args
  ) => ({
    root: e(
      "div",
      {},
      {
        styles_: [
          contHboxStyle,
          ss(uniq("cont_view_row"), {
            "": (s) => {
              s.flexWrap = "wrap";
              s.columnGap = varPViewCol;
              s.rowGap = varPViewList;
              s.justifyContent = "space-around";
              s.maxWidth = "100%";
            },
          }),
        ],
        children_: args.blocks,
      }
    ),
  });

  presentation.contViewBlock = /** @type {Presentation["contViewBlock"]} */ (
    args
  ) => {
    const out = e(
      "div",
      {},
      {
        styles_: [
          ss(uniq("cont_view_block"), {
            "": (s) => {
              s.maxWidth = "100%";
              s.flexBasis = "0";
              s.flexGrow = "1";
              s.minWidth = `5cm`;
            },
          }),
        ],
        children_: args.children,
      }
    );
    if (args.width != null) {
      const w = `max(5cm, min(${args.width}, 100%))`;
      out.style.width = w;
      out.style.maxWidth = w;
      out.style.minWidth = `min(5cm, ${args.width})`;
    }
    return {
      root: out,
    };
  };

  presentation.contMediaFullscreen =
    /** @type {Presentation["contMediaFullscreen"]} */ (args) => {
      const buttonClose = e(
        "button",
        {},
        {
          styles_: [
            leafButtonStyle,
            ss(uniq("cont_media_fullscreen_close"), {
              "": (s) => {
                const size = varSFullscreenIconClose;
                s.width = size;
                s.height = size;
              },
            }),
          ],
          children_: [leafIcon({ text: textIconClose })],
        }
      );
      const buttonFullscreen = e(
        "button",
        {},
        {
          styles_: [
            leafButtonStyle,
            ss(uniq("cont_media_fullscreen_fullscreen"), {
              "": (s) => {
                const size = varSFullscreenIconClose;
                s.width = size;
                s.height = size;
              },
            }),
          ],
          children_: [leafIcon({ text: textIconFullscreen })],
        }
      );
      return {
        buttonClose: buttonClose,
        buttonFullscreen: buttonFullscreen,
        root: e(
          "div",
          {},
          {
            styles_: [
              contVboxStyle,
              ss(uniq("cont_fullscreen"), {
                "": (s) => {
                  s.zIndex = "5";
                  s.backgroundColor = varCBackground;
                  s.justifyContent = "stretch";
                  s.width = "100dvw";
                  s.height = "100dvh";
                },
                ">*:nth-child(1)": (s) => {
                  s.flexGrow = "1";
                },
              }),
            ],
            children_: [
              args.media,
              e(
                "div",
                {},
                {
                  styles_: [
                    contHboxStyle,
                    ss(uniq("cont_media_fullscreen_close_bar"), {
                      "": (s) => {
                        s.justifyContent = "end";
                        s.padding = varPSmall;
                      },
                    }),
                  ],
                  children_: [
                    buttonClose,
                    leafSpace({}).root,
                    buttonFullscreen,
                  ],
                }
              ),
            ],
          }
        ),
      };
    };

  const newContModal = /** @type { 
    (args: { 
      title: string, 
      child: HTMLElement
    }) => { 
      root: HTMLElement, 
      buttonClose: HTMLElement,
    }
  } */ (args) => {
    const buttonClose = e(
      "button",
      {
        textContent: textIconClose,
      },
      {
        styles_: [
          leafIconStyle,
          leafButtonStyle,
          ss(uniq("cont_modal_close"), {
            "": (s) => {
              s.fontSize = varFModalIconClose;
              const size = varSModalIconClose;
              s.width = size;
              s.height = size;
              s.borderTopRightRadius = varRModal;
            },
          }),
        ],
      }
    );
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            contStackStyle,
            ss(uniq("cont_modal_outer"), {
              "": (s) => {
                s.position = "fixed";
                s.top = "0";
                s.bottom = "0";
                s.left = "0";
                s.right = "0";

                s.zIndex = "4";
                s.backgroundColor = varCModalVeil;
                s.pointerEvents = "initial";
              },
            }),
          ],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [
                  contVboxStyle,
                  ss(uniq("cont_modal"), {
                    "": (s) => {
                      s.justifySelf = "center";
                      s.alignSelf = "center";
                      /** @type { (min: string, pad: string, max: string) => string } */
                      const threeState = (min, pad, max) => {
                        return `max(min(${min}, 100%), min(100% - ${pad}, ${max}))`;
                      };
                      s.width = threeState("6cm", "1cm", varSNarrow);
                      s.height = threeState("10cm", "1cm", varSModalHeight);
                      s.background = varCBackground;
                      s.borderRadius = varRModal;
                    },
                  }),
                ],
                children_: [
                  e(
                    "div",
                    {},
                    {
                      styles_: [
                        contHboxStyle,
                        ss(uniq("cont_modal_title_bar"), {
                          "": (s) => {
                            s.alignItems = "center";
                          },
                        }),
                      ],
                      children_: [
                        e(
                          "h1",
                          {
                            textContent: args.title,
                          },
                          {
                            styles_: [
                              ss(uniq("cont_modal_title"), {
                                "": (s) => {
                                  s.marginLeft = varPModalTitleLeft;
                                  s.fontSize = varFModalTitle;
                                },
                              }),
                            ],
                          }
                        ),
                        presentation.leafSpace({}).root,
                        buttonClose,
                      ],
                    }
                  ),
                  args.child,
                ],
              }
            ),
          ],
        }
      ),
      buttonClose: buttonClose,
    };
  };

  const leafTransportButton =
    /** @type {(args: { title: string, icons: {[s: string]: string}, extraStyles?: string[] }) => { root: HTMLElement }} */
    (args) => {
      const size = "1cm";
      const baseIconStyle = ss(uniq("leaf_transport_button_base_child_state"), {
        "": (s) => {
          s.display = "none";
        },
      });

      const statePairs = Object.entries(args.icons);
      statePairs.sort();
      const children = [];
      const buildStyleId = [];
      /** @type {{[s:string]: (s: CSSStyleDeclaration) => void}} */
      const buildStyle = {};
      for (const [state, icon] of statePairs) {
        buildStyleId.push(state);
        buildStyleId.push(
          JSON.stringify(icon).replaceAll(/[^a-zA-Z0-9]*/g, "_")
        );
        const childMark = (() => {
          if (state == "") {
            return "default";
          } else {
            return state;
          }
        })();
        buildStyle[`[data-state="${state}"]>.${childMark}`] = (s) => {
          s.display = "initial";
        };
        children.push(
          leafIcon({ text: icon, extraStyles: [childMark, baseIconStyle] })
        );
      }

      const out = e(
        "button",
        {
          title: args.title,
        },
        {
          styles_: [
            leafButtonStyle,
            ss(uniq("leaf_transport_button"), {
              "": (s) => {
                s.width = size;
                s.height = size;
              },
              ">*": (s) => {
                s.width = "100%";
                s.height = "100%";
              },
              ">* text": (s) => {
                s.fontWeight = varWLight;
              },
            }),
            ss(uniq(...buildStyleId), buildStyle),
            ...(args.extraStyles || []),
          ],
          children_: children,
        }
      );
      out.setAttribute("data-state", "");
      return { root: out };
    };

  presentation.contModalViewShare =
    /** @type {Presentation["contModalViewShare"]} */ (args) => {
      const buttonUnshare = presentation.leafButtonBig({
        title: "Unlink",
        icon: textIconUnlink,
        text: `Unlink`,
        extraStyles: [
          ss(uniq("cont_modal_view_share_unlink_button"), {
            "": (s) => {
              s.borderBottomLeftRadius = varRModal;
              s.borderBottomRightRadius = varRModal;
            },
          }),
        ],
      });
      const out = newContModal({
        title: "Link",
        child: e(
          "div",
          {},
          {
            styles_: [
              contVboxStyle,
              ss(uniq("cont_modal_share_vbox"), {
                "": (s) => {
                  s.flexGrow = "1";
                },
              }),
            ],
            children_: [
              e(
                "div",
                {},
                {
                  styles_: [
                    contStackStyle,
                    ss(uniq("cont_modal_share_qr"), {
                      "": (s) => {
                        s.flexGrow = "1";
                        s.aspectRatio = "1/1";
                        s.alignSelf = "center";
                        s.maxWidth = "100%";
                        s.maxHeight = "100%";
                      },
                    }),
                  ],
                  children_: [
                    e(
                      "a",
                      { href: args.link, title: "Link URL" },
                      {
                        styles_: [
                          ss(uniq("cont_modal_view_share_a"), {
                            "": (s) => {
                              s.display = "block";
                              s.width = "100%";
                              s.height = "100%";
                            },
                            ">*": (s) => {
                              s.width = "100%";
                              s.height = "100%";
                            },
                          }),
                        ],
                        children_: [args.qr],
                      }
                    ),
                  ],
                }
              ),
              buttonUnshare.root,
            ],
          }
        ),
      });
      return {
        root: out.root,
        buttonClose: out.buttonClose,
        buttonUnshare: buttonUnshare.root,
      };
    };

  /** @type { (o: Orientation)=>Direction } */
  const conv = (o) => {
    switch (o) {
      case "up_left":
      case "up_right":
        return "up";
      case "down_left":
      case "down_right":
        return "down";
      case "left_up":
      case "left_down":
        return "left";
      case "right_up":
      case "right_down":
        return "right";
    }
  };

  /** @type { (o: Orientation)=>Direction } */
  const trans = (o) => {
    switch (o) {
      case "up_left":
      case "down_left":
        return "left";
      case "up_right":
      case "down_right":
        return "right";
      case "left_up":
      case "right_up":
        return "up";
      case "left_down":
      case "right_down":
        return "down";
    }
  };

  const classViewTransverseStart = "trans_start";
  const classViewTransverseMiddle = "trans_middle";
  const classViewTransverseEnd = "trans_end";

  const contViewListStyle = ss(uniq("cont_view_list"), {
    "": (s) => {
      s.pointerEvents = "initial";
      s.maxWidth = "100%";

      s.display = "flex";
      s.gap = varPViewList;
      s.overflow = "hidden";
    },
    ".root": (s) => {
      s.gap = varPViewRows;
    },
    [`>.${classViewTransverseStart}`]: (s) => {
      s.alignSelf = "first baseline";
    },
    [`>.${classViewTransverseEnd}`]: (s) => {
      s.alignSelf = "last baseline";
    },
  });
  const contViewListStyleWrap = /** @type {(wrap:boolean)=>string} */ (
    wrap
  ) => {
    if (wrap) {
      return ss(uniq("cont_view_list_wrap"), {
        "": (s) => {
          s.flexWrap = "wrap";
        },
      });
    } else {
      return ss(uniq("cont_view_list_nowrap"), { "": (s) => {} });
    }
  };

  presentation.contViewList = /** @type { Presentation["contViewList"] } */ (
    args
  ) => {
    const out = e(
      "div",
      {},
      {
        styles_: [
          contViewListStyle,
          contViewListStyleWrap(args.wrap),
          ss(
            uniq("cont_view_list", args.direction),
            /** @type { () => ({[prefix: string]: (s: CSSStyleDeclaration) => void}) } */ (
              () => {
                switch (args.direction) {
                  case "up":
                    return {
                      "": (s) => {
                        s.flexDirection = "column-reverse";
                      },
                    };
                  case "down":
                    return {
                      "": (s) => {
                        s.flexDirection = "column";
                      },
                    };
                  case "left":
                    return {
                      "": (s) => {
                        s.flexDirection = "row-reverse";
                      },
                    };
                  case "right":
                    return {
                      "": (s) => {
                        s.flexDirection = "row";
                      },
                    };
                }
              }
            )()
          ),
        ],
        children_: args.children,
      }
    );
    if (args.xScroll) {
      out.style.overflowX = "auto";
    }
    if (args.gap != null) {
      out.style.gap = args.gap;
    }
    return { root: out };
  };

  presentation.contViewTable =
    /** @type { Presentation["contViewTable"] } */
    (args) => {
      const template = [];
      const children1 = [];
      for (let j0 = 0; j0 < args.children.length; ++j0) {
        const j = j0 + 1;
        const row = args.children[j0];
        for (let i0 = 0; i0 < row.length; ++i0) {
          const child = /** @type {HTMLElement|SVGElement} */ (row[i0]);
          const i = i0 + 1;
          switch (conv(args.orientation)) {
            case "up":
              child.style.gridRow = `${args.children.length - j0}`;
              break;
            case "down":
              child.style.gridRow = `${j}`;
              break;
            case "left":
              child.style.gridColumn = `${args.children.length - j0}`;
              break;
            case "right":
              child.style.gridColumn = `${j}`;
              break;
          }
          switch (trans(args.orientation)) {
            case "up":
              child.style.gridRow = `${row.length - i0}`;
              break;
            case "down":
              child.style.gridRow = `${i}`;
              break;
            case "left":
              child.style.gridColumn = `${row.length - i0}`;
              break;
            case "right":
              child.style.gridColumn = `${i}`;
              break;
          }
          children1.push(child);
        }
        template.push("auto");
      }
      const out = e(
        "div",
        {},
        {
          styles_: [
            ss(uniq("cont_view_table"), {
              "": (s) => {
                s.display = "grid";
                s.pointerEvents = "initial";
                s.maxWidth = "100%";
              },
              [`>.${contViewListStyle}`]: (s) => {
                s.display = "contents";
              },
            }),
          ],
          children_: children1,
        }
      );
      switch (conv(args.orientation)) {
        case "up":
          out.style.gridTemplateRows = `1fr ${template.join(" ")}`;
          break;
        case "down":
          out.style.gridTemplateRows = `${template.join(" ")} 1fr`;
          break;
        case "left":
          out.style.gridTemplateColumns = `1fr ${template.join(" ")}`;
          break;
        case "right":
          out.style.gridTemplateColumns = `${template.join(" ")} 1fr`;
          break;
      }
      if (args.gap != null) {
        switch (conv(args.orientation)) {
          case "up":
          case "down":
            out.style.rowGap = args.gap;
            break;
          case "left":
          case "right":
            out.style.columnGap = args.gap;
            break;
        }
      }
      if (args.xScroll) {
        out.style.overflowX = "auto";
      }
      return { root: out };
    };

  const viewLeafTransStyle =
    /** @type { (args:{orientation: Orientation, transAlign: TransAlign})=>string } */ (
      args
    ) =>
      ss(uniq("view_leaf_trans_style", args.orientation, args.transAlign), {
        "": (s) => {
          const [key, val] =
            /** @type {()=>["align"|"justify", "end"|"middle"|"start"]} */ (
              () => {
                switch (trans(args.orientation)) {
                  case "up":
                    switch (args.transAlign) {
                      case "start":
                        return ["align", "end"];
                      case "middle":
                        return ["align", "center"];
                      case "end":
                        return ["align", "start"];
                    }
                  case "down":
                    switch (args.transAlign) {
                      case "start":
                        return ["align", "start"];
                      case "middle":
                        return ["align", "center"];
                      case "end":
                        return ["align", "end"];
                    }
                  case "left":
                    switch (args.transAlign) {
                      case "start":
                        return ["justify", "end"];
                      case "middle":
                        return ["justify", "center"];
                      case "end":
                        return ["justify", "start"];
                    }
                  case "right":
                    switch (args.transAlign) {
                      case "start":
                        return ["justify", "start"];
                      case "middle":
                        return ["justify", "center"];
                      case "end":
                        return ["justify", "end"];
                    }
                }
              }
            )();
          switch (key) {
            case "align":
              s.alignSelf = val;
              break;
            case "justify":
              s.justifySelf = val;
              break;
          }
        },
      });

  const viewMediaLinkMediaStyle = ss(uniq("leaf_view_media_link_media"), {
    "": (s) => {
      s.objectFit = "contain";
      s.aspectRatio = "auto";
      s.borderRadius = varRMedia;
      s.width = "100%";
      s.height = "100%";
    },
  });
  const viewMediaLinkStyle = ss(uniq("leaf_view_media_link"), {
    "": (s) => {
      s.flexShrink = "0";
    },
  });
  const viewMediaNonlinkMediaStyle = ss(uniq("leaf_view_media_nonlink_media"), {
    "": (s) => {
      s.objectFit = "contain";
      s.aspectRatio = "auto";
      s.flexShrink = "0";
      s.borderRadius = varRMedia;
    },
  });
  presentation.leafViewImage = /** @type { Presentation["leafViewImage"] } */ (
    args
  ) => {
    const out = (() => {
      if (args.link != null) {
        const img = e(
          "img",
          { src: args.src, loading: "lazy" },
          {
            styles_: [viewMediaLinkMediaStyle],
          }
        );
        if (args.text != null) {
          img.alt = args.text;
        }
        return e(
          "a",
          { href: args.link },
          {
            styles_: [viewMediaLinkStyle],
            children_: [img],
          }
        );
      } else {
        const img = e(
          "img",
          { src: args.src, loading: "lazy" },
          {
            styles_: [viewMediaNonlinkMediaStyle],
          }
        );
        if (args.text != null) {
          img.alt = args.text;
        }
        return img;
      }
    })();
    // todo add viewLeafTransStyle, need to add orientation
    if (args.width) {
      out.style.width = args.width;
    }
    if (args.height) {
      out.style.height = args.height;
    }
    return { root: out };
  };
  presentation.leafViewVideo = /** @type { Presentation["leafViewVideo"] } */ (
    args
  ) => {
    const out = (() => {
      if (args.link != null) {
        const media = e(
          "video",
          { src: args.src, autoplay: true, muted: true, loop: true },
          { styles_: [viewMediaLinkMediaStyle] }
        );
        const out = e(
          "a",
          { href: args.link },
          {
            styles_: [viewMediaLinkStyle],
            children_: [media],
          }
        );
        if (args.text != null) {
          out.title = args.text;
        }
        return out;
      } else {
        const media = e(
          "video",
          { src: args.src, controls: true },
          { styles_: [viewMediaNonlinkMediaStyle] }
        );
        if (args.src != null) {
          media.src = args.src;
        }
        if (args.text != null) {
          media.title = args.text;
        }
        return media;
      }
    })();
    // todo add viewLeafTransStyle, need to add orientation
    if (args.width) {
      out.style.width = args.width;
    }
    if (args.height) {
      out.style.height = args.height;
    }
    return { root: out };
  };
  presentation.leafViewAudio = /** @type { Presentation["leafViewAudio"] } */ (
    args
  ) => {
    const directionStyle = ss(
      uniq("leaf_view_audio_dir", args.direction),
      (() => {
        switch (args.direction) {
          case "up":
            return {
              "": (s) => {
                s.rotate = "270deg";
              },
            };
          case "down":
            return {
              "": (s) => {
                s.rotate = "90deg";
              },
            };
          case "left":
            return { "": (s) => {} };
          case "right":
            return { "": (s) => {} };
        }
      })()
    );
    const out = (() => {
      if (args.link != null) {
        const media = e(
          "audio",
          { src: args.src, autoplay: true, muted: true, loop: true },
          { styles_: [viewMediaLinkMediaStyle, directionStyle] }
        );
        const out = e(
          "a",
          { href: args.link },
          {
            styles_: [viewMediaLinkStyle],
            children_: [media],
          }
        );
        if (args.text != null) {
          out.title = args.text;
        }
        return out;
      } else {
        const media = e(
          "video",
          { src: args.src, controls: true },
          { styles_: [viewMediaNonlinkMediaStyle, directionStyle] }
        );
        if (args.src != null) {
          media.src = args.src;
        }
        if (args.text != null) {
          media.title = args.text;
        }
        return media;
      }
    })();
    // todo add viewLeafTransStyle, need to add orientation
    if (args.length) {
      out.style.width = args.length;
    }
    return { root: out };
  };
  presentation.leafViewColor = /** @type { Presentation["leafViewColor"] } */ (
    args
  ) => {
    const out = e("div", {}, {});
    out.style.backgroundColor = args.color;
    out.style.width = args.width;
    out.style.height = args.height;
    out.style.borderRadius = varRMedia;
    return { root: out };
  };

  const viewTextBaseStyle = ss(uniq("leaf_view_text_base"), {
    "": (s) => {
      s.pointerEvents = "initial";
      s.whiteSpace = "pre-wrap";
      s.overflowWrap = "anywhere";
      s.flexShrink = "1";
    },
  });
  const viewTextOrientationStyle = /** @type {(dir:Orientation)=>string} */ (
    orient
  ) =>
    ss(uniq("leaf_view_text_dir", orient), {
      "": /** @type { () => ((s: CSSStyleDeclaration) => void) } */ (
        () => {
          switch (orient) {
            case "up_left":
            case "down_left":
              return (s) => {
                s.writingMode = "vertical-rl";
              };
            case "up_right":
            case "down_right":
              return (s) => {
                s.writingMode = "vertical-lr";
              };
            case "left_up":
            case "left_down":
            case "right_up":
            case "right_down":
              return (s) => {
                s.writingMode = "horizontal-tb";
              };
          }
        }
      )(),
    });
  presentation.leafViewText = /** @type { Presentation["leafViewText"] } */ (
    args
  ) => {
    const alignStyle = viewLeafTransStyle({
      orientation: args.orientation,
      transAlign: args.transAlign,
    });
    const out = (() => {
      if (args.link == null) {
        return e(
          "span",
          {
            textContent: args.text,
          },
          {
            styles_: [
              viewTextBaseStyle,
              viewTextOrientationStyle(args.orientation),
              alignStyle,
            ],
          }
        );
      } else {
        return e(
          "a",
          {
            textContent: args.text,
            href: args.link,
          },
          {
            styles_: [
              viewTextBaseStyle,
              viewTextOrientationStyle(args.orientation),
              alignStyle,
            ],
          }
        );
      }
    })();
    if (args.fontSize != null) {
      out.style.fontSize = args.fontSize;
    }
    if (args.color != null) {
      out.style.color = args.color;
    }
    if (args.maxSize != null) {
      switch (conv(args.orientation)) {
        case "up":
        case "down":
          out.style.maxHeight = args.maxSize;
          break;
        case "left":
        case "right":
          out.style.maxWidth = args.maxSize;
          break;
      }
    }
    return { root: out };
  };
  presentation.leafViewDatetime =
    /** @type { Presentation["leafViewDatetime"] } */ (args) => {
      const alignStyle = viewLeafTransStyle({
        orientation: args.orientation,
        transAlign: args.transAlign,
      });
      const out = (() => {
        return e(
          "span",
          {
            textContent: (() => {
              try {
                return new Date(args.value).toLocaleString();
              } catch (e) {}
              return " ";
            })(),
          },
          {
            styles_: [
              viewTextBaseStyle,
              viewTextOrientationStyle(args.orientation),
              alignStyle,
            ],
          }
        );
      })();
      if (args.fontSize != null) {
        out.style.fontSize = args.fontSize;
      }
      if (args.color != null) {
        out.style.color = args.color;
      }
      return { root: out };
    };
  presentation.leafViewDate = /** @type { Presentation["leafViewDate"] } */ (
    args
  ) => {
    const alignStyle = viewLeafTransStyle({
      orientation: args.orientation,
      transAlign: args.transAlign,
    });
    const out = (() => {
      return e(
        "span",
        {
          textContent: (() => {
            try {
              return new Date(args.value).toLocaleDateString();
            } catch (e) {}
            return " ";
          })(),
        },
        {
          styles_: [
            viewTextBaseStyle,
            viewTextOrientationStyle(args.orientation),
            alignStyle,
          ],
        }
      );
    })();
    if (args.fontSize != null) {
      out.style.fontSize = args.fontSize;
    }
    if (args.color != null) {
      out.style.color = args.color;
    }
    return { root: out };
  };
  presentation.leafViewTime = /** @type { Presentation["leafViewTime"] } */ (
    args
  ) => {
    const alignStyle = viewLeafTransStyle({
      orientation: args.orientation,
      transAlign: args.transAlign,
    });
    const out = (() => {
      return e(
        "span",
        {
          textContent: (() => {
            try {
              return new Date(args.value).toLocaleTimeString();
            } catch (e) {}
            try {
              return new Date(`2000-01-01 ${args.value}`).toLocaleTimeString();
            } catch (e) {}
            return " ";
          })(),
        },
        {
          styles_: [
            viewTextBaseStyle,
            viewTextOrientationStyle(args.orientation),
            alignStyle,
          ],
        }
      );
    })();
    if (args.fontSize != null) {
      out.style.fontSize = args.fontSize;
    }
    if (args.color != null) {
      out.style.color = args.color;
    }
    return { root: out };
  };

  presentation.leafViewPlayButton =
    /** @type { Presentation["leafViewPlayButton"] } */ (args) => {
      const orientation = args.orientation || "right_down";
      const conv1 = conv(orientation);
      const iconStyle = ss(uniq("leaf_view_play_inner"), {
        "": (s) => {
          s.width = "100%";
          s.height = "100%";
          s.objectFit = "contain";
          s.aspectRatio = "1/1";
          s.position = "relative";
        },
        " text": (s) => {
          s.fontWeight = "100";
        },
      });
      const iconStyleConv = ss(
        uniq("leaf_view_play_inner_conv", conv1),
        /** @type { () => ({[suffix: string]: (s: CSSStyleDeclaration) => void}) } */ (
          () => {
            switch (conv1) {
              case "up":
                return {
                  " text": (s) => {
                    s.rotate = "270deg";
                  },
                };
              case "down":
                return {
                  " text": (s) => {
                    s.rotate = "90deg";
                  },
                };
              case "left":
                return {
                  " text": (s) => {
                    s.rotate = "180deg";
                  },
                };
              case "right":
                return {
                  " text": (s) => {},
                };
            }
          }
        )()
      );
      const buttonStyles = [
        leafButtonStyle,
        ss(uniq("leaf_view_play"), {
          "": (s) => {
            s.borderRadius = varRMedia;
            s.position = "relative";
          },
          ">*": (s) => {
            s.display = "none";
          },
          [`[data-state=""]>*:nth-child(1)`]: (s) => {
            s.display = "initial";
          },
          [`[data-state="${attrStatePlaying}"]>*:nth-child(2)`]: (s) => {
            s.display = "initial";
          },
          [`.${classStateSelected}`]: (s) => {
            s.color = varCSelected;
          },
        }),
        viewLeafTransStyle({
          orientation: args.orientation,
          transAlign: args.transAlign,
        }),
      ];
      if (args.image != null) {
        buttonStyles.push(
          ss(uniq("leaf_view_play_image"), {
            "": (s) => {
              s.backgroundSize = "cover";
              s.color = "white";
              s.backdropFilter = "invert()";
            },
          })
        );
      }
      const out = e(
        "button",
        {},
        {
          styles_: buttonStyles,
          children_: [
            leafIcon({
              text: textIconPlay,
              extraStyles: [iconStyle, iconStyleConv],
            }),
            leafIcon({
              text: textIconPause,
              extraStyles: [iconStyle, iconStyleConv],
            }),
          ],
        }
      );
      if (args.image != null) {
        out.style.backgroundImage = `url(${args.image})`;
      }
      const size = "1cm";
      out.style.width = (() => {
        if (args.width == null) {
          return size;
        } else {
          return args.width;
        }
      })();
      out.style.minWidth = (() => {
        if (args.width == null) {
          return size;
        } else {
          return args.width;
        }
      })();
      out.style.height = (() => {
        if (args.height == null) {
          return size;
        } else {
          return args.height;
        }
      })();
      out.style.minHeight = (() => {
        if (args.height == null) {
          return size;
        } else {
          return args.height;
        }
      })();
      out.setAttribute("data-state", "");
      return {
        root: out,
      };
    };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: fullscreen, media comic
  presentation.contMediaComicOuter =
    /** @type {Presentation["contMediaComicOuter"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            ss(uniq("cont_media_comic_outer2"), {
              "": (s) => {
                s.width = "100%";
                s.maxWidth = "100%";
                s.minHeight = "0";
                s.flexBasis = "0";

                s.display = "grid";
                s.gridTemplateColumns = "1fr";
                s.gridTemplateRows = "1fr auto 1fr";
                s.alignItems = "center";

                s.backgroundColor = varCBackground;
              },
              ">*": (s) => {
                s.gridRow = "2";
                s.gridColumn = "1";
              },
            }),
          ],
          children_: args.children,
        }
      ),
    });
  presentation.contMediaComicInner =
    /** @type {Presentation["contMediaComicInner"]} */ (args) => {
      // This scales to the height required to show a representative (media (TODO), please don't include obi) page's width.
      // This is done by making a container with 2 children:
      // - The strut uses aspect ratio to get the height required to show full width. This will be short for wide pages
      //   (but very tall for narrow pages). The container sets its height to max (the strut, the available height)
      // - The 2nd child is the container for the pages, which is scaled to the parent height.

      const strut = e(
        "div",
        {},
        {
          styles_: [
            ss(uniq("cont_media_comic_inner_strut"), {
              "": (s) => {
                s.maxWidth = "100%";
                s.gridColumn = "1";
                s.gridRow = "1";
              },
            }),
          ],
        }
      );
      strut.style.aspectRatio = `${args.minAspectX}/${args.minAspectY}`;

      if (args.rtl) {
        args.children.reverse();
      }
      const contScroll = e(
        "div",
        {},
        {
          styles_: [
            ss(uniq("cont_media_comic_inner_inner"), {
              "": (s) => {
                s.position = "absolute";
                s.left = "0";
                s.right = "0";
                s.top = "0";
                s.bottom = "0";

                s.overflowX = "auto";
                s.display = "flex";

                // For user scrollbar interaction
                s.pointerEvents = "initial";
              },
            }),
          ],
          children_: args.children,
        }
      );

      return {
        contScroll: contScroll,
        root: e(
          "div",
          {},
          {
            styles_: [
              ss(uniq("cont_media_comic_outer"), {
                "": (s) => {
                  s.width = "100%";
                  s.maxWidth = "100%";

                  // Hack, we can't use min(max-content, 100%) so instead use maxHeight and height which
                  // combine to form a min... thanks again CSS for straightforward and intuitive basic tools
                  s.height = "max-content";
                  s.maxHeight = "100%";

                  s.display = "grid";
                  s.gridTemplateColumns = "1fr";
                  s.gridTemplateRows = "1fr";
                  s.overflow = "hidden";
                  s.position = "relative";

                  // For click, scroll wheel events
                  s.pointerEvents = "initial";
                },
              }),
            ],
            children_: [strut, contScroll],
          }
        ),
      };
    };
  presentation.leafMediaComicEndPad =
    /** @type {Presentation["leafMediaComicEndPad"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            ss(uniq("leaf_media_comic_end_pad"), {
              "": (s) => {
                s.minWidth = "50dvw";
              },
            }),
          ],
        }
      ),
    });
  presentation.leafMediaComicMidPad =
    /** @type {Presentation["leafMediaComicMidPad"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            ss(uniq("leaf_media_comic_mid_pad"), {
              "": (s) => {
                s.minWidth = "1cm";
              },
            }),
          ],
        }
      ),
    });
  presentation.leafMediaComicPage =
    /** @type {Presentation["leafMediaComicPage"]} */ (args) => {
      const out = e(
        "img",
        { src: args.src, loading: "lazy" },
        {
          styles_: [
            ss(uniq("leaf_media_comic_page"), {
              "": (s) => {
                s.height = "100%";
                s.flexShrink = "0";
              },
            }),
          ],
        }
      );
      out.style.aspectRatio = `${args.aspectX}/${args.aspectY}`;
      return { root: out };
    };
  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, form

  presentation.contPageForm = /** @type {Presentation["contPageForm"]} */ (
    args
  ) => ({
    root: presentation.contGroup({
      children: [
        presentation.contBarMain({
          leftChildren: [],
          leftMidChildren: [],
          midChildren: [],
          rightMidChildren: [],
          rightChildren: args.barChildren,
        }).root,
        e(
          "div",
          {},
          {
            styles_: [
              classMenuWantStateOpen,
              contBodyStyle,
              contBodyNarrowStyle,
              contVboxStyle,
            ],
            children_: [
              e(
                "div",
                {},
                {
                  styles_: [
                    ss(uniq("cont_page_form"), {
                      "": (s) => {
                        s.display = "grid";
                        s.gridTemplateColumns = "auto 1fr";
                        s.alignItems = "first baseline";
                        s.columnGap = varPSmall;
                        s.rowGap = varPSmall;
                      },
                      ">label": (s) => {
                        s.gridColumn = "1";
                      },
                      ">input": (s) => {
                        s.gridColumn = "2";
                      },
                    }),
                  ],
                  children_: args.entries,
                }
              ),
              presentation.leafSpace({}).root,
            ],
          }
        ),
      ],
    }).root,
  });
  presentation.leafFormComment =
    /** @type {Presentation["leafFormComment"]} */ (args) => ({
      root: e(
        "p",
        { textContent: args.text },
        {
          styles_: [
            ss(uniq("leaf_form_comment"), {
              "": (s) => {
                s.gridColumn = "1/3";
                s.marginTop = varPFormCommentTop;
              },
            }),
          ],
        }
      ),
    });

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, view/edit node

  presentation.contPageNode = /** @type {Presentation["contPageNode"]} */ (
    args
  ) => {
    const body = e(
      "div",
      {},
      {
        styles_: [
          contBodyNarrowStyle,
          contVboxStyle,
          ss(uniq("page_node"), {
            "": (s) => {
              s.gap = varSNodeGap;
            },
          }),
        ],
        children_: args.children,
      }
    );
    return {
      root: presentation.contGroup({
        children: [
          presentation.contBarMain({
            leftChildren: [],
            leftMidChildren: [],
            midChildren: [],
            rightMidChildren: [],
            rightChildren: args.barChildren,
          }).root,
          e(
            "div",
            {},
            {
              styles_: [classMenuWantStateOpen, contVboxStyle, contBodyStyle],
              children_: [body],
            }
          ),
        ],
      }).root,
      body: body,
    };
  };

  let varSNodeGap = v(uniq(), "0.5cm");
  const leafNodeHboxStyle = ss(uniq("leaf_edit_hbox"), {
    "": (s) => {
      s.alignItems = "stretch";
      s.position = "relative";
    },
    ">*": (s) => {
      s.flexBasis = "0";
    },
  });
  const leafNodeVboxStyle = ss(uniq("leaf_edit_vbox"), {
    "": (s) => {
      s.flexGrow = "1";
      s.gap = varPSmall;
      s.border = `${varLThick} solid ${varCNodeCenterLine}`;
      s.borderRadius = varRNode;
      s.padding = varPSmall;
      s.overflow = "hidden";
    },
  });
  const leafNodeVBoxNewStyle = ss(uniq("leaf_edit_vbox_new"), {
    // increase specifity...
    [`.${leafNodeVboxStyle}`]: (s) => {
      s.borderStyle = "dashed";
    },
  });
  const leafNodeRelStyle = ss(uniq("leaf_edit_rel"), {
    "": (s) => {
      s.color = varCNodeCenterLine;
      s.fontSize = varFRelIcon;
      s.fontWeight = varWRelIcon;
    },
  });
  const leafNodeRelIncomingStyle = ss(uniq("leaf_edit_rel_incoming"), {
    "": (s) => {
      s.alignSelf = "end";
      s.width = varSRelIcon;
      s.minWidth = varSRelIcon;
      s.rotate = "90deg";
    },
  });
  const leafNodeRelOutgoingStyle = ss(uniq("leaf_edit_rel_outgoing"), {
    "": (s) => {
      s.alignSelf = "start";
      s.width = varSRelIcon;
      s.minWidth = varSRelIcon;
      s.rotate = "180deg";
    },
  });

  presentation.contPageNodeSectionRel =
    /** @type {Presentation["contPageNodeSectionRel"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            ss(uniq("cont_page_edit_section_rel"), {
              "": (s) => {
                s.gap = varSNodeGap;
              },
            }),
          ],
          children_: args.children,
        }
      ),
    });
  presentation.contNodeRowIncoming =
    /** @type {Presentation["contNodeRowIncoming"]} */ (args) => {
      const vboxStyles = [contVboxStyle, leafNodeVboxStyle];
      if (args.new) {
        vboxStyles.push(leafNodeVBoxNewStyle);
      }
      return {
        root: e(
          "div",
          {},
          {
            styles_: [contHboxStyle, leafNodeHboxStyle],
            children_: [
              e(
                "div",
                {},
                {
                  styles_: vboxStyles,
                  children_: args.children,
                }
              ),
              leafIcon({
                text: textIconRelIn,
                extraStyles: [
                  leafIconStyle,
                  leafNodeRelStyle,
                  leafNodeRelIncomingStyle,
                ],
              }),
            ],
          }
        ),
      };
    };

  presentation.contNodeRowOutgoing =
    /** @type {Presentation["contNodeRowOutgoing"]} */ (args) => {
      const vboxStyles = [contVboxStyle, leafNodeVboxStyle];
      if (args.new) {
        vboxStyles.push(leafNodeVBoxNewStyle);
      }
      return {
        root: e(
          "div",
          {},
          {
            styles_: [contHboxStyle, leafNodeHboxStyle],
            children_: [
              leafIcon({
                text: textIconRelOut,
                extraStyles: [
                  leafIconStyle,
                  leafNodeRelStyle,
                  leafNodeRelOutgoingStyle,
                ],
              }),
              e(
                "div",
                {},
                {
                  styles_: vboxStyles,
                  children_: args.children,
                }
              ),
            ],
          }
        ),
      };
    };

  presentation.contNodeSectionCenter =
    /** @type {Presentation["contNodeSectionCenter"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            leafNodeVboxStyle,
            ss(uniq("cont_page_edit_center"), {
              // increase specifity...
              [`.${leafNodeVboxStyle}`]: (s) => {
                s.border = "none";
              },
              "": (s) => {
                s.padding = varPSmall;
                s.backgroundColor = varCBackground2;
                s.borderRadius = varRNode;
                s.margin = `${varPNodeCenter} 0`;
              },
            }),
          ],
          children_: args.children,
        }
      ),
    });
  const leafButtonEditFreeStyle = ss(uniq("leaf_button_free"), {
    "": (s) => {
      s.borderRadius = varRNodeButton;
      s.color = `color-mix(in srgb, ${varCForeground}, transparent 30%)`;
      s.width = varSNodeButton;
      s.maxWidth = varSNodeButton;
      s.height = varSNodeButton;
      s.maxHeight = varSNodeButton;
    },
    ":hover": (s) => {
      s.color = varCForeground;
    },
    ":hover:active": (s) => {
      s.color = varCForeground;
    },
    [`.${classStatePressed}`]: (s) => {
      s.color = varCModified;
    },
  });
  const leafButtonEditFree =
    /** @type { (args: { icon: string, hint: string }) => { root: HTMLElement } } */ (
      args
    ) =>
      leafButton({
        title: args.hint,
        icon: args.icon,
        extraStyles: [leafButtonEditFreeStyle],
      });
  const leafButtonEditFreeLink =
    /** @type { (args: { icon: string, hint: string, url: string, download?: boolean }) => { root: HTMLElement } } */ (
      args
    ) =>
      leafButtonLink({
        title: args.hint,
        icon: args.icon,
        url: args.url,
        download: args.download,
        extraStyles: [leafButtonEditFreeStyle],
      });

  const leafNodeButtons =
    /** @type { (args: {children: Element[]})=>{root: Element}} */ (args) => {
      return {
        root: e(
          "div",
          {},
          {
            styles_: [contHboxStyle, contEditNodeHboxStyle],
            children_: args.children,
          }
        ),
      };
    };

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, node view

  presentation.leafNodeViewNodeButtons =
    /** @type {Presentation["leafNodeViewNodeButtons"]} */ (args) => {
      const children = [presentation.leafSpace({}).root];
      if (args.edit != null) {
        children.push(
          leafButtonEditFreeLink({
            icon: textIconEdit,
            hint: "Edit",
            url: args.edit,
            download: false,
          }).root
        );
      }
      if (args.history != null) {
        children.push(
          leafButtonEditFreeLink({
            icon: textIconHistory,
            hint: "History",
            url: args.history,
            download: false,
          }).root
        );
      }
      if (args.download != null) {
        children.push(
          leafButtonEditFreeLink({
            icon: textIconSave,
            hint: "Save",
            url: args.download,
            download: true,
          }).root
        );
      }
      return {
        root: e(
          "div",
          {},
          {
            styles_: [contHboxStyle, contEditNodeHboxStyle],
            children_: children,
          }
        ),
      };
    };
  presentation.leafNodeViewNodeText =
    /** @type {Presentation["leafNodeViewNodeText"]} */ (args) => {
      const out = e(
        "a",
        { textContent: args.value },
        {
          styles_: [
            ss(uniq("leaf_node_view_node_text"), {
              "": (s) => {
                s.whiteSpace = "pre-wrap";
                s.overflowWrap = "anywhere";
                s.flexShrink = "1";
              },
            }),
            leafLinkStyle,
          ],
        }
      );
      if (args.link != null) {
        out.href = args.link;
      }
      return {
        root: out,
      };
    };

  presentation.leafNodeViewPredicate =
    /** @type {Presentation["leafNodeViewPredicate"]} */ (args) => ({
      root: e(
        "p",
        { textContent: args.value },
        {
          styles_: [
            ss(uniq("leaf_node_view_predicate"), {
              "": (s) => {
                s.opacity = varONodePredicate;
                s.whiteSpace = "pre-wrap";
                s.overflowWrap = "anywhere";
              },
            }),
          ],
        }
      ),
    });

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, node edit

  const leafButtonNodeEditAdd =
    /** @type {(args: {hint: string})=>{root: Element}} */ (args) =>
      leafButtonEditFree({ icon: textIconAdd, hint: args.hint });
  presentation.contNodeRowIncomingAdd =
    /** @type {Presentation["contNodeRowIncomingAdd"]} */ (args) => {
      const button = leafButtonNodeEditAdd({ hint: args.hint }).root;
      return {
        root: presentation.contNodeRowIncoming({
          children: [
            leafNodeButtons({
              children: [leafSpace({}).root, button, leafSpace({}).root],
            }).root,
          ],
          new: true,
        }).root,
        button: button,
      };
    };
  presentation.contNodeRowOutgoingAdd =
    /** @type {Presentation["contNodeRowOutgoingAdd"]} */ (args) => {
      const button = leafButtonNodeEditAdd({ hint: args.hint }).root;
      return {
        root: presentation.contNodeRowOutgoing({
          children: [
            leafNodeButtons({
              children: [leafSpace({}).root, button, leafSpace({}).root],
            }).root,
          ],
          new: true,
        }).root,
        button: button,
      };
    };

  presentation.leafNodeEditButtons =
    /** @type {Presentation["leafNodeEditButtons"]} */ (args) => {
      const buttonDelete = leafButtonEditFree({
        icon: textIconDelete,
        hint: "Delete",
      });
      const buttonRevert = leafButtonEditFree({
        icon: textIconRevert,
        hint: "Revert",
      });
      return {
        root: e(
          "div",
          {},
          {
            styles_: [contHboxStyle, contEditNodeHboxStyle],
            children_: [
              presentation.leafSpace({}).root,
              buttonDelete.root,
              buttonRevert.root,
            ],
          }
        ),
        buttonDelete: buttonDelete.root,
        buttonRevert: buttonRevert.root,
      };
    };

  const contEditNodeHboxStyle = ss(uniq("cont_edit_node_hbox"), {
    "": (s) => {
      s.justifyContent = "stretch";
      s.gap = "0.2cm";
    },
  });
  presentation.leafNodeEditNode =
    /** @type {Presentation["leafNodeEditNode"]} */ (args) => {
      return {
        root: e(
          "div",
          {
            onclick: () => {
              for (const i of args.inputValue.getElementsByTagName("span")) {
                i.focus();
              }
            },
          },
          {
            styles_: [
              leafInputBorderStyle,
              ss(uniq("leaf_node_edit_node"), {
                "": (s) => {
                  s.pointerEvents = "initial";
                  s.padding = varSInputPad;
                  s.cursor = "text";
                },
                ">select": (s) => {
                  s.marginRight = varPSmall;
                },
                " span": (s) => {
                  s.borderBottom = "none";
                  s.padding = "0";
                },
                " select": (s) => {
                  s.borderBottom = "none";
                  s.padding = "0";
                },
              }),
            ],
            children_: [args.inputType, args.inputValue],
          }
        ),
      };
    };

  presentation.leafNodeEditPredicate =
    /** @type {Presentation["leafNodeEditPredicate"]} */ (args) =>
      presentation.leafInputText({
        id: undefined,
        title: "Predicate",
        value: args.value,
      });

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, history
  presentation.contPageHistory =
    /** @type {Presentation["contPageHistory"]} */ (args) => {
      const body = e(
        "div",
        {},
        {
          styles_: [contBodyNarrowStyle, contVboxStyle],
          children_: args.children,
        }
      );
      return {
        root: presentation.contGroup({
          children: [
            presentation.contBarMain({
              leftChildren: [],
              leftMidChildren: [],
              midChildren: [],
              rightMidChildren: [],
              rightChildren: args.barChildren,
            }).root,
            e(
              "div",
              {},
              {
                styles_: [classMenuWantStateOpen, contVboxStyle, contBodyStyle],
                children_: [body],
              }
            ),
          ],
        }).root,
        body: body,
      };
    };

  presentation.contHistoryCommit =
    /** @type {Presentation["contHistoryCommit"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            ss(uniq("cont_history_commit"), {
              "": (s) => {
                s.marginBottom = varPHistoryBig;
              },
              ":not(:first-child)": (s) => {
                s.marginTop = `calc(2 * ${varPHistoryBig})`;
              },
            }),
          ],
          children_: [
            e(
              "h2",
              { textContent: new Date(args.stamp).toLocaleString() },
              {
                styles_: [
                  ss(uniq("leaf_history_commit"), {
                    "": (s) => {
                      s.fontSize = varFPageTitle;
                    },
                  }),
                ],
              }
            ),
            e("p", { textContent: args.desc }, {}),
          ],
        }
      ),
    });
  presentation.contHistorySubject =
    /** @type {Presentation["contHistorySubject"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            ss(uniq("cont_history_subject"), {
              "": (s) => {
                s.marginTop = varPHistoryMid;
              },
            }),
          ],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [
                  contVboxStyle,
                  ss(uniq("leaf_history_subject"), {
                    "": (s) => {
                      s.padding = `${varPSmall} 0`;
                    },
                  }),
                ],
                children_: args.center,
              }
            ),
            e(
              "div",
              {},
              {
                styles_: [
                  contVboxStyle,
                  ss(uniq("cont_history_subject_rows"), {
                    "": (s) => {
                      s.gap = varPHistoryMid;
                    },
                  }),
                ],
              }
            ),
          ],
        }
      ),
    });
  const contHistoryPredicateObject =
    /** @type {(args: {icon: Element, button?: Element, children: Element[]})=>{root: Element}} */ (
      args
    ) => {
      const children = /** @type {Element[]} */ ([
        args.icon,
        e(
          "div",
          {},
          {
            styles_: [
              contVboxStyle,
              ss(uniq("cont_history_rel_center"), {
                "": (s) => {
                  s.flexBasis = "0";
                  s.flexGrow = "1";
                  s.gap = varPSmall;
                },
              }),
            ],
            children_: args.children,
          }
        ),
      ]);
      if (args.button != null) {
        children.push(args.button);
      }
      return {
        root: e(
          "div",
          {},
          {
            styles_: [
              contHboxStyle,
              ss(uniq("cont_history_row_hbox"), {
                "": (s) => {
                  s.gap = varPHistoryMid;
                },
              }),
            ],
            children_: children,
          }
        ),
      };
    };
  const leafHistoryRevertButton = () =>
    e(
      "button",
      {},
      {
        styles_: [
          leafButtonStyle,
          ss(uniq("leaf_history_revert"), {
            "": (s) => {
              s.alignSelf = "center";
              s.width = varSHistPredObj;
              s.minWidth = varSHistPredObj;
              s.height = varSHistPredObj;
            },
            ">svg": (s) => {
              s.width = "100%";
              s.height = "100%";
            },
            [`.${classStatePressed}`]: (s) => {
              s.color = varCModified;
            },
          }),
        ],
        children_: [
          leafIcon({
            text: textIconRevert,
          }),
        ],
      }
    );
  presentation.contHistoryPredicateObjectRemove =
    /** @type {Presentation["contHistoryPredicateObjectRemove"]} */ (args) => {
      const revertButton = leafHistoryRevertButton();
      return {
        root: contHistoryPredicateObject({
          icon: leafIcon({
            text: textIconDelete,
            extraStyles: [
              ss(uniq("cont_history_row_rel_icon"), {
                "": (s) => {
                  s.color = varCRemove;
                  s.opacity = varONoninteractiveLight;
                  s.fontSize = varFRelIcon;
                  s.fontWeight = varWRelIcon;
                },
              }),
              leafNodeRelOutgoingStyle,
            ],
          }),
          button: revertButton,
          children: args.children,
        }).root,
        button: revertButton,
      };
    };
  presentation.contHistoryPredicateObjectAdd =
    /** @type {Presentation["contHistoryPredicateObjectAdd"]} */ (args) => {
      const revertButton = leafHistoryRevertButton();
      return {
        root: contHistoryPredicateObject({
          icon: leafIcon({
            text: textIconAdd,
            extraStyles: [
              ss(uniq("cont_history_row_rel_icon"), {
                "": (s) => {
                  s.opacity = varONoninteractiveLight;
                  s.fontSize = varFRelIcon;
                  s.fontWeight = varWRelIcon;
                },
              }),
              leafNodeRelOutgoingStyle,
            ],
          }),
          button: revertButton,
          children: args.children,
        }).root,
        button: revertButton,
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, query

  presentation.contPageQuery = /** @type {Presentation["contPageQuery"]} */ (
    args
  ) => {
    const blockWidth = `min(10cm, 100%)`;
    const query = e(
      "div",
      { contentEditable: "plaintext-only", textContent: args.initialQuery },
      {
        styles_: [
          ss(uniq("cont_page_query_query"), {
            "": (s) => {
              s.minWidth = blockWidth;
              s.pointerEvents = "initial";
              s.flexBasis = "1cm";
              s.flexGrow = "1";
              s.fontFamily = "monospace";
              s.fontSize = varFCode;
              s.whiteSpace = "pre-wrap";
              s.overflowWrap = "anywhere";
            },
          }),
        ],
      }
    );
    const results = e(
      "div",
      {},
      {
        styles_: [
          contVboxStyle,
          ss(uniq("cont_page_query_results"), {
            "": (s) => {
              s.minWidth = blockWidth;
              s.flexBasis = "1cm";
              s.flexGrow = "1";
              s.fontFamily = "monospace";
              s.fontSize = varFCode;
              s.gap = varPSmall;
            },
            ":empty::before": (s) => {
              s.display = "grid";
              s.gridTemplateRows = "1fr";
              s.gridTemplateColumns = "1fr";
              s.justifyItems = "center";
              s.alignItems = "center";

              s.opacity = varONoninteractive;
              s.content = '"No results"';
              s.flexGrow = "1";
            },
          }),
        ],
      }
    );
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            contBodyStyle,
            ss(uniq("cont_page_query"), {
              "": (s) => {
                s.flexGrow = "1";

                s.display = "flex";
                s.flexWrap = "wrap";
                s.justifyContent = "stretch";

                s.margin = `0 calc(${varSCol1Width} * 2)`;
              },
            }),
          ],
          children_: [query, results],
        }
      ),
      query: query,
      results: results,
    };
  };

  presentation.leafQueryRow = /** @type {Presentation["leafQueryRow"]} */ (
    args
  ) => ({
    root: e(
      "div",
      { textContent: args.data },
      {
        styles_: [
          ss(uniq("leaf_query_row"), {
            "": (s) => {
              s.pointerEvents = "initial";
              s.padding = varPSmall;
              s.whiteSpace = "pre";
            },
            ":nth-child(odd)": (s) => {
              s.backgroundColor = varCBackground2;
              s.borderRadius = varRMedia;
            },
          }),
        ],
      }
    ),
  });

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu

  const contMenuGroupVBoxStyle = ss(uniq("cont_menu_group0"), {
    "": (s) => {
      s.marginLeft = "0.6cm";
      s.gap = "0.3cm";
    },
  });

  presentation.contBarMenu = /** @type {Presentation["contBarMenu"]} */ (
    args
  ) =>
    presentation.contBar({
      extraStyles: [
        ss(uniq("cont_bar_menu"), {
          "": (s) => {
            s.gridColumn = "1/3";

            s.backgroundColor = varCBackgroundMenuBar;
            s.margin = `${varPMenu} 0`;
          },
        }),
      ],
      leftChildren: [],
      leftMidChildren: [],
      midChildren: [],
      rightMidChildren: [],
      rightChildren: args.children,
    });
  presentation.leafMenuBarButtonLogin =
    /** @type {Presentation["leafMenuBarButtonLogin"]} */ (args) =>
      presentation.leafButtonBig({
        title: "Login",
        icon: textIconLogin,
        text: "Login",
        extraStyles: [],
      });
  presentation.leafMenuBarButtonLogout =
    /** @type {Presentation["leafMenuBarButtonLogout"]} */ (args) =>
      presentation.leafButtonBig({
        title: "Logout",
        icon: textIconLogout,
        text: "Logout",
        extraStyles: [],
      });

  presentation.contMenuGroup = /** @type {Presentation["contMenuGroup"]} */ (
    args
  ) => ({
    root: e(
      "details",
      {},
      {
        styles_: [
          ss(uniq("cont_menu_group"), {
            [`>.${contMenuGroupVBoxStyle}`]: (s) => {
              s.padding = `${varPMenu} 0`;
            },
            ">summary": (s) => {
              s.listStyle = "none";
              s.position = "relative";
              s.display = "flex";
              s.flexDirection = "row";
              s.alignContent = "center";
              s.justifyContent = "flex-start";
              s.fontSize = varFMenu;
            },
            ">summary>.icon": (s) => {
              s.fontSize = varFMenuIcon;
              s.width = varSMenuIndent;
              s.opacity = varONoninteractive;
            },
            ">summary:hover>.icon": (s) => {
              s.opacity = "1";
            },
            ">summary>.icon.open": (s) => {
              s.display = "none";
            },
            "[open]>summary>.icon.closed": (s) => {
              s.display = "none";
            },
            "[open]>summary>.icon.open": (s) => {
              s.display = "grid";
            },
          }),
        ],
        children_: [
          e(
            "summary",
            {},
            {
              styles_: [],
              children_: [
                leafIcon({
                  text: textIconFoldClosed,
                  extraStyles: ["icon", "closed"],
                }),
                leafIcon({
                  text: textIconFoldOpened,
                  extraStyles: ["icon", "open"],
                }),
                e("span", { textContent: args.title }, {}),
              ],
            }
          ),
          e(
            "div",
            {},
            {
              styles_: [contVboxStyle, contMenuGroupVBoxStyle],
              children_: args.children,
            }
          ),
        ],
      }
    ),
  });

  presentation.leafMenuLink = /** @type {Presentation["leafMenuLink"]} */ (
    args
  ) => {
    return {
      root: e(
        "div",
        {},
        {
          children_: [
            e(
              "a",
              {
                href: args.href,
              },
              {
                styles_: [
                  leafLinkStyle,
                  ss(uniq("leaf_menu_link"), {
                    "": (s) => {
                      s.marginLeft = varSMenuIndent;
                      s.fontSize = varFMenu;

                      s.display = "flex";
                      s.flexDirection = "row";
                      s.alignItems = "center";
                      s.justifyContent = "flex-start";
                    },
                  }),
                ],
                children_: [
                  e(
                    "span",
                    { textContent: args.title },
                    {
                      styles_: [
                        ss(uniq("leaf_menu_link_text"), {
                          "": (s) => {
                            s.flexShrink = "1";
                          },
                        }),
                      ],
                    }
                  ),
                ],
              }
            ),
          ],
        }
      ),
    };
  };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: Main

  presentation.contMenuBody = /** @type {Presentation["contMenuBody"]} */ (
    args
  ) => ({
    root: e(
      "div",
      {},
      {
        styles_: [
          ss(uniq("cont_menu1"), {
            "": (s) => {
              s.display = "grid";
              s.gridTemplateColumns = `${varSCol1Width} 1fr`;
              s.gridTemplateRows = "auto auto 1fr";
            },
          }),
        ],
        children_: [
          e(
            "div",
            {},
            {
              styles_: [
                classMenuWantStateOpen,
                contVboxStyle,
                contMenuGroupVBoxStyle,
                ss(uniq("cont_menu_body"), {
                  "": (s) => {
                    s.gridColumn = "2";

                    s.justifyContent = "start";
                    s.minHeight = `calc(100dvh - 5cm)`;
                  },
                  ">*": (s) => {
                    s.maxWidth = varSColWidth;
                  },
                  [`.${contMenuGroupVBoxStyle}`]: (s) => {
                    s.marginLeft = "0";
                  },
                }),
              ],
              children_: args.children,
            }
          ),
          presentation.contBarMenu({
            children: [
              e(
                "span",
                {
                  textContent: args.user,
                },
                {
                  styles_: [
                    ss(uniq("cont_bar_menu_user"), {
                      "": (s) => {
                        s.opacity = varOMenuBar;
                      },
                    }),
                  ],
                }
              ),
              ...args.barChildren,
            ],
          }).root,
          presentation.leafSpace({}).root,
        ],
      }
    ),
  });

  presentation.appMain = /** @type {Presentation["appMain"]} */ (args) => {
    const admenuButton = leafButton({
      title: "Menu",
      icon: textIconMenu,
      extraStyles: [
        leafIconStyle,
        ss(uniq("cont_main_title_admenu"), {
          "": (s) => {
            s.gridColumn = "3";
            s.gridRow = "1";
          },
          ">svg": (s) => {
            s.width = varSCol3Width;
            s.height = varSCol3Width;
            s.padding = "20%";
          },
        }),
      ],
    }).root;
    return {
      root: e(
        "div",
        {},
        {
          styles_: [contStackStyle],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [
                  ss(uniq("body"), {
                    "": (s) => {
                      s.display = "grid";
                      s.width = "100dvw";
                      s.maxWidth = "100dvw";
                      s.height = "100dvh";
                      s.pointerEvents = "initial";

                      s.overflowX = "hidden";
                      s.overflowY = "auto";
                      s.gridTemplateColumns = `${varSCol1Width} 1fr auto`;
                      s.gridTemplateRows = "auto 1fr";
                    },
                  }),
                ],
                children_: [
                  presentation.contTitle({
                    left: args.mainTitle,
                    right: admenuButton,
                  }).root,
                  args.mainBody,
                ],
              }
            ),
            e(
              "div",
              {
                id: "menu",
              },
              {
                styles_: [
                  classMenuWantStateOpen,
                  contVboxStyle,
                  ss(uniq("cont_menu"), {
                    "": (s) => {
                      s.zIndex = "3";
                      s.width = `calc(100dvw - ${varSCol3Width})`;
                      s.backgroundColor = varCBackgroundMenu;
                      s.filter =
                        "drop-shadow(0.05cm 0px 0.05cm rgba(0, 0, 0, 0.06))";

                      s.overflowX = "hidden";
                      s.overflowY = "auto";
                      s.position = "relative";
                      s.transition = "0.03s left";
                      s.pointerEvents = "initial";
                    },
                    [`.${classMenuStateOpen}`]: (s) => {
                      s.left = "0";
                    },
                    [`:not(.${classMenuStateOpen})`]: (s) => {
                      s.left = "-110dvw";
                    },
                  }),
                ],
                children_: [
                  presentation.contTitle({
                    left: e(
                      "div",
                      {},
                      {
                        styles_: [
                          contHboxStyle,
                          ss(uniq("menu_title"), {
                            "": (s) => {
                              s.paddingLeft = varSCol1Width;
                              s.alignItems = "center";
                              s.gap = varPAppTitle;
                            },
                          }),
                        ],
                        children_: [
                          e(
                            "h1",
                            { textContent: "sunwet" },
                            {
                              styles_: [
                                ss(uniq("menu_title_text"), {
                                  "": (s) => {
                                    s.fontSize = varFTitle;
                                    s.marginTop = "-0.15cm";
                                    s.color = varCAppTitle;
                                  },
                                }),
                              ],
                            }
                          ),
                        ],
                      }
                    ),
                  }).root,
                  args.menuBody,
                ],
              }
            ),
          ],
        }
      ),
      admenuButton: admenuButton,
    };
  };
  presentation.contBarMain = /** @type {Presentation["contBarMain"]} */ (
    args
  ) =>
    presentation.contBar({
      extraStyles: [
        classMenuWantStateOpen,
        contBarMainStyle,
        ss(uniq("cont_bar_main"), {
          "": (s) => {
            s.position = "relative";
            s.backdropFilter = "blur(0.2cm)";
          },
          "::before": (s) => {
            s.display = "block";
            s.content = '""';
            s.position = "absolute";
            s.left = "0";
            s.right = "0";
            s.top = "0";
            s.bottom = "0";
            s.backgroundColor = varCBackground;
            s.opacity = "0.5";
            s.zIndex = "-1";
          },
        }),
      ],
      leftChildren: args.leftChildren,
      leftMidChildren: args.leftMidChildren,
      midChildren: args.midChildren,
      rightMidChildren: args.rightMidChildren,
      rightChildren: args.rightChildren,
    });

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: Main

  presentation.appLinkPerms = /** @type {Presentation["appLinkPerms"]} */ (
    args
  ) => {
    const button = e(
      "button",
      {},
      {
        styles_: [
          ss(uniq("app_link_perms"), {
            "": (s) => {
              s.display = "grid";
              s.gridTemplateColumns = "1fr";
              s.gridTemplateRows = "1fr auto 1fr";
              s.justifyItems = "center";
              s.margin = varPLink;
              s.borderRadius = varRLink;
              s.border = `${varLMid} solid ${varCForegroundFade}`;
            },
            ":hover": (s) => {
              s.borderColor = varCButtonHover;
            },
            ":hover:active": (s) => {
              s.borderColor = varCButtonClick;
            },
            ">*:nth-child(1)": (s) => {
              s.gridRow = "2";
            },
            ">*:nth-child(2)": (s) => {
              s.gridRow = "3";
            },
          }),
        ],
        children_: [
          leafLogoText({
            extraStyles: [
              ss(uniq("app_link_perms_logo"), {
                " text": (s) => {
                  s.fill = varCLinkLogoText;
                  s.fontWeight = varWLinkLogoText;
                },
              }),
            ],
          }),
          leafIcon({
            text: textIconPlay,
            extraStyles: [
              ss(uniq("app_link_perms_play"), {
                "": (s) => {
                  s.width = varSLinkIcon;
                  s.minWidth = varSLinkIcon;
                  s.height = varSLinkIcon;
                  s.minHeight = varSLinkIcon;
                  s.alignSelf = "end";
                },
                " text": (s) => {
                  s.fill = varCLinkForegroundDark;
                  s.fontWeight = varWLight;
                },
              }),
            ],
          }),
        ],
      }
    );
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            contStackStyle,
            ss(uniq("app_link_perms_bg"), {
              "": (s) => {
                s.backgroundColor = varCLinkBackground;
              },
            }),
          ],
          children_: [button],
        }
      ),
      button: button,
    };
  };

  presentation.appLink = /** @type {Presentation["appLink"]} */ (args) => {
    const display = e(
      "div",
      {},
      {
        styles_: [
          ss(uniq("cont_app_link_display"), {
            "": (s) => {
              // Fit/width/height ignored otherwise...
              s.overflow = "hidden";
            },
            ">*": (s) => {
              s.objectFit = "contain";
              s.aspectRatio = "auto";
              s.width = "100%";
              s.height = "100%";
            },
          }),
        ],
        children_: [],
      }
    );
    const hideableStyle = ss(uniq("app_link_display_hideable"), {
      [`.${classStateHide}`]: (s) => {
        s.display = "none";
      },
    });
    const displayUnder = et(
      `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 164.51 164.51">
  <path fill="none" stroke="currentColor" stroke-dasharray="511.84,12.796,31.99,12.796,25.592,12.796,19.194,12.796,12.796,12.796,6.398" stroke-width="6.4" d="M94.57 99.8A25.29 25.29 0 0 0 81 96.03c-11.64 0-21.08 7.17-21.08 16.01 0 8.84 9.44 16.01 21.08 16.01 11.65 0 21.08-7.17 21.08-16.01V26.71c-.04-11.8 6.87-19.1 21.8-11.18 23.03 14.39 37.43 38.87 37.43 66.72A79.05 79.05 0 1 1 82.25 3.2" paint-order="fill markers stroke"/>
</svg>
      `,
      {
        styles_: [
          hideableStyle,
          classStateHide,
          ss(uniq("leaf_app_display_under"), {
            "": (s) => {
              s.padding = varPSmall;
              s.color = varCLinkBackground;
              s.objectFit = "contain";
              s.aspectRatio = "auto";
              s.width = "100%";
              s.height = "100%";
            },
          }),
        ],
      }
    );
    const displayOver = presentation.leafSpinner({
      extraStyles: [
        hideableStyle,
        ss(uniq("leaf_app_display_over"), {
          "": (s) => {
            s.justifySelf = "center";
            s.alignSelf = "center";
          },
        }),
      ],
    }).root;
    const displayStack = e(
      "div",
      {},
      {
        styles_: [
          contStackStyle,
          ss(uniq("app_link_display_stack"), {
            "": (s) => {
              s.flexShrink = "1";

              s.backgroundColor = varCLinkDisplayBg;

              s.paddingTop = varPSmall;
              s.paddingBottom = varPSmall;
              s.overflow = "hidden";
            },
          }),
        ],
        children_: [
          e(
            "div",
            {},
            {
              styles_: [
                ss(uniq("leaf_app_display_under2"), {
                  "": (s) => {
                    s.aspectRatio = "1/1";
                    // Ignores max height/width otherwise... why?
                    s.overflow = "hidden";
                    s.width = "100%";
                    s.maxWidth = "100%";
                    s.height = "100%";
                    s.maxHeight = "100%";
                  },
                }),
              ],
            }
          ),
          displayUnder,
          display,
          displayOver,
        ],
      }
    );
    const textStyle = ss(uniq("leaf_app_link_text"), {
      "": (s) => {
        s.color = varCLinkForegroundDark;
        s.pointerEvents = "initial";
      },
    });
    /** @type { (text:string)=>HTMLElement} */
    const bigText = (text) =>
      e(
        "span",
        { textContent: text },
        {
          styles_: [
            textStyle,
            ss(uniq("leaf_app_link_bigtext"), {
              "": (s) => {
                s.fontSize = varFLinkBig;
              },
            }),
          ],
        }
      );
    const title = bigText("Waiting...");
    const albumArtist = e(
      "span",
      { textContent: "" },
      { styles_: [textStyle] }
    );
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            ss(uniq("app_link"), {
              "": (s) => {
                s.backgroundColor = varCLinkBackground;
                s.gap = varPLinkGap;
                s.height = "100dvh";
                s.width = "100dvw";
              },
              ">*": (s) => {
                s.flexShrink = "0";
              },
            }),
          ],
          children_: [
            leafSpace({}).root,
            displayStack,
            e(
              "div",
              {},
              {
                styles_: [
                  contVboxStyle,
                  ss(uniq("cont_app_link_vert"), {
                    "": (s) => {
                      s.paddingLeft = varPLinkText;
                      s.paddingRight = varPLinkText;
                      s.gap = varPSmall;
                    },
                  }),
                ],
                children_: [title, albumArtist],
              }
            ),
            leafSpace({}).root,
          ],
        }
      ),
      displayUnder: displayUnder,
      display: display,
      displayOver: displayOver,
      albumArtist: albumArtist,
      title: title,
    };
  };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Assemble

  window.sunwetPresentation = presentation;

  addEventListener("DOMContentLoaded", async (_) => {
    const resetStyle = e(
      "link",
      { rel: "stylesheet", href: "style_reset.css" },
      {}
    );
    document.head.appendChild(resetStyle);
    const htmlStyle = ss(uniq("html"), {
      "": (s) => {
        s.fontFamily = "X";
        s.backgroundColor = varCBackground;
        s.color = varCForeground;
      },
    });
    notnull(document.body.parentElement).classList.add(htmlStyle);
    document.body.classList.add(contStackStyle);
  });
}
