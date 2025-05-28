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

  const s = /** @type {(
    id: string,
    f: { [s: string]: (r: CSSStyleDeclaration) => void }
  ) => string} */ (id, f) => {
    for (const [suffix, f1] of Object.entries(f)) {
      globalStyle.insertRule(`.${id}${suffix} {}`, 0);
      f1(/** @type { CSSStyleRule } */ (globalStyle.cssRules[0]).style);
    }
    return id;
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

  const textIconClick = "\uf718";
  const textIconPlay = "\ue037";
  const textIconPause = "\ue034";
  const textIconDelete = "\ue15b";
  const textIconRevert = "\ue166";
  const textIconApply = "\uf4f6";
  const textIconAdd = "\ue145";
  const textIconNext = "\ue5cc";
  const textIconPrev = "\ue5cb";
  const textIconLink = "\ue157";
  const textIconSave = "\ue161";
  const textIconUnlink = "\ue16f";
  const textIconLogin = "\uea77";
  const textIconLogout = "\ue9ba";
  const textIconMenu = "\ue5d2";
  const textIconGo = "\ue5c8";
  const textIconFoldClosed = "\ue316";
  const textIconFoldOpened = "\ue313";
  const textIconClose = "\ue5cd";
  const textIconRelIn = "\uf72d";
  const textIconRelOut = "\uf72e";
  const textIconEdit = "\ue3c9";
  const textIconView = "\ue8f4";

  // xx Variables

  const varSPadTitle = v(uniq(), "0.4cm");
  const varSFontTitle = v(uniq(), "24pt");
  const varSFontAdmenu = v(uniq(), "20pt");
  const varSFontMenu = v(uniq(), "20pt");
  const varSFontPageTitle = v(uniq(), "18pt");
  const varSNarrow = v(uniq(), "20cm");
  const varSModalHeight = v(uniq(), "20cm");
  const varSCol1Width = v(uniq(), "min(0.8cm, 5dvw)");
  const varSCol3Width = v(uniq(), "1.4cm");
  const varSMenuColWidth = v(uniq(), "min(100%, 12cm)");

  //const varCBackground = vs("rgb(205, 207, 212)", "rgb(0,0,0)");
  const varCBackground = vs(uniq(), "rgb(230, 232, 238)", "rgb(0,0,0)");
  const varCBg2 = vs(uniq(), "rgb(218, 220, 226)", "rgb(0,0,0)");
  const varCBackgroundDark = v(uniq(), "rgb(45, 45, 46)");
  //const varCBackgroundMenu = vs(uniq(), "rgb(173, 177, 188)", "rgb(0,0,0)");
  const varCBackgroundMenu = vs(uniq(), "rgb(205, 208, 217)", "rgb(0,0,0)");
  const varCBackgroundMenuButtons = vs(
    uniq(),
    "rgb(219, 223, 232)",
    "rgb(0,0,0)"
  );

  const varCButtonHover = vs(uniq(), "rgba(255, 255, 255, 0.7)", "rgb(0,0,0)");
  const varCButtonClick = vs(uniq(), "rgba(255, 255, 255, 1)", "rgb(0,0,0)");

  const varCSeekbarEmpty = vs(uniq(), "rgb(212, 216, 223)", "rgb(0,0,0)");

  const varSButtonPadBig = v(uniq(), "0.3cm");
  const varSButtonPadSmall = v(uniq(), "0.1cm");

  const varCForeground = vs(uniq(), "rgb(0, 0, 0)", "rgb(0,0,0)");
  const varCForegroundFade = vs(uniq(), "rgb(123, 123, 123)", "rgb(0,0,0)");
  const varCForegroundDark = v(uniq(), "rgb(249, 248, 240)");
  const varCForegroundError = vs(uniq(), "rgb(154, 60, 74)", "rgb(0,0,0)");
  const varCBorderModified = vs(uniq(), "rgb(120, 149, 235)", "rgb(0,0,0)");
  const varCBorderError = vs(uniq(), "rgb(192, 61, 80)", "rgb(0,0,0)");
  const varCInputBorder = vs(uniq(), "rgb(154, 157, 168)", "rgb(0,0,0)");
  const varCSpinner = v(uniq(), "rgb(155, 178, 229)");
  const varCHighlightBold = vs(uniq(), "rgb(62, 119, 251)", "rgb(0,0,0)");
  const varCNodeCenter = varCBg2;
  const varCRemove = vs(uniq("remove"), "rgb(216, 0, 0)", "rgb(0,0,0)");

  // xx State classes

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
  const classStateDeleted = "deleted";
  presentation.classStateDeleted =
    /** @type { Presentation["classStateDeleted"]} */ () => ({
      value: classStateDeleted,
    });
  const classStatePlaying = "playing";
  presentation.classStatePlaying =
    /** @type { Presentation["classStatePlaying"]} */ () => ({
      value: classStatePlaying,
    });
  const classStateSharing = "sharing";
  presentation.classStateSharing =
    /** @type { Presentation["classStateSharing"]} */ () => ({
      value: classStateSharing,
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

  const contTitleStyle = s(uniq("cont_title"), {
    "": (s) => {
      s.gridColumn = "1/4";
      s.gridRow = "1";
      s.margin = `${varSPadTitle} 0`;
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

  const leafTitleStyle = s(uniq("leaf_title"), {
    "": (s) => {
      s.fontSize = varSFontTitle;
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

  const leafIconStyle = s(uniq("icon"), {
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
                s.gap = "0.2cm";
                s.margin = `0 0.2cm`;
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
      s.bottom = "0.7cm";

      s.transition = "0.03s opacity";
      s.opacity = "1";
    },
    [`.${classMenuStateOpen}`]: (s) => {
      s.opacity = "0";
    },
  });

  presentation.contBarMainForm =
    /** @type {Presentation["contBarMainForm"]} */ (args) =>
      presentation.contBar({
        extraStyles: [
          classMenuWantStateOpen,
          contBarMainStyle,
          ss(uniq("cont_bar_main_form"), {
            "": (s) => {
              s.backdropFilter = "brightness(1.06) blur(0.2cm)";
            },
          }),
        ],
        leftChildren: args.leftChildren,
        leftMidChildren: args.leftMidChildren,
        midChildren: args.midChildren,
        rightMidChildren: args.rightMidChildren,
        rightChildren: args.rightChildren,
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
      root: e(
        "div",
        {},
        {
          styles_: [contGroupStyle],
          children_: [inner],
        }
      ),
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

  const leafSpaceStyle = s(uniq("leaf_space"), {
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
      s.gap = "0.2cm";
    },
    [`.${classStateDisabled}`]: (s) => {
      s.opacity = "0.5";
    },
    [`:not(.${classStateDisabled}):hover`]: (s) => {
      s.backgroundColor = varCButtonHover;
    },
    [`:not(.${classStateDisabled}):hover:active`]: (s) => {
      s.backgroundColor = varCButtonClick;
    },
    [`:not(.${classStateDisabled}).${classStateThinking}`]: (s) => {
      // TODO horiz line with marching dashes instead
      s.opacity = "0.5";
    },
  });
  const leafButton = /** @type {
    (args: { title: string, icon?: string, text?: string, extraStyles: string[] }) => { root: HTMLElement }
  } */ (args) => {
    const children = [];
    if (args.icon != null) {
      children.push(
        e("div", { textContent: args.icon }, { styles_: [leafIconStyle] })
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
    (args: { title: string, icon?: string, text?: string, extraStyles: string[], url: string }) => { root: HTMLElement }
  } */ (args) => {
    const children = [];
    if (args.icon != null) {
      children.push(
        e("div", { textContent: args.icon }, { styles_: [leafIconStyle] })
      );
    }
    if (args.text != null) {
      children.push(e("span", { textContent: args.text }, {}));
    }
    return {
      root: e(
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
      ),
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
              s.padding = varSButtonPadBig;
            },
          }),
        ],
      });
  presentation.leafButtonBigSave =
    /** @type { Presentation["leafButtonBigSave"] } */
    (args) =>
      presentation.leafButtonBig({
        title: "Save",
        icon: textIconSave,
        text: "Save",
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
              s.padding = `${varSButtonPadSmall} ${varSButtonPadBig}`;
              s.color = varCForegroundFade;
            },
          }),
        ],
      });

  presentation.leafButtonSmallEdit =
    /** @type {Presentation["leafButtonSmallEdit"]} */ (args) =>
      leafButtonLinkSmall({
        title: "Edit",
        icon: textIconEdit,
        text: "Edit",
        url: args.link,
      });
  presentation.leafButtonSmallView =
    /** @type {Presentation["leafButtonSmallView"]} */ (args) =>
      leafButtonLinkSmall({
        title: "View",
        icon: textIconView,
        text: "View",
        url: args.link,
      });

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: home
  const leafLogoText =
    /** @type {(args: {extraStyles?:string[]})=>HTMLElement} */ (args) =>
      et(
        `
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 210.77">
              <path fill="#fefefe" d="M187.6 100.09a69.05 69.05 0 0 0-68.8-63.7A69.05 69.05 0 0 0 57 74.7c4.35-.17 8.86-.06 13.54.37a56.99 56.99 0 0 1 105.12 26.33 72.7 72.7 0 0 0 11.94-1.31zm-9.93 41.27c-4.6.16-9.37.03-14.31-.45a56.91 56.91 0 0 1-44.56 21.47 57.06 57.06 0 0 1-56.25-47.73c-4.14-.1-8.12.2-12.01.83a69 69 0 0 0 127.14 25.88Z"/>
              <path fill="none" stroke="#7ca7db" stroke-linecap="round" stroke-width="10" d="M5 110.87c20.49-9.6 40.98-19.2 68-15.39 27.02 3.81 60.58 21.04 88 25 27.42 3.97 48.71-5.32 70-14.6"/>
              <path fill="none" stroke="#fefefe" stroke-linecap="square" stroke-width="10" d="m34.52 44.15 12.13 8.81M86.6 6.3l4.64 14.27M151 6.3l-4.64 14.27m56.72 23.58-12.13 8.81m12.13 113.66-12.13-8.82M151 204.46l-4.64-14.26M86.6 204.46l4.64-14.26m-56.72-23.58 12.13-8.82"/>
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
                  s.fill = "#fefefe";
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
  const leafInputPairStyle = s(uniq("leaf_form_input_pair"), {
    "": (s) => {
      s.display = "contents";
    },
    ">*:nth-child(1)": (s) => {
      s.gridColumn = "1";
    },
    ">*:nth-child(2)": (s) => {
      s.gridColumn = "2";
      // Otherwise some inexplicable space is created above the element
      s.alignSelf = "start";
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
  const leafInputBorderStyle = s(uniq("leaf_input_border"), {
    "": (s) => {
      s.borderBottom = `0.04cm solid ${varCInputBorder}`;
    },
    [`.${classInputStateModified}`]: (s) => {
      s.borderColor = varCBorderModified;
      s.borderBottomWidth = "0.04cm";
    },
    [`.${classInputStateInvalid}`]: (s) => {
      s.borderColor = varCBorderError;
      s.borderBottomStyle = "dashed";
      s.borderBottomWidth = "0.06cm";
    },
  });
  const leafInputStyle = s(uniq("leaf_input"), {
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
            },
            ":empty::before": (s) => {
              s.whiteSpace = "pre-wrap";
              s.opacity = "0.5";
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
              media.appendChild(e("img", { src: url }, {}));
            }
            break;
          case "video":
            {
              media.appendChild(e("video", { src: url, controls: true }, {}));
            }
            break;
          case "audio":
            {
              media.appendChild(
                e(
                  "audio",
                  {
                    src: url,
                    controls: true,
                  },
                  {}
                )
              );
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
              s.fontWeight = "300";
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
                s.borderRadius = "0.05cm";
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
                s.opacity = "50%";
                s.justifySelf = "end";
                s.margin = "0 0.2cm";
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
                      s.borderRadius = "0.05cm";
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
        icons: { "": textIconPlay, [classStatePlaying]: textIconPause },
      });
      return {
        root: presentation.contBar({
          extraStyles: [
            classMenuWantStateOpen,
            contBarMainStyle,
            ss(uniq("cont_bar_main_transport"), {
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
                s.opacity = "0.3";
              },
            }),
          ],
          leftChildren: [buttonShare.root],
          leftMidChildren: [],
          midChildren: [buttonPrev.root, seekbar, buttonNext.root],
          rightMidChildren: [buttonPlay.root],
          rightChildren: [],
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
    children.push(
      e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            ss(uniq("view_list_body"), {
              "": (s) => {
                s.padding = `0 max(0.3cm, min(${varSCol1Width}, 100dvw / 20))`;
                s.paddingBottom = "2cm";
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
          styles_: [
            classMenuWantStateOpen,
            contVboxStyle,
            ss(uniq("cont_view_body"), {
              "": (s) => {
                s.gridColumn = "1/4";
                s.gridRow = "2";
              },
              [`.${classMenuStateOpen}`]: (s) => {
                s.display = "none";
              },
            }),
          ],
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
                s.gap = "0.8cm";
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
              s.columnGap = "0.5cm";
              s.rowGap = "0.3cm";
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
            "": (s) => {},
          }),
        ],
        children_: args.children,
      }
    );
    if (args.width != null) {
      out.style.width = `max(2cm, min(${args.width}, 100%))`;
    } else {
      out.style.flexGrow = "1";
    }
    return {
      root: out,
    };
  };

  presentation.contMediaFullscreen =
    /** @type {Presentation["contMediaFullscreen"]} */ (args) => {
      const buttonClose = e(
        "button",
        {
          textContent: textIconClose,
        },
        {
          styles_: [
            leafIconStyle,
            leafButtonStyle,
            ss(uniq("cont_media_fullscreen_close"), {
              "": (s) => {
                s.color = "white";
                const size = "1cm";
                s.width = size;
                s.height = size;
                s.fontSize = "20pt";
              },
            }),
          ],
        }
      );
      return {
        buttonClose: buttonClose,
        root: e(
          "div",
          {},
          {
            styles_: [
              contVboxStyle,
              ss(uniq("cont_fullscreen"), {
                "": (s) => {
                  s.backgroundColor = "black";
                  s.justifyContent = "stretch";
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
                      },
                    }),
                  ],
                  children_: [buttonClose],
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
      bg: HTMLElement,
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
              s.fontSize = "20pt";
              const size = "1.4cm";
              s.width = size;
              s.height = size;
              s.borderTopRightRadius = "0.2cm";
            },
          }),
        ],
      }
    );
    const bg = e(
      "div",
      {},
      {
        styles_: [
          ss(uniq("cont_modal_bg"), {
            "": (s) => {
              s.backgroundColor = "rgba(0,0,0,0.3)";
              s.pointerEvents = "initial";
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
                s.zIndex = "3";
                s.top = "0";
                s.bottom = "0";
                s.left = "0";
                s.right = "0";
              },
            }),
          ],
          children_: [
            bg,
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
                      s.borderRadius = "0.2cm";
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
                                  s.marginLeft = "0.5cm";
                                  s.fontSize = "24pt";
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
      bg: bg,
      buttonClose: buttonClose,
    };
  };

  const leafTransportButton =
    /** @type {(args: { title: string, icons: {[s: string]: string}, extraStyles?: string[] }) => { root: HTMLElement }} */
    (args) => {
      const size = "1cm";
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
        const parentState = (() => {
          if (state == "") {
            return "";
          } else {
            return `.${state}`;
          }
        })();
        for (const [otherState, _] of statePairs) {
          const childMark = (() => {
            if (state == "") {
              return "default";
            } else {
              return state;
            }
          })();
          if (otherState == state) {
            children.push(leafIcon({ text: icon, extraStyles: [childMark] }));
            continue;
          } else {
            buildStyle[`${parentState}>.${childMark}`] = (s) => {
              s.display = "none";
            };
          }
        }
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
                s.fontWeight = "100";
              },
            }),
            ss(uniq(...buildStyleId), buildStyle),
            ...(args.extraStyles || []),
          ],
          children_: children,
        }
      );
      return { root: out };
    };

  presentation.contModalViewShare =
    /** @type {Presentation["contModalViewShare"]} */ (args) => {
      const buttonUnshare = presentation.leafButtonBig({
        title: "Unlink",
        icon: textIconUnlink,
        text: `Unlink`,
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
        bg: out.bg,
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
      s.flexGrow = "1";
      s.display = "flex";
      s.gap = "0.3cm";
    },
    ".root": (s) => {
      s.gap = "0.8cm";
    },
    [`>.${classViewTransverseStart}`]: (s) => {
      s.alignSelf = "first baseline";
    },
    [`>.${classViewTransverseEnd}`]: (s) => {
      s.alignSelf = "last baseline";
    },
  });

  presentation.contViewList = /** @type { Presentation["contViewList"] } */ (
    args
  ) => {
    const out = e(
      "div",
      {},
      {
        styles_: [
          contViewListStyle,
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
    if (args.xScroll != null) {
      out.style.overflowX = "scroll";
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
        out.style.overflowX = "scroll";
      }
      return { root: out };
    };

  const viewLeafTransStyle =
    /** @type { (args:{orientation: Orientation, transAlign: TransAlign})=>string } */ (
      args
    ) =>
      ss(uniq("view_leaf_trans_style", args.orientation, args.transAlign), {
        "": (s) => {
          switch (trans(args.orientation)) {
            case "up":
              switch (args.transAlign) {
                case "start":
                  s.alignSelf = "end";
                  break;
                case "middle":
                  s.alignSelf = "middle";
                  break;
                case "end":
                  s.alignSelf = "start";
                  break;
              }
              break;
            case "down":
              switch (args.transAlign) {
                case "start":
                  s.alignSelf = "start";
                  break;
                case "middle":
                  s.alignSelf = "middle";
                  break;
                case "end":
                  s.alignSelf = "end";
                  break;
              }
              break;
            case "left":
              switch (args.transAlign) {
                case "start":
                  s.justifySelf = "end";
                  break;
                case "middle":
                  s.justifySelf = "middle";
                  break;
                case "end":
                  s.justifySelf = "start";
                  break;
              }
              break;
            case "right":
              switch (args.transAlign) {
                case "start":
                  s.justifySelf = "start";
                  break;
                case "middle":
                  s.justifySelf = "middle";
                  break;
                case "end":
                  s.justifySelf = "end";
                  break;
              }
              break;
          }
        },
      });

  presentation.leafViewImage = /** @type { Presentation["leafViewImage"] } */ (
    args
  ) => {
    const out = (() => {
      if (args.link != null) {
        const img = e(
          "img",
          {},
          {
            styles_: [
              ss(uniq("leaf_view_image_url_img"), {
                "": (s) => {
                  s.objectFit = "contain";
                  s.aspectRatio = "auto";
                  s.borderRadius = "0.2cm";
                  s.width = "100%";
                  s.height = "100%";
                },
              }),
            ],
          }
        );
        if (args.src != null) {
          img.src = args.src;
        }
        if (args.text != null) {
          img.alt = args.text;
        }
        return e(
          "a",
          { href: args.link },
          {
            styles_: [
              ss(uniq("leaf_view_image_url"), {
                "": (s) => {
                  s.flexShrink = "0";
                },
              }),
            ],
            children_: [img],
          }
        );
      } else {
        const img = e(
          "img",
          {},
          {
            styles_: [
              ss(uniq("leaf_view_image_nourl"), {
                "": (s) => {
                  s.objectFit = "contain";
                  s.aspectRatio = "auto";
                  s.flexShrink = "0";
                  s.borderRadius = "0.2cm";
                },
              }),
            ],
          }
        );
        if (args.src != null) {
          img.src = args.src;
        }
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

  presentation.leafViewText = /** @type { Presentation["leafViewText"] } */ (
    args
  ) => {
    const baseStyle = ss(uniq("leaf_view_text"), {
      "": (s) => {
        s.pointerEvents = "initial";
        s.whiteSpace = "pre-wrap";
      },
    });
    const dirStyle = ss(uniq("leaf_view_text_dir", args.orientation), {
      "": /** @type { () => ((s: CSSStyleDeclaration) => void) } */ (
        () => {
          switch (args.orientation) {
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
            styles_: [baseStyle, dirStyle, alignStyle],
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
            styles_: [baseStyle, dirStyle, alignStyle],
          }
        );
      }
    })();
    if (args.fontSize != null) {
      out.style.fontSize = args.fontSize;
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

  presentation.leafViewPlayButton =
    /** @type { Presentation["leafViewPlayButton"] } */ (args) => {
      const transAlign = args.transAlign || "start";
      const orientation = args.orientation || "right_down";
      const conv1 = conv(orientation);
      const trans1 = trans(orientation);
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
      const lineShift = "-0.7em";
      return {
        root: e(
          "button",
          {},
          {
            styles_: [
              leafButtonStyle,
              ss(uniq("leaf_view_play"), {
                "": (s) => {
                  const size = "1cm";
                  s.width = size;
                  s.height = size;
                  s.position = "relative";
                },
                [`>*:nth-child(2)`]: (s) => {
                  s.display = "none";
                },
                [`.${classStatePlaying}>*:nth-child(1)`]: (s) => {
                  s.display = "none";
                },
                [`.${classStatePlaying}>*:nth-child(2)`]: (s) => {
                  s.display = "initial";
                },
              }),
              ss(
                uniq("leaf_view_play_inner_trans", trans1),
                /** @type { () => ({[suffix: string]: (s: CSSStyleDeclaration) => void}) } */ (
                  () => {
                    switch (trans1) {
                      case "up":
                        return {
                          "": (s) => {
                            s.bottom = lineShift;
                          },
                        };
                      case "down":
                        return {
                          "": (s) => {
                            s.top = lineShift;
                          },
                        };
                      case "left":
                        return {
                          "": (s) => {
                            s.right = lineShift;
                          },
                        };
                      case "right":
                        return {
                          "": (s) => {
                            s.left = lineShift;
                          },
                        };
                    }
                  }
                )()
              ),
              viewLeafTransStyle({
                orientation: args.orientation,
                transAlign: args.transAlign,
              }),
            ],
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
        ),
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, form

  const contBodyStyle = s(uniq("cont_body"), {
    "": (s) => {
      s.gridRow = "2";
      s.gridColumn = "1/4";
      s.marginBottom = "2.5cm";
    },
    [`.${classMenuStateOpen}`]: (s) => {
      s.display = "none";
    },
  });
  const contBodyNarrowStyle = s(uniq("cont_body_narrow"), {
    "": (s) => {
      s.width = `min(${varSNarrow}, 100% - ${varSCol1Width} * 2)`;
      s.marginLeft = "auto";
      s.marginRight = "auto";
    },
  });
  presentation.contPageForm = /** @type {Presentation["contPageForm"]} */ (
    args
  ) => ({
    root: presentation.contGroup({
      children: [
        presentation.contBarMainForm({
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
                        s.columnGap = "0.2cm";
                        s.rowGap = "0.2cm";
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
                s.marginTop = "0.3cm";
              },
            }),
          ],
        }
      ),
    });

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, view/edit node
  let varSNodeGap = v(uniq(), "0.5cm");
  const leafNodeHboxStyle = s(uniq("leaf_edit_hbox"), {
    "": (s) => {
      s.alignItems = "stretch";
      s.position = "relative";
    },
  });
  const leafNodeVboxStyle = s(uniq("leaf_edit_vbox"), {
    "": (s) => {
      s.flexGrow = "1";
      s.gap = "0.2cm";
      s.border = `0.08cm solid ${varCNodeCenter}`;
      s.borderRadius = "0.2cm";
      s.padding = "0.2cm";
    },
  });
  const leafNodeVBoxNewStyle = s(uniq("leaf_edit_vbox_new"), {
    // increase specifity...
    [`.${leafNodeVboxStyle}`]: (s) => {
      s.borderStyle = "dashed";
    },
  });
  const varSRelIconSize = v(uniq(), "32pt");
  const varSRelIconWeight = v(uniq(), "800");
  const leafNodeRelStyle = s(uniq("leaf_edit_rel"), {
    "": (s) => {
      s.color = varCNodeCenter;
      s.fontSize = varSRelIconSize;
      s.fontWeight = varSRelIconWeight;
    },
  });
  const relIconSize = v(uniq(), "1.5cm");
  const leafNodeRelIncomingStyle = s(uniq("leaf_edit_rel_incoming"), {
    "": (s) => {
      s.alignSelf = "end";
      s.width = relIconSize;
      s.minWidth = relIconSize;
      s.rotate = "90deg";
    },
  });
  const leafNodeRelOutgoingStyle = s(uniq("leaf_edit_rel_outgoing"), {
    "": (s) => {
      s.alignSelf = "start";
      s.width = relIconSize;
      s.minWidth = relIconSize;
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
              e(
                "div",
                {
                  textContent: textIconRelIn,
                },
                {
                  styles_: [
                    leafIconStyle,
                    leafNodeRelStyle,
                    leafNodeRelIncomingStyle,
                  ],
                }
              ),
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
              e(
                "div",
                {
                  textContent: textIconRelOut,
                },
                {
                  styles_: [
                    leafIconStyle,
                    leafNodeRelStyle,
                    leafNodeRelOutgoingStyle,
                  ],
                }
              ),
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
            s(uniq("cont_page_edit_center"), {
              "": (s) => {
                s.padding = "0.2cm";
                s.backgroundColor = varCNodeCenter;
                s.borderRadius = "0.2cm";
                s.margin = "0.4cm 0";
              },
            }),
          ],
          children_: args.children,
        }
      ),
    });

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, node view

  const pageButtonsStyle = s(uniq("page_buttons_style"), {
    "": (s) => {
      s.justifyContent = "end";
      s.paddingLeft = "0.2cm";
      s.paddingRight = "0.2cm";
      s.paddingBottom = "0.2cm";
    },
  });

  presentation.contPageNodeViewAndHistory =
    /** @type {Presentation["contPageNodeViewAndHistory"]} */ (args) => {
      const body = e(
        "div",
        {},
        {
          styles_: [
            classMenuWantStateOpen,
            contBodyNarrowStyle,
            contVboxStyle,
            ss(uniq("page_view"), {
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
            e(
              "div",
              {},
              {
                styles_: [contBodyStyle, contVboxStyle],
                children_: [
                  e(
                    "div",
                    {},
                    {
                      styles_: [contHboxStyle, pageButtonsStyle],
                      children_: args.pageButtonChildren,
                    }
                  ),
                  body,
                ],
              }
            ),
          ],
        }).root,
        body: body,
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
                s.opacity = "0.5";
                s.whiteSpace = "pre-wrap";
              },
            }),
          ],
        }
      ),
    });

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, node edit

  presentation.contPageNodeEdit =
    /** @type {Presentation["contPageNodeEdit"]} */ (args) => ({
      root: presentation.contGroup({
        children: [
          presentation.contBarMainForm({
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
              styles_: [contVboxStyle, contBodyStyle],
              children_: [
                e(
                  "div",
                  {},
                  {
                    styles_: [contHboxStyle, pageButtonsStyle],
                    children_: args.pageButtonChildren,
                  }
                ),
                e(
                  "div",
                  {},
                  {
                    styles_: [
                      classMenuWantStateOpen,
                      contBodyNarrowStyle,
                      contVboxStyle,
                      ss(uniq("page_edit"), {
                        "": (s) => {
                          s.gap = varSNodeGap;
                        },
                      }),
                    ],
                    children_: args.children,
                  }
                ),
              ],
            }
          ),
        ],
      }).root,
    });

  const leafButtonEditFree =
    /** @type { (args: { icon: string, hint: string }) => { root: HTMLElement } } */ (
      args
    ) =>
      leafButton({
        title: args.hint,
        icon: args.icon,
        extraStyles: [
          leafIconStyle,
          ss(uniq("leaf_button_free"), {
            "": (s) => {
              s.borderRadius = "0.2cm";
              s.color = `color-mix(in srgb, ${varCForeground}, transparent 30%)`;
            },
            ":hover": (s) => {
              s.color = varCForeground;
            },
            ":hover:active": (s) => {
              s.color = varCForeground;
            },
            ">div": (s) => {
              s.fontSize = "22pt";
              s.fontWeight = "300";
              const size = "1.2cm";
              s.width = size;
              s.height = size;
            },
          }),
        ],
      });

  presentation.leafButtonNodeEditAdd =
    /** @type {Presentation["leafButtonNodeEditAdd"]} */ (args) =>
      leafButtonEditFree({ icon: textIconAdd, hint: args.hint });

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

  const contEditNodeHboxStyle = s(uniq("cont_edit_node_hbox"), {
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
                  s.marginRight = "0.2cm";
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
                s.marginBottom = "0.5cm";
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
                      s.fontSize = varSFontPageTitle;
                    },
                  }),
                ],
              }
            ),
            e("p", { textContent: args.desc }, {}),
            e(
              "div",
              {},
              {
                styles_: [
                  contVboxStyle,
                  ss(uniq("cont_history_commit_children"), {
                    "": (s) => {
                      s.marginTop = "0.8cm";
                      s.gap = "0.5cm";
                    },
                  }),
                ],
                children_: args.children,
              }
            ),
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
          styles_: [contVboxStyle],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [
                  contVboxStyle,
                  s(uniq("leaf_history_subject"), {
                    "": (s) => {
                      s.padding = "0.2cm 0";
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
                      s.gap = "0.5cm";
                    },
                  }),
                ],
                children_: args.rows,
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
              s(uniq("cont_history_rel_center"), {
                "": (s) => {
                  s.flexGrow = "1";
                  s.gap = "0.2cm";
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
              s(uniq("cont_history_row_hbox"), {
                "": (s) => {
                  s.gap = "0.5cm";
                },
              }),
            ],
            children_: children,
          }
        ),
      };
    };
  const vHistPredObjSize = v(uniq(), "1cm");
  presentation.contHistoryPredicateObjectRemove =
    /** @type {Presentation["contHistoryPredicateObjectRemove"]} */ (args) => {
      const revertButton = e(
        "button",
        {},
        {
          styles_: [
            leafButtonStyle,
            s(uniq("leaf_history_revert"), {
              "": (s) => {
                s.alignSelf = "center";
                s.width = vHistPredObjSize;
                s.minWidth = vHistPredObjSize;
                s.height = vHistPredObjSize;
              },
              ">svg": (s) => {
                s.width = "100%";
                s.height = "100%";
              },
              [`.${classStateDisabled}`]: (s) => {
                s.opacity = "0.3";
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
      return {
        root: contHistoryPredicateObject({
          icon: leafIcon({
            text: textIconDelete,
            extraStyles: [
              s(uniq("cont_history_row_rel_icon"), {
                "": (s) => {
                  s.color = varCRemove;
                  s.opacity = "0.3";
                  s.fontSize = varSRelIconSize;
                  s.fontWeight = varSRelIconWeight;
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
      return {
        root: contHistoryPredicateObject({
          icon: leafIcon({
            text: textIconAdd,
            extraStyles: [
              s(uniq("cont_history_row_rel_icon"), {
                "": (s) => {
                  s.opacity = "0.3";
                  s.fontSize = varSRelIconSize;
                  s.fontWeight = varSRelIconWeight;
                },
              }),
              leafNodeRelOutgoingStyle,
            ],
          }),
          button: e(
            "div",
            {},
            {
              styles_: [
                s(uniq("leaf_history_revert"), {
                  "": (s) => {
                    s.display = "block";
                    s.alignSelf = "center";
                    s.width = vHistPredObjSize;
                    s.minWidth = vHistPredObjSize;
                    s.height = vHistPredObjSize;
                  },
                }),
              ],
              children_: [],
            }
          ),
          children: args.children,
        }).root,
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: menu

  const contMenuGroupVBoxStyle = s(uniq("cont_menu_group0"), {
    "": (s) => {
      s.marginLeft = "0.6cm";
      s.gap = "0.3cm";
    },
  });

  presentation.contBodyMenu =
    /** @type {Presentation["contMenuBody"]} */ () => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            contMenuGroupVBoxStyle,
            ss(uniq("cont_body_menu"), {
              "": (s) => {
                s.gridColumn = "2";
                s.gridRow = "2";
                s.columns = "min(100dvw, 12cm)";
                s.columnGap = "0.5cm";
                s.justifyContent = "start";
              },
            }),
          ],
        }
      ),
    });

  presentation.contBarMenu = /** @type {Presentation["contBarMenu"]} */ (
    args
  ) =>
    presentation.contBar({
      extraStyles: [
        ss(uniq("cont_bar_menu"), {
          "": (s) => {
            s.gridColumn = "1/3";

            s.backgroundColor = varCBackgroundMenuButtons;
            s.margin = "0.5cm 0";
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
      });
  presentation.leafMenuBarButtonLogout =
    /** @type {Presentation["leafMenuBarButtonLogout"]} */ (args) =>
      presentation.leafButtonBig({
        title: "Login",
        icon: textIconLogout,
        text: "Login",
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
              s.padding = "0.5cm 0";
            },
            ">summary": (s) => {
              s.listStyle = "none";
              s.position = "relative";
              s.display = "flex";
              s.flexDirection = "row";
              s.alignContent = "center";
              s.justifyContent = "flex-start";
              s.fontSize = varSFontMenu;
            },
            ">summary>div": (s) => {
              s.marginLeft = "-0.6cm";
              s.fontSize = "14pt";
              s.width = "0.6cm";
              s.opacity = "0.5";
            },
            ">summary:hover>div": (s) => {
              s.opacity = "1";
            },
            ">summary>div.open": (s) => {
              s.display = "none";
            },
            "[open]>summary>div.closed": (s) => {
              s.display = "none";
            },
            "[open]>summary>div.open": (s) => {
              s.display = "grid";
            },
          }),
        ],
        children_: [
          e(
            "summary",
            {},
            {
              children_: [
                e(
                  "div",
                  {
                    textContent: textIconFoldClosed,
                  },
                  {
                    styles_: ["closed", leafIconStyle],
                  }
                ),
                e(
                  "div",
                  {
                    textContent: textIconFoldOpened,
                  },
                  {
                    styles_: ["open", leafIconStyle],
                  }
                ),
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
                  ss(uniq("leaf_menu_link"), {
                    "": (s) => {
                      s.fontSize = varSFontMenu;
                      s.display = "flex";
                      s.flexDirection = "row";
                      s.alignItems = "center";
                      s.justifyContent = "flex-start";
                    },
                    ">div": (s) => {
                      s.opacity = "0";
                      s.paddingLeft = "0.5cm";
                      s.fontSize = "14pt";
                    },
                    ":hover>div": (s) => {
                      s.opacity = "1";
                    },
                    ":hover:active>div": (s) => {
                      s.opacity = "1";
                    },
                  }),
                ],
                children_: [
                  e("span", { textContent: args.title }, {}),
                  e(
                    "div",
                    {
                      textContent: textIconGo,
                    },
                    {
                      styles_: [leafIconStyle],
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
              s.gridColumn = "1/3";
              s.display = "grid";
              s.gridTemplateColumns = "subgrid";
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
                    s.columns = "min(100%, 12cm)";
                    s.columnGap = "0.5cm";
                    s.justifyContent = "start";
                    s.minHeight = `calc(100dvh - 5cm)`;
                  },
                  ">*": (s) => {
                    s.maxWidth = varSMenuColWidth;
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
                        s.opacity = "0.5";
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
          ">div": (s) => {
            s.fontSize = varSFontAdmenu;
            s.width = varSCol3Width;
            s.height = varSCol3Width;
          },
        }),
      ],
    }).root;
    return {
      root: e(
        "div",
        {},
        {
          styles_: [
            ss(uniq("body"), {
              "": (s) => {
                s.display = "grid";
                s.overflowX = "hidden";
                s.maxWidth = "100dvw";
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
            e(
              "div",
              {
                id: "menu",
              },
              {
                styles_: [
                  classMenuWantStateOpen,
                  s(uniq("cont_menu"), {
                    "": (s) => {
                      s.zIndex = "3";
                      s.gridRow = "1/4";
                      s.gridColumn = "1/3";
                      s.backgroundColor = varCBackgroundMenu;
                      s.filter =
                        "drop-shadow(0.05cm 0px 0.05cm rgba(0, 0, 0, 0.06))";
                      s.overflow = "hidden";
                      s.display = "grid";
                      s.gridTemplateColumns = "subgrid";
                      s.gridTemplateRows = "subgrid";
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
                              s.gridColumn = "2";
                              s.gridRow = "1";
                              s.alignItems = "center";
                              s.gap = "0.3cm";
                            },
                            ">svg": (s) => {
                              s.height = "0.9cm";
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
                                    s.fontSize = varSFontTitle;
                                    s.marginTop = "-0.15cm";
                                    s.color = "#5d7186";
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
              s.alignItems = "center";
              s.margin = "0.5cm";
              s.borderRadius = "0.5cm";
              s.border = `0.06cm solid ${varCForegroundFade}`;
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
                  s.fill = "#fefefe";
                  s.fontWeight = "200";
                },
              }),
            ],
          }),
          leafIcon({
            text: textIconClick,
            extraStyles: [
              ss(uniq("app_link_perms_play"), {
                "": (s) => {
                  s.width = "3cm";
                  s.height = "3cm";
                },
                " text": (s) => {
                  s.fill = varCForegroundDark;
                  s.fontWeight = "100";
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
                s.backgroundColor = varCBackgroundDark;
              },
            }),
          ],
          children_: [button],
        }
      ),
      button: button,
    };
  };

  const varCLinkDisplayBg = v(uniq(), "rgb(35, 35, 36)");
  presentation.appLink = /** @type {Presentation["appLink"]} */ (args) => {
    const display = e(
      "div",
      {},
      {
        styles_: [
          ss(uniq("cont_app_link_display"), {
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
    const displayUnder2 = e(
      "div",
      {},
      {
        styles_: [
          ss(uniq("leaf_app_display_under2"), {
            "": (s) => {
              s.aspectRatio = "1/1";
              s.objectFit = "contain";
              s.width = "100%";
              s.height = "100%";
            },
          }),
        ],
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
              s.padding = "0.2cm";
              s.color = varCBackgroundDark;
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
              s.paddingTop = "0.2cm";
              s.paddingBottom = "0.2cm";
            },
          }),
        ],
        children_: [displayUnder2, displayUnder, display, displayOver],
      }
    );
    const textStyle = ss(uniq("leaf_app_link_text"), {
      "": (s) => {
        s.color = varCForegroundDark;
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
                s.fontSize = "14pt";
              },
            }),
          ],
        }
      );
    /** @type { (children:Element[])=>HTMLElement} */
    const vert = (children) =>
      e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            ss(uniq("cont_app_link_vert"), {
              "": (s) => {
                s.paddingLeft = "0.4cm";
                s.paddingRight = "0.4cm";
                s.gap = "0.2cm";
              },
            }),
          ],
          children_: children,
        }
      );
    const title = bigText("waiting...");
    const albumArtist = e(
      "span",
      { textContent: "Waiting..." },
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
                s.backgroundColor = varCBackgroundDark;
                s.gap = "0.3cm";
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
            vert([title, albumArtist]),
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
