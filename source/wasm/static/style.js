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
    let uniq = [""];
    for (const e of notnull(new Error().stack).matchAll(/(\d+):\d+/g)) {
      uniq[0] = `${e[1]}`;
    }
    uniq.push(...args);
    return `r${uniq.join("_")}`;
  };

  const uniqn = /** @type { (...args: string[]) => string} */ (...args) => {
    const uniq = [];
    for (const e of notnull(new Error().stack).matchAll(/(\d+):\d+/g)) {
      uniq.push(e[1]);
    }
    uniq.push(...args);
    return `r${uniq.join("_")}`;
  };

  const e = /** @type {
    <N extends keyof HTMLElementTagNameMap>(
      name: N,
      args: Partial<HTMLElementTagNameMap[N]>,
      args2: {
        styles_?: string[];
        children_?: HTMLElement[];
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
          children_?: HTMLElement[];
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

  const v = /** @type {(v: string) => string} */ (val) => {
    const name = `--${uniq()}`;
    globalStyleRoot.setProperty(name, val);
    return `var(${name})`;
  };

  const vs = /** @type {(light: string, dark: string) => string} */ (
    light,
    dark
  ) => {
    const name = `--${uniq()}`;
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

  const textIconPlay = "\ue037";
  const textIconPause = "\ue034";
  const textIconDelete = "\ue15b";
  const textIconRevert = "\ue166";
  const textIconAdd = "\ue145";
  const textIconNext = "\ue5cc";
  const textIconPrev = "\ue5cb";
  const textIconLink = "\ue157";
  const textIconSave = "\ue161";
  const textIconUnlink = "\ue16f";
  const textIconLogin = "\uea77";
  const textIconMenu = "\ue5d2";
  const textIconMenuLink = "\ue5c8";
  const textIconFoldClosed = "\ue316";
  const textIconFoldOpened = "\ue313";
  const textIconClose = "\ue5cd";
  const textIconRelIn = "\uf72d";
  const textIconRelOut = "\uf72e";

  // xx Variables

  const varSPadTitle = v("0.4cm");
  const varSPadViewOuter = varSPadTitle;
  const varSFontTitle = v("24pt");
  const varSFontAdmenu = v("20pt");
  const varSFontMenu = v("20pt");
  const varSNarrow = v("20cm");
  const varSModalHeight = v("20cm");
  const varSCol1Width = v("min(0.8cm, 5dvw)");
  const varSCol3Width = v("1.4cm");
  const varSMenuColWidth = v("min(100%, 12cm)");
  const varSEditRelWidth = v("1.5cm");
  //const varCBackground = vs("rgb(205, 207, 212)", "rgb(0,0,0)");
  const varCBackground = vs("rgb(230, 232, 238)", "rgb(0,0,0)");
  const varCBg2 = vs("rgb(218, 220, 226)", "rgb(0,0,0)");
  const varCBackgroundDark = v("rgb(45, 45, 46)");
  //const varCBackgroundMenu = vs("rgb(173, 177, 188)", "rgb(0,0,0)");
  const varCBackgroundMenu = vs("rgb(205, 208, 217)", "rgb(0,0,0)");
  const varCBackgroundMenuButtons = vs("rgb(219, 223, 232)", "rgb(0,0,0)");

  const varCButtonHover = vs("rgba(255, 255, 255, 0.7)", "rgb(0,0,0)");
  const varCButtonClick = vs("rgba(255, 255, 255, 1)", "rgb(0,0,0)");

  const varCSeekbarEmpty = vs("rgb(212, 216, 223)", "rgb(0,0,0)");
  const varCSeekbarFill = vs("rgb(197, 196, 209)", "rgb(0,0,0)");

  const varSButtonPad = v("0.3cm");

  const varCForeground = vs("rgb(0, 0, 0)", "rgb(0,0,0)");
  const varCForegroundDark = v("rgb(249, 248, 240)");
  const varCForegroundError = vs("rgb(154, 60, 74)", "rgb(0,0,0)");
  const varCBorderError = vs("rgb(192, 61, 80)", "rgb(0,0,0)");
  const varCInputBorder = vs("rgb(154, 157, 168)", "rgb(0,0,0)");
  const varCInputBackground = vs(varCBg2, "rgb(0,0,0)");
  const varCHighlightNode = varCBg2;
  const varCSpinner = "rgb(155, 178, 229)";
  const varCEditCenter = varCBg2;
  const varCEditButtonFreeHover = vs(
    `color-mix(in srgb, ${varCBg2}, transparent 30%)`,
    "rgb(0,0,0)"
  );
  const varCEditButtonFreeClick = varCBg2;

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
  const classInputStateInvalid = "invalid";
  presentation.classStateInvalid =
    /** @type { Presentation["classStateInvalid"]} */ () => ({
      value: classInputStateInvalid,
    });
  const classStateThinking = "thinking";
  presentation.classStateThinking =
    /** @type { Presentation["classStateThinking"]} */ () => ({
      value: classStateThinking,
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

  presentation.contBar = /** @type {Presentation["contBar"]} */ (args) => {
    /** @type { (children: HTMLElement[]) => HTMLElement} */
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

  presentation.contSpinner = /** @type {Presentation["contSpinner"]} */ () => ({
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
        ],
      }
    ),
  });

  presentation.leafAsyncBlock = /** @type {Presentation["leafAsyncBlock"]} */ (
    args
  ) => ({
    root: e(
      "div",
      {},
      {
        styles_: [contGroupStyle],
        children_: [
          e(
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
              children_: [presentation.contSpinner({}).root],
            }
          ),
        ],
      }
    ),
  });

  presentation.leafErrBlock = /** @type {Presentation["leafErrBlock"]} */ (
    args
  ) => ({
    root: e(
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
            },
          }),
        ],
        children_: [e("span", { textContent: args.data }, {})],
      }
    ),
  });

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
      s.gap = "0.2cm";
    },
    ":hover": (s) => {
      s.backgroundColor = varCButtonHover;
    },
    ":hover:active": (s) => {
      s.backgroundColor = varCButtonClick;
    },
    ".thinking": (s) => {
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

  presentation.leafButtonBig =
    /** @type { Presentation["leafButtonBig"] } */
    (args) =>
      leafButton({
        title: args.title,
        icon: args.icon,
        text: args.text,
        extraStyles: [
          ss(uniq("leaf_text_button"), {
            "": (s) => {
              s.padding = varSButtonPad;
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

  // /////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: home
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
              s.gridRow = "2";
              s.justifyContent = "center";
              s.alignItems = "center";
            },
            [`.${classMenuStateOpen}`]: (s) => {
              s.display = "none";
            },
            ">svg": (s) => {
              s.width = "min(100%, 12cm)";
              s.padding = "0.5cm";
            },
            ">svg>text": (s) => {
              s.fontSize = "140pt";
              s.fill = "#fefefe";
            },
          }),
        ],
        children_: [
          et(`
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 210.77">
              <path fill="#fefefe" d="M187.6 100.09a69.05 69.05 0 0 0-68.8-63.7A69.05 69.05 0 0 0 57 74.7c4.35-.17 8.86-.06 13.54.37a56.99 56.99 0 0 1 105.12 26.33 72.7 72.7 0 0 0 11.94-1.31zm-9.93 41.27c-4.6.16-9.37.03-14.31-.45a56.91 56.91 0 0 1-44.56 21.47 57.06 57.06 0 0 1-56.25-47.73c-4.14-.1-8.12.2-12.01.83a69 69 0 0 0 127.14 25.88Z"/>
              <path fill="none" stroke="#7ca7db" stroke-linecap="round" stroke-width="10" d="M5 110.87c20.49-9.6 40.98-19.2 68-15.39 27.02 3.81 60.58 21.04 88 25 27.42 3.97 48.71-5.32 70-14.6"/>
              <path fill="none" stroke="#fefefe" stroke-linecap="square" stroke-width="10" d="m34.52 44.15 12.13 8.81M86.6 6.3l4.64 14.27M151 6.3l-4.64 14.27m56.72 23.58-12.13 8.81m12.13 113.66-12.13-8.82M151 204.46l-4.64-14.26M86.6 204.46l4.64-14.26m-56.72-23.58 12.13-8.82"/>
              <text x="286" y="50%" dominant-baseline="middle">sunwet</text>
            </svg>
          `),
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

  const leafInputStyle = s(uniq("leaf_form_input_text"), {
    "": (s) => {
      s.borderBottom = `0.04cm solid ${varCInputBorder}`;
      s.padding = "0.1cm";
      s.maxWidth = "9999cm";
    },
    [`.${classInputStateInvalid}`]: (s) => {
      s.borderColor = varCBorderError;
      s.borderBottomStyle = "dashed";
      s.borderBottomWidth = "0.06cm";
    },
  });

  presentation.leafInputText = /** @type {Presentation["leafInputText"]} */ (
    args
  ) => ({
    root: e(
      "input",
      {
        type: "text",
        placeholder: args.title,
        title: args.title,
        name: args.id,
        value: args.value,
      },
      {
        styles_: [leafInputStyle],
      }
    ),
  });

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
              styles_: [leafInputStyle],
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
            styles_: [leafInputStyle],
          }
        )
      );
    out.checked = args.value;
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
            styles_: [leafInputStyle],
          }
        )
      );
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
            styles_: [leafInputStyle],
          }
        )
      );
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
              styles_: [leafInputStyle],
            }
          )
        );
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
            styles_: [leafInputStyle],
          }
        )
      );
    return { root: out };
  };
  presentation.leafInputEnum = /** @type {Presentation["leafInputEnum"]} */ (
    args
  ) => {
    const children = [];
    for (const [k, v] of Object.entries(args.options)) {
      children.push(e("option", { textContent: v, value: v }, {}));
    }
    const out = e(
      "select",
      {
        title: args.title,
        name: args.id,
        value: args.value,
      },
      {
        styles_: [leafInputStyle],
        children_: children,
      }
    );
    return { root: out };
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

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, view

  presentation.contPageView = /** @type {Presentation["contPageView"]} */ (
    args
  ) => ({
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
        children_: args.entries,
      }
    ),
  });

  presentation.contBarViewTransport =
    /** @type {Presentation["contBarViewTransport"]} */ () => {
      const buttonShare = leafTransportButton({
        title: "Share",
        icons: { "": textIconLink },
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
                s.backgroundColor = varCSeekbarFill;
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
                s.backdropFilter = "blur(0.2cm)";
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

  presentation.contPageViewList =
    /** @type {Presentation["contPageViewList"]} */ (args) => {
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
              contStackStyle,
              ss(uniq("view_list_body"), {
                "": (s) => {
                  s.padding = `0 max(0.3cm, min(${varSCol1Width}, 100dvw / 20))`;
                  s.paddingBottom = "2cm";
                  s.flexGrow = "1";
                },
              }),
            ],
            children_: [args.rows],
          }
        )
      );
      return presentation.contPageView({
        entries: children,
      });
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

  const transportButtonClipPaths = new Map();
  const leafTransportButton =
    /** @type {(args: { title: string, icons: {[s: string]: string} }) => { root: HTMLElement }} */
    (args) => {
      const size = "1cm";
      const statePairs = Object.entries(args.icons);
      statePairs.sort();
      const buildStyleId = [];
      /** @type {{[s:string]: (s: CSSStyleDeclaration) => void}} */
      const buildStyle = {};
      for (const [state, icon] of Object.entries(args.icons)) {
        if (!transportButtonClipPaths.has(icon)) {
          const clipPathId = window.crypto.randomUUID();
          // Debug by adding 0-1 viewbox and moving text outside of defs/clipPath; y is scaled by scale so 100x it
          document.body.appendChild(
            et(`
              <svg width="0" height="0">
                <defs>
                  <clipPath id="${clipPathId}" clipPathUnits="objectBoundingBox">
                    <text x="50" y="96" style="
                      text-anchor: middle;
                      font-family: I;
                      font-weight: 100;
                      font-size: 90px;
                      scale: 1%;
                    ">${icon}</text>
                  </clipPath>
                </defs>
              </svg>
            `)
          );
          transportButtonClipPaths.set(icon, clipPathId);
        }
        const clipPathId = transportButtonClipPaths.get(icon);
        buildStyleId.push(state);
        buildStyleId.push(clipPathId);
        buildStyle[
          (() => {
            if (state == "") {
              return "";
            } else {
              return `.${state}`;
            }
          })() + ">div"
        ] = (s) => {
          s.clipPath = `url(#${clipPathId})`;
        };
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
              ">div": (s) => {
                s.display = "inline-block";
                s.width = "100%";
                s.height = "100%";
                s.backdropFilter = "grayscale() brightness(0.8) invert(1)";
              },
            }),
            ss(uniq(...buildStyleId), buildStyle),
          ],
          children_: [e("div", {}, {})],
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
                      [`>.${classViewTransverseStart}`]: (s) => {
                        s.alignSelf = "first baseline";
                      },
                      [`>.${classViewTransverseEnd}`]: (s) => {
                        s.alignSelf = "last baseline";
                      },
                    };
                  case "right":
                    return {
                      "": (s) => {
                        s.flexDirection = "row";
                      },
                      [`>.${classViewTransverseStart}`]: (s) => {
                        s.alignSelf = "first baseline";
                      },
                      [`>.${classViewTransverseEnd}`]: (s) => {
                        s.alignSelf = "last baseline";
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
          const child = row[i0];
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
            ss(
              uniq("cont_view_table_conv", conv(args.orientation)),
              /** @type { () => ({[prefix: string]: (s: CSSStyleDeclaration) => void}) } */ (
                () => {
                  switch (conv(args.orientation)) {
                    case "up":
                    case "down":
                      return {
                        "": (s) => {},
                        [`>.${classViewTransverseStart}`]: (s) => {
                          s.alignSelf = "first baseline";
                        },
                        [`>.${classViewTransverseEnd}`]: (s) => {
                          s.alignSelf = "last baseline";
                        },
                      };
                    case "left":
                    case "right":
                      return {
                        "": (s) => {},
                        [`>.${classViewTransverseStart}`]: (s) => {
                          s.justifySelf = "first baseline";
                        },
                        [`>.${classViewTransverseEnd}`]: (s) => {
                          s.justifySelf = "last baseline";
                        },
                      };
                  }
                }
              )()
            ),
            ss(
              uniq("cont_view_table_trans", trans(args.orientation)),
              /** @type { () => ({[prefix: string]: (s: CSSStyleDeclaration) => void}) } */ (
                () => {
                  switch (trans(args.orientation)) {
                    case "up":
                    case "left":
                      return {
                        "": (s) => {
                          s.justifyItems = "end";
                        },
                      };
                    case "down":
                    case "right":
                      return {
                        "": (s) => {
                          s.justifyItems = "start";
                        },
                      };
                  }
                }
              )()
            ),
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
      if (args.xScroll) {
        out.style.overflowX = "scroll";
      }
      return { root: out };
    };

  presentation.leafViewImage = /** @type { Presentation["leafViewImage"] } */ (
    args
  ) => {
    const out = (() => {
      if (args.link != null) {
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
              (() => {
                switch (args.transAlign) {
                  case "start":
                    return classViewTransverseStart;
                  case "middle":
                    return classViewTransverseMiddle;
                  case "end":
                    return classViewTransverseEnd;
                }
              })(),
            ],
            children_: [
              e(
                "img",
                {
                  src: args.src,
                  alt: args.text,
                },
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
              ),
            ],
          }
        );
      } else {
        return e(
          "img",
          {
            src: args.src,
            alt: args.text,
          },
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
              (() => {
                switch (args.transAlign) {
                  case "start":
                    return classViewTransverseStart;
                  case "middle":
                    return classViewTransverseMiddle;
                  case "end":
                    return classViewTransverseEnd;
                }
              })(),
            ],
          }
        );
      }
    })();
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
    /** @type {string} */
    const alignStyle = (() => {
      switch (args.transAlign) {
        case "start":
          return classViewTransverseStart;
        case "middle":
          return classViewTransverseMiddle;
        case "end":
          return classViewTransverseEnd;
      }
    })();
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
      const direction = args.direction || "right";
      const playIconStyle = ss(uniq("leaf_view_play_inner"), {
        "": (s) => {
          s.width = "100%";
          s.height = "100%";
          s.writingMode = "horizontal-tb";
          s.fontSize = "24pt";
          s.fontWeight = "100";
        },
      });
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
                  // Hack to override baseline using inline-block weirdness...
                  // https://stackoverflow.com/questions/39373787/css-set-baseline-of-inline-block-element-manually-and-have-it-take-the-expected
                  s.display = "inline-block";
                  s.textWrapMode = "nowrap";
                },
                [`div:nth-child(2)`]: (s) => {
                  s.display = "none";
                },
                [`.${classStatePlaying}>div:nth-child(1)`]: (s) => {
                  s.display = "none";
                },
                [`.${classStatePlaying}>div:nth-child(2)`]: (s) => {
                  s.display = "initial";
                },
              }),
              ss(
                uniq("leaf_view_play", direction),
                /** @type { () => ({[suffix: string]: (s: CSSStyleDeclaration) => void}) } */ (
                  () => {
                    switch (direction) {
                      case "up":
                        return {
                          "": (s) => {
                            s.writingMode = "vertical-rl";
                          },
                          ">*:nth-child(1)": (s) => {
                            s.rotate = "270deg";
                          },
                        };
                      case "down":
                        return {
                          "": (s) => {
                            s.writingMode = "vertical-rl";
                          },
                          ">*:nth-child(1)": (s) => {
                            s.rotate = "90deg";
                          },
                        };
                      case "left":
                        return {
                          ">*:nth-child(1)": (s) => {
                            s.rotate = "180deg";
                          },
                        };
                      case "right":
                        return {
                          ">*:nth-child(1)": (s) => {},
                        };
                    }
                  }
                )()
              ),
              (() => {
                switch (transAlign) {
                  case "start":
                    return classViewTransverseStart;
                  case "middle":
                    return classViewTransverseMiddle;
                  case "end":
                    return classViewTransverseEnd;
                }
              })(),
            ],
            children_: [
              e(
                "div",
                { textContent: textIconPlay },
                { styles_: [leafIconStyle, playIconStyle] }
              ),
              e(
                "div",
                { textContent: textIconPause },
                { styles_: [leafIconStyle, playIconStyle] }
              ),
            ],
          }
        ),
      };
    };

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, form

  const contBodyNarrowStyle = s(uniq("cont_body_narrow"), {
    "": (s) => {
      s.gridRow = "2";
      s.gridColumn = "1/4";
      s.width = `min(${varSNarrow}, 100% - ${varSCol1Width} * 2)`;
      s.justifySelf = "center";
      s.marginBottom = "2.5cm";
    },
    [`.${classMenuStateOpen}`]: (s) => {
      s.display = "none";
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

  ///////////////////////////////////////////////////////////////////////////////
  // xx Components, styles: page, edit

  let varSEditGap = v("0.5cm");

  presentation.contPageEdit = /** @type {Presentation["contPageEdit"]} */ (
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
              contBodyNarrowStyle,
              contVboxStyle,
              ss(uniq("page_edit"), {
                "": (s) => {
                  s.gap = varSEditGap;
                },
              }),
            ],
            children_: args.children,
          }
        ),
      ],
    }).root,
  });

  presentation.contPageEditSectionRel =
    /** @type {Presentation["contPageEditSectionRel"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            contVboxStyle,
            ss(uniq("cont_page_edit_section_rel"), {
              "": (s) => {
                s.gap = varSEditGap;
              },
            }),
          ],
          children_: args.children,
        }
      ),
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

  presentation.leafButtonEditAdd =
    /** @type {Presentation["leafButtonEditAdd"]} */ (args) =>
      leafButtonEditFree({ icon: textIconAdd, hint: args.hint });

  const contEditNodeVboxStyle = s(uniq("cont_edit_node_vbox"), {
    "": (s) => {
      s.gap = "0.2cm";
    },
  });
  const contEditNodeHboxStyle = s(uniq("cont_edit_node_hbox"), {
    "": (s) => {
      s.justifyContent = "stretch";
      s.alignItems = "end";
      s.gap = "0.2cm";
    },
  });
  presentation.leafEditNode = /** @type {Presentation["leafEditNode"]} */ (
    args
  ) => {
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
          styles_: [contVboxStyle, contEditNodeVboxStyle],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [contHboxStyle, contEditNodeHboxStyle],
                children_: [
                  args.inputType,
                  presentation.leafSpace({}).root,
                  buttonDelete.root,
                  buttonRevert.root,
                ],
              }
            ),
            args.inputValue,
          ],
        }
      ),
      buttonDelete: buttonDelete.root,
      buttonRevert: buttonRevert.root,
    };
  };

  presentation.leafEditPredicate =
    /** @type {Presentation["leafEditPredicate"]} */ (args) =>
      presentation.leafInputText({
        id: undefined,
        title: "Predicate",
        value: args.value,
      });

  const leafEditHBoxStyle = s(uniq("leaf_edit_incoming_hbox"), {
    "": (s) => {
      s.alignItems = "stretch";
      s.position = "relative";
    },
  });
  const leafEditVBoxStyle = s(uniq("leaf_edit_incoming_vbox"), {
    "": (s) => {
      s.flexGrow = "1";
      s.gap = "0.2cm";
      s.border = `0.08cm solid ${varCEditCenter}`;
      s.borderRadius = "0.2cm";
      s.padding = "0.2cm";
    },
  });
  const leafEditRelStyle = s(uniq("leaf_edit_rel"), {
    "": (s) => {
      s.color = varCEditCenter;
      s.fontSize = "32pt";
      s.fontWeight = "800";
    },
  });
  const varSEditRelOverlap = v("1.2cm");
  const leafEditRelIncomingStyle = s(uniq("leaf_edit_rel_incoming"), {
    "": (s) => {
      s.gridColumn = "2";
      s.alignSelf = "end";
      s.width = "1.5cm";
      s.rotate = "90deg";
    },
  });
  const leafEditRelOutgoingStyle = s(uniq("leaf_edit_rel_incoming"), {
    "": (s) => {
      s.gridColumn = "1";
      s.alignSelf = "start";
      s.width = "1.5cm";
      s.rotate = "180deg";
    },
  });

  presentation.contEditRowIncoming =
    /** @type {Presentation["contEditRowIncoming"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [contHboxStyle, leafEditHBoxStyle],
          children_: [
            e(
              "div",
              {},
              {
                styles_: [contVboxStyle, leafEditVBoxStyle],
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
                  leafEditRelStyle,
                  leafEditRelIncomingStyle,
                ],
              }
            ),
          ],
        }
      ),
    });

  presentation.contEditRowOutgoing =
    /** @type {Presentation["contEditRowOutgoing"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [contHboxStyle, leafEditHBoxStyle],
          children_: [
            e(
              "div",
              {
                textContent: textIconRelOut,
              },
              {
                styles_: [
                  leafIconStyle,
                  leafEditRelStyle,
                  leafEditRelOutgoingStyle,
                ],
              }
            ),
            e(
              "div",
              {},
              {
                styles_: [contVboxStyle, leafEditVBoxStyle],
                children_: args.children,
              }
            ),
          ],
        }
      ),
    });

  presentation.contEditSectionCenter =
    /** @type {Presentation["contEditSectionCenter"]} */ (args) => ({
      root: e(
        "div",
        {},
        {
          styles_: [
            s(uniq("cont_page_edit_center"), {
              "": (s) => {
                s.padding = "0.2cm";
                s.backgroundColor = varCEditCenter;
                s.borderRadius = "0.2cm";
                s.margin = "0.4cm 0";
              },
            }),
          ],
          children_: [args.child],
        }
      ),
    });

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
                      textContent: textIconMenuLink,
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
                  textContent: "Guest",
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
              presentation.leafButtonBig({
                title: "Login",
                icon: textIconLogin,
                text: "Login",
              }).root,
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

  presentation.appLink = /** @type {Presentation["appLink"]} */ (args) => {
    const display = e(
      "div",
      {},
      {
        styles_: [
          ss(uniq("cont_app_link_display"), {
            "": (s) => {
              s.backgroundColor = "rgb(35, 35, 36)";
              s.paddingTop = "0.2cm";
              s.paddingBottom = "0.2cm";
            },
          }),
        ],
        children_: [presentation.leafAsyncBlock({}).root],
      }
    );
    const displayOver = e("div", {}, { styles_: [contGroupStyle] });
    const displayStack = e(
      "div",
      {},
      { styles_: [contStackStyle], children_: [display, displayOver] }
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
    /** @type { (children:HTMLElement[])=>HTMLElement} */
    const horiz = (children) =>
      e(
        "div",
        {},
        {
          styles_: [
            contHboxStyle,
            ss(uniq("cont_app_link_horiz"), {
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
    const artist = bigText("Linking...");
    const title = bigText("linking...");
    const album = e(
      "span",
      { textContent: "Linking..." },
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
              },
            }),
          ],
          children_: [
            leafSpace({}).root,
            displayStack,
            horiz([
              artist,
              e("span", { textContent: " - " }, { styles_: [textStyle] }),
              title,
            ]),
            horiz([album]),
            leafSpace({}).root,
          ],
        }
      ),
      display: display,
      display_over: displayOver,
      album: album,
      artist: artist,
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
