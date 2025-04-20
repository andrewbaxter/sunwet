/// <reference path="style_export.d.ts" />

const presentation = {};

///////////////////////////////////////////////////////////////////////////////
// xx Utility, globals

/** @type { <T>(x: T|null|undefined) => T } */
presentation.notnull = /** @type {Presentation["notnull"]} */ (x) => {
  if (x == null) {
    throw Error();
  }
  return x;
};
const notnull = presentation.notnull;

presentation.uniq = /** @type {Presentation["uniq"]} */ (...args) => {
  var uniq = [""];
  for (const e of notnull(new Error().stack).matchAll(/(\d+):\d+/g)) {
    uniq[0] = `${e[1]}`;
  }
  uniq.push(...args);
  return `r${uniq.join("_")}`;
};
const uniq = presentation.uniq;

/** @type { (...args: string[]) => string} */
presentation.uniqn = /** @type {Presentation["uniqn"] } */ (...args) => {
  const uniq = [];
  for (const e of notnull(new Error().stack).matchAll(/(\d+):\d+/g)) {
    uniq.push(e[1]);
  }
  uniq.push(...args);
  return `r${uniq.join("_")}`;
};

presentation.e = /** @type {Presentation["e"]} */ (name, args1, args2) => {
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
const e = presentation.e;

presentation.et = /** @type { Presentation["et"]} */ (t, args) => {
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
const et = presentation.et;

const resetStyle = e(
  "link",
  { rel: "stylesheet", href: "style_reset.css" },
  {}
);
document.head.appendChild(resetStyle);

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
  (globalStyleMediaLight.cssRules[globalStyleMediaLight.insertRule(":root {}")])
    .style;
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

presentation.v = /** @type {Presentation["v"]} */ (val) => {
  const name = `--${uniq()}`;
  globalStyleRoot.setProperty(name, val);
  return `var(${name})`;
};
const v = presentation.v;

presentation.vs = /** @type {Presentation["vs"]} */ (light, dark) => {
  const name = `--${uniq()}`;
  globalStyleLight.setProperty(name, light);
  globalStyleDark.setProperty(name, dark);
  return `var(${name})`;
};
const vs = presentation.vs;

presentation.s = /** @type {Presentation["s"]} */ (id, f) => {
  for (const [suffix, f1] of Object.entries(f)) {
    globalStyle.insertRule(`.${id}${suffix} {}`, 0);
    f1(/** @type { CSSStyleRule } */ (globalStyle.cssRules[0]).style);
  }
  return id;
};
const s = presentation.s;

const staticStyles = new Map();
// Static style - the id must be unique for every value closed on (i.e. put all the arguments in the id).
presentation.ss = /** @type {Presentation["ss"]} */ (id, f) => {
  if (staticStyles.has(id)) {
    return id;
  }
  for (const [suffix, f1] of Object.entries(f)) {
    globalStyle.insertRule(`.${id}${suffix} {}`, 0);
    f1(/** @type { CSSStyleRule } */ (globalStyle.cssRules[0]).style);
  }
  return id;
};
const ss = presentation.ss;

///////////////////////////////////////////////////////////////////////////////
// xx Constants

const textIconPlay = "\ue037";
const textIconDelete = "\ue15b";
const textIconRevert = "\ue166";
const textIconAdd = "\ue145";
const textIconNext = "\ue5cc";
const textIconPrev = "\ue5cb";
const textIconShare = "\ue80d";
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
//const varCBackgroundMenu = vs("rgb(173, 177, 188)", "rgb(0,0,0)");
const varCBackgroundMenu = vs("rgb(205, 208, 217)", "rgb(0,0,0)");
const varCBackgroundMenuButtons = vs("rgb(219, 223, 232)", "rgb(0,0,0)");

const varCButtonHover = vs("rgba(255, 255, 255, 0.7)", "rgb(0,0,0)");
const varCButtonClick = vs("rgba(255, 255, 255, 1)", "rgb(0,0,0)");

const varCSeekbarEmpty = vs("rgb(212, 216, 223)", "rgb(0,0,0)");
const varCSeekbarFill = vs("rgb(197, 196, 209)", "rgb(0,0,0)");

const varSButtonPad = v("0.3cm");

const varCForeground = vs("rgb(0, 0, 0)", "rgb(0,0,0)");
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
const classMenuStateOpen = "state_open";

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: all

const contGroupStyle = "group";
presentation.contGroupStyle = contGroupStyle;
const contVboxStyle = "vbox";
presentation.contVboxStyle = contVboxStyle;
const contHboxStyle = "hbox";
presentation.contHboxStyle = contHboxStyle;
const contStackStyle = "stack";
presentation.contStackStyle = contStackStyle;

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
presentation.leafTitle = /** @type {Presentation["leafTitle"]} */ (text) => ({
  root: e(
    "h1",
    {
      textContent: text,
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

presentation.contBar = /** @type {Presentation["contBar"]} */ (
  extraStyles,
  leftChildren,
  leftMidChildren,
  midChildren,
  rightMidChildren,
  rightChildren
) => {
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
          ...extraStyles,
        ],
        children_: [
          newHbox(leftChildren),
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
                newHbox(leftMidChildren),
                newHbox(midChildren),
                newHbox(rightMidChildren),
              ],
            }
          ),
          newHbox(rightChildren),
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

presentation.contBarMainForm = /** @type {Presentation["contBarMainForm"]} */ (
  leftChildren,
  leftMidChildren,
  midChildren,
  rightMidChildren,
  rightChildren
) =>
  presentation.contBar(
    [
      classMenuWantStateOpen,
      contBarMainStyle,
      ss(uniq("cont_bar_main_form"), {
        "": (s) => {
          s.backdropFilter = "brightness(1.06) blur(0.2cm)";
        },
      }),
    ],
    leftChildren,
    leftMidChildren,
    midChildren,
    rightMidChildren,
    rightChildren
  );

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
  cb
) => {
  const out = e(
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
            children_: [presentation.contSpinner().root],
          }
        ),
      ],
    }
  );
  (async () => {
    try {
      await new Promise((r) => setTimeout(() => r(null), 5000));
      const r = await cb();
      out.innerHTML = "";
      out.appendChild(r);
    } catch (e) {
      out.innerHTML = "";
      out.appendChild(presentation.leafErrBlock(/** @type {Error} */ (e)).root);
    }
  })();
  return { root: out };
};

presentation.leafErrBlock = /** @type {Presentation["leafErrBlock"]} */ (
  e1
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
            s.color = "rgb(154, 60, 74)";
          },
        }),
      ],
      children_: [e("span", { textContent: e1.message }, {})],
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

const leafButtonStyle = ss(uniq("leaf_button"), {
  ":hover": (s) => {
    s.backgroundColor = varCButtonHover;
  },
  ":hover:active": (s) => {
    s.backgroundColor = varCButtonClick;
  },
});
presentation.leafButton = /** @type {Presentation["leafButton"]} */ (
  title,
  text,
  extraStyles,
  onclick
) => {
  const out = e(
    "button",
    {
      title: title,
      textContent: text,
      onclick: onclick,
    },
    {
      styles_: [leafButtonStyle, ...extraStyles],
    }
  );
  return { root: out, button: out };
};

presentation.leafBarButtonBig =
  /** @type {Presentation["leafBarButtonBig"]} */ (title, text) =>
    presentation.leafButton(title, text, [
      ss(uniq("leaf_button_bar_big"), {
        "": (s) => {
          s.padding = varSButtonPad;
        },
      }),
    ]);

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
  label,
  inputId,
  input
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
            textContent: label,
            htmlFor: inputId,
          },
          {}
        ),
        input,
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
});

presentation.leafInputText = /** @type {Presentation["leafInputText"]} */ (
  id,
  title,
  value
) => {
  const out =
    /** @type {HTMLInputElement} */
    (
      e(
        "input",
        {
          type: "text",
          placeholder: title,
          title: title,
          name: id,
          value: value,
        },
        {
          styles_: [leafInputStyle],
        }
      )
    );
  return { root: out, input: out };
};
presentation.leafInputPairText =
  /** @type {Presentation["leafInputPairText"]} */ (id, title, value) => {
    const input = presentation.leafInputText(id, title, value);
    return {
      root: presentation.leafInputPair(title, id, input.root).root,
      input: input.input,
    };
  };

presentation.leafInputSelect = /** @type {Presentation["leafInputSelect"]} */ (
  id,
  children
) => {
  const out = /** @type {HTMLSelectElement} */ (
    e(
      "select",
      {
        name: id,
      },
      {
        styles_: [leafInputStyle],
        children_: children,
      }
    )
  );
  return { root: out, input: out };
};

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: page, view

presentation.contPageView = /** @type {Presentation["contPageView"]} */ (
  entries
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
      children_: entries,
    }
  ),
});

presentation.contBarViewTransport =
  /** @type {Presentation["contBarViewTransport"]} */ () => {
    const buttonShare = presentation.leafTransportButton(
      "Share",
      textIconShare
    );
    const buttonPrev = presentation.leafTransportButton(
      "Previous",
      textIconPrev
    );
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
          e(
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
          ),
        ],
      }
    );
    const buttonNext = presentation.leafTransportButton("Next", textIconNext);
    const buttonPlay = presentation.leafTransportButton("Play", textIconPlay);
    return {
      root: presentation.contBar(
        [
          classMenuWantStateOpen,
          contBarMainStyle,
          ss(uniq("cont_bar_main_transport"), {
            "": (s) => {
              s.backdropFilter = "blur(0.2cm)";
            },
          }),
        ],
        [buttonShare.root],
        [],
        [buttonPrev.root, seekbar, buttonNext.root],
        [buttonPlay.root],
        []
      ).root,
      buttonShare: buttonShare.button,
      buttonNext: buttonNext.button,
      buttonPlay: buttonPlay.button,
      buttonPrev: buttonPrev.button,
      seekbar: seekbar,
      seekbarFill: seekbarFill,
    };
  };

presentation.contMediaFullscreen =
  /** @type {Presentation["contMediaFullscreen"]} */ (media) => {
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
            media,
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

presentation.contModal = /** @type {Presentation["contModal"]} */ (
  title,
  child
) => {
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
          e(
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
          ),
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
                          textContent: title,
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
                      presentation.leafSpace().root,
                      buttonClose,
                    ],
                  }
                ),
                child,
              ],
            }
          ),
        ],
      }
    ),
    buttonClose: buttonClose,
  };
};

presentation.leafTransportButton =
  /** @type {Presentation["leafTransportButton"]} */ (title, icon) => {
    const size = "1cm";
    const svgId = window.crypto.randomUUID();
    const div = e("div", {}, {});
    div.style.clipPath = `url(#${svgId})`;
    const out = e(
      "button",
      {
        title: title,
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
        ],
        children_: [
          // Debug by adding 0-1 viewbox and moving text outside of defs/clipPath; y is scaled by scale so 100x it
          et(`
        <svg width="0" height="0">
          <defs>
            <clipPath id="${svgId}" clipPathUnits="objectBoundingBox">
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
      `),
          div,
        ],
      }
    );
    return { root: out, button: out };
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
  entries
) => ({
  root: e(
    "div",
    {},
    {
      styles_: [classMenuWantStateOpen, contBodyNarrowStyle, contVboxStyle],
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
            children_: entries,
          }
        ),
        presentation.leafSpace().root,
      ],
    }
  ),
});

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: page, edit

var varSEditGap = v("0.5cm");

presentation.contPageEdit = /** @type {Presentation["contPageEdit"]} */ (
  children
) => ({
  root: e(
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
      children_: children,
    }
  ),
});

presentation.contPageEditSectionRel =
  /** @type {Presentation["contPageEditSectionRel"]} */ (children) => ({
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
        children_: children,
      }
    ),
  });

presentation.leafButtonEditFree =
  /** @type {Presentation["leafButtonEditFree"]} */ (icon, hint) =>
    presentation.leafButton(hint, icon, [
      leafIconStyle,
      ss(uniq("leaf_button_free"), {
        "": (s) => {
          s.fontSize = "22pt";
          s.fontWeight = "300";
          const size = "1.2cm";
          s.width = size;
          s.height = size;
          s.borderRadius = "0.2cm";
          s.color = `color-mix(in srgb, ${varCForeground}, transparent 30%)`;
        },
        ":hover": (s) => {
          s.color = varCForeground;
        },
        ":hover:active": (s) => {
          s.color = varCForeground;
        },
      }),
    ]);

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
  id,
  nodeHint,
  nodeType,
  node
) => {
  const inputType = presentation.leafInputSelect(`${id}_type`, [
    e("option", { textContent: "Value", value: "value" }, {}),
    e("option", { textContent: "File", value: "file" }, {}),
  ]);
  inputType.input.value = nodeType;
  const inputValue = presentation.leafInputText(id, nodeHint, node);
  const buttonDelete = presentation.leafButtonEditFree(
    textIconDelete,
    "Delete"
  );
  const buttonRevert = presentation.leafButtonEditFree(
    textIconRevert,
    "Revert"
  );
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
                inputType.root,
                presentation.leafSpace().root,
                buttonDelete.root,
                buttonRevert.root,
              ],
            }
          ),
          inputValue.root,
        ],
      }
    ),
    inputType: inputType.input,
    inputValue: inputValue.input,
    buttonDelete: buttonDelete.button,
    buttonRevert: buttonRevert.button,
  };
};

presentation.leafEditPredicate =
  /** @type {Presentation["leafEditPredicate"]} */ (id, value) =>
    presentation.leafInputText(id, "Predicate", value);

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

presentation.leafEditRowIncoming =
  /** @type {Presentation["leafEditRowIncoming"]} */ (children) => ({
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
              children_: children,
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

presentation.leafEditRowOutgoing =
  /** @type {Presentation["leafEditRowOutgoing"]} */ (children) => ({
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
              children_: children,
            }
          ),
        ],
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

presentation.contBodyMenu = /** @type {Presentation["contBodyMenu"]} */ () => ({
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
  children
) =>
  presentation.contBar(
    [
      ss(uniq("cont_bar_menu"), {
        "": (s) => {
          s.gridColumn = "1/3";

          s.backgroundColor = varCBackgroundMenuButtons;
          s.margin = "0.5cm 0";
        },
      }),
    ],
    [],
    [],
    [],
    [],
    children
  );

presentation.contMenuGroup = /** @type {Presentation["contMenuGroup"]} */ (
  title,
  children
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
              e("span", { textContent: title }, {}),
            ],
          }
        ),
        e(
          "div",
          {},
          {
            styles_: [contVboxStyle, contMenuGroupVBoxStyle],
            children_: children,
          }
        ),
      ],
    }
  ),
});

presentation.leafMenuLink = /** @type {Presentation["leafMenuLink"]} */ (
  title,
  href
) => ({
  root: e(
    "div",
    {},
    {
      children_: [
        e(
          "a",
          {
            href: href,
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
              e("span", { textContent: title }, {}),
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
});

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: Main

presentation.appMain = /** @type {Presentation["appMain"]} */ (children) => ({
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
          left: presentation.leafTitle("Music").root,
          right: presentation.leafButton(
            "Menu",
            textIconMenu,
            [
              leafIconStyle,
              ss(uniq("cont_main_title_admenu"), {
                "": (s) => {
                  s.gridColumn = "3";
                  s.gridRow = "1";
                  s.fontSize = varSFontAdmenu;
                  s.width = varSCol3Width;
                  s.height = varSCol3Width;
                },
              }),
            ],
            (() => {
              let state = false;
              return () => {
                state = !state;
                for (const e of document.getElementsByClassName(
                  classMenuWantStateOpen
                )) {
                  e.classList.toggle(classMenuStateOpen, state);
                }
              };
            })()
          ).root,
        }).root,
        ...children,
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
                left: presentation.leafTitle("Menu").root,
              }).root,
              e(
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
                        children_: [
                          presentation.leafMenuLink("Thing 1", "x").root,
                          presentation.leafMenuLink("Thing 2", "x").root,
                          presentation.leafMenuLink("Thing 3", "x").root,
                          presentation.contMenuGroup("Group 1", [
                            presentation.leafMenuLink("Thing 1", "x").root,
                            presentation.leafMenuLink("Thing 2", "x").root,
                            presentation.leafMenuLink("Thing 3", "x").root,
                          ]).root,
                        ],
                      }
                    ),
                    presentation.contBarMenu([
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
                      presentation.leafBarButtonBig("Login", "Login").root,
                    ]).root,
                    presentation.leafSpace().root,
                  ],
                }
              ),
            ],
          }
        ),
      ],
    }
  ),
});

///////////////////////////////////////////////////////////////////////////////
// xx PLUGINS

presentation.buildView = /** @type {Presentation["buildView"]} */ async (
  pluginPath,
  args
) => {
  const plugin = await import(`./${pluginPath}`);
  return { root: plugin.build(args) };
};

///////////////////////////////////////////////////////////////////////////////
// xx Assemble

window.sunwet_presentation = presentation;

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: staging
addEventListener("DOMContentLoaded", async (_) => {
  const htmlStyle = s(uniq("html"), {
    "": (s) => {
      s.fontFamily = "X";
      s.backgroundColor = varCBackground;
      s.color = varCForeground;
    },
  });
  notnull(document.body.parentElement).classList.add(htmlStyle);
  document.body.classList.add(contStackStyle);

  const hash = location.hash;
  if (hash == "#view") {
    document.body.appendChild(
      presentation.appMain([
        (await presentation.buildView("plugin_view_list.js", {})).root,
      ]).root
      /*
      presentation.appMain([
        presentation.contBarViewTransport().root,
        presentation.contPageView([
          presentation.contViewList({
            direction: "down",
            children: [
              presentation.contViewList({
                direction: "right",
                children: [
                  presentation.leafViewImage({
                    align: "start",
                    url: "testcover.jpg",
                    width: "min(6cm, 40%)",
                  }).root,
                  presentation.contViewList({
                    direction: "down",
                    children: [
                      presentation.leafViewText({
                        align: "start",
                        orientation: "right_down",
                        text: "Harmônicos",
                        fontSize: "20pt",
                      }).root,
                      presentation.contViewTable({
                        orientation: "right_down",
                        xScroll: true,
                        children: [
                          [
                            presentation.leafViewPlay({
                              align: "start",
                              direction: "down",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: "1. ",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: "Fabiano do Nascimento and Shin Sasakubo",
                              url: "abcd-xyzg",
                              maxSize: "6cm",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: " - ",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: "Primeiro Encontro",
                              maxSize: "6cm",
                            }).root,
                          ],
                          [
                            presentation.leafViewPlay({
                              align: "start",
                              direction: "down",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: "2. ",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: "Fabiano do Nascimento and Shin Sasakubo",
                              url: "abcd-xyzg",
                              maxSize: "6cm",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: " - ",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: "Primeiro Encontro",
                              maxSize: "6cm",
                            }).root,
                          ],
                        ],
                      }).root,
                    ],
                  }).root,
                ],
              }).root,
              presentation.contViewList({
                direction: "right",
                children: [
                  presentation.leafViewImage({
                    align: "start",
                    url: "testcover.jpg",
                    width: "6cm",
                  }).root,
                  presentation.contViewList({
                    direction: "down",
                    children: [
                      presentation.leafViewText({
                        align: "start",
                        orientation: "right_down",
                        text: "Harmônicos",
                      }).root,
                      presentation.contViewTable({
                        orientation: "right_down",
                        xScroll: true,
                        children: [
                          [
                            presentation.leafViewPlay({
                              align: "start",
                              direction: "down",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: "1. ",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: "Fabiano do Nascimento and Shin Sasakubo",
                              url: "abcd-xyzg",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: " - ",
                            }).root,
                            presentation.leafViewText({
                              align: "start",
                              orientation: "down_left",
                              text: "Primeiro Encontro",
                            }).root,
                          ],
                        ],
                      }).root,
                    ],
                  }).root,
                ],
              }).root,
            ],
          }).root,
        ]).root,
        presentation.contModal(
          "Share",
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
            }
          )
        ).root,
      ]).root
      */
    );
  } else if (hash == "#fullscreen") {
    document.body.appendChild(
      presentation.contMediaFullscreen(
        e(
          "div",
          {},
          {
            styles_: [
              ss(uniq("test"), {
                "": (s) => {
                  s.border = "1px solid blue";
                },
              }),
            ],
          }
        )
      ).root
    );
  } else if (hash == "#form") {
    document.body.appendChild(
      presentation.appMain([
        presentation.contBarMainForm(
          [],
          [],
          [],
          [],
          [presentation.leafBarButtonBig("Save", "Save").root]
        ).root,
        presentation.contPageForm([
          presentation.leafInputPairText("item1", "Title", "ABCD").root,
          presentation.leafInputPairText("item2", "Text", "WXYC").root,
          presentation.leafSpace().root,
        ]).root,
      ]).root
    );
  } else if (hash == "#edit") {
    document.body.appendChild(
      presentation.appMain([
        presentation.contBarMainForm(
          [],
          [],
          [],
          [],
          [presentation.leafBarButtonBig("Save", "Save").root]
        ).root,
        presentation.contPageEdit([
          presentation.leafEditRowIncoming([
            presentation.leafButtonEditFree(textIconAdd, "Add incoming triple")
              .root,
          ]).root,
          presentation.contPageEditSectionRel([
            presentation.leafEditRowIncoming([
              presentation.leafEditNode(uniq(), "Subject", "value", "WXYZ-9999")
                .root,
              presentation.leafEditPredicate(uniq(), "sunwet/1/is").root,
            ]).root,
            presentation.leafEditRowIncoming([
              presentation.leafEditNode(uniq(), "Subject", "file", "LMNO-4567")
                .root,
              presentation.leafEditPredicate(uniq(), "sunwet/1/has").root,
            ]).root,
          ]).root,
          e(
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
              children_: [
                presentation.leafEditNode(
                  uniq(),
                  "Current node",
                  "value",
                  "ABCD-01234"
                ).root,
              ],
            }
          ),
          presentation.contPageEditSectionRel([
            presentation.leafEditRowOutgoing([
              presentation.leafEditNode(uniq(), "Subject", "file", "WXYZ-9999")
                .root,
              presentation.leafEditPredicate(uniq(), "sunwet/1/is").root,
            ]).root,
            presentation.leafEditRowOutgoing([
              presentation.leafEditNode(uniq(), "Subject", "value", "LMNO-4567")
                .root,
              presentation.leafEditPredicate(uniq(), "sunwet/1/has").root,
            ]).root,
          ]).root,
          presentation.leafEditRowOutgoing([
            presentation.leafButtonEditFree(textIconAdd, "Add outgoing triple")
              .root,
          ]).root,
        ]).root,
      ]).root
    );
  } else {
    throw new Error();
  }
});
