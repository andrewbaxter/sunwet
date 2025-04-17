/// <reference path="style.d.ts" />

///////////////////////////////////////////////////////////////////////////////
// xx Utility, globals

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

/** @type { <T>(x: T|null|undefined) => T } */
const notnull = (x) => {
  if (x == null) {
    throw Error();
  }
  return x;
};

/** @type { (...args: string[]) => string} */
const uniq = (...args) => {
  const uniq = [];
  for (const e of notnull(new Error().stack).matchAll(/(\d+):\d+/g)) {
    uniq.push(e[1]);
  }
  uniq.push(...args);
  return `r${uniq.join("_")}`;
};

/** @type { (
 *   name: keyof HTMLElementTagNameMap,
 *   args?: Partial<HTMLElementTagNameMap[name]> | {
 *     styles_?: string[],
 *     children_?: HTMLElement[],
 *   },
 * ) => HTMLElement }
 */
const e = (name, args) => {
  const out = document.createElement(name);
  if (args != null) {
    for (const [k, v] of Object.entries(args)) {
      if (k == "styles_") {
        for (const c of v) {
          out.classList.add(c);
        }
      } else if (k == "children_") {
        for (const c of v) {
          out.appendChild(c);
        }
      } else {
        // @ts-ignore
        out[k] = v;
      }
    }
  }
  return out;
};
/** @type { (
 *   markup: string,
 *   args?: {
 *     styles_?: string[],
 *     children_?: HTMLElement[],
 *   },
 * ) => HTMLElement }
 */
const et = (t, args) => {
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

const resetStyle = e("link", { rel: "stylesheet", href: "style_reset.css" });
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

/** @type { (v: string) => string } */
const v = (val) => {
  const name = `--${uniq()}`;
  globalStyleRoot.setProperty(name, val);
  return `var(${name})`;
};
/** @type { (light: string, dark: string) => string } */
const vs = (light, dark) => {
  const name = `--${uniq()}`;
  globalStyleLight.setProperty(name, light);
  globalStyleDark.setProperty(name, dark);
  return `var(${name})`;
};

/** @type { (id: string, f: {[s: string]: (r: CSSStyleDeclaration) => void}) => string } */
const s = (id, f) => {
  for (const [suffix, f1] of Object.entries(f)) {
    globalStyle.insertRule(`.${id}${suffix} {}`, 0);
    f1(/** @type { CSSStyleRule } */ (globalStyle.cssRules[0]).style);
  }
  return id;
};

const staticStyles = new Map();
// Static style - the id must be unique for every value closed on (i.e. put all the arguments in the id).
/** @type { (id: string, f: {[s: string]: (r: CSSStyleDeclaration) => void}) => string } */
const ss = (id, f) => {
  if (staticStyles.has(id)) {
    return id;
  }
  for (const [suffix, f1] of Object.entries(f)) {
    globalStyle.insertRule(`.${id}${suffix} {}`, 0);
    f1(/** @type { CSSStyleRule } */ (globalStyle.cssRules[0]).style);
  }
  return id;
};

///////////////////////////////////////////////////////////////////////////////
// xx Variables

const varSPadTitle = v("0.4cm");
const varSFontTitle = v("24pt");
const varSFontAdmenu = v("20pt");
const varSFontMenu = v("20pt");
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

const contVBoxStyle = "vbox";
const contHBoxStyle = "hbox";
const contStackStyle = "stack";

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
/** @type { (args: {left: HTMLElement, right?: HTMLElement}) => HTMLElement} */
const newContTitle = (args) => {
  const children = [args.left];
  if (args.right != null) {
    children.push(args.right);
  }
  return e("div", {
    styles_: [contTitleStyle],
    children_: children,
  });
};

const leafTitleStyle = s(uniq("leaf_title"), {
  "": (s) => {
    s.fontSize = varSFontTitle;
    s.gridColumn = "2";
    s.gridRow = "1";
  },
});
/** @type { (text: string) => HTMLElement} */
const newLeafTitle = (text) =>
  e("h1", {
    styles_: [leafTitleStyle],
    textContent: text,
  });

const leafIconStyle = s(uniq("icon"), {
  "": (s) => {
    s.display = "grid";
    s.fontFamily = "I";
    s.gridTemplateColumns = "1fr";
    s.gridTemplateRows = "1fr";
    s.justifyItems = "center";
    s.alignItems = "center";
  },
});

/** @type { (
 *    extraStyles: string[],
 *    leftChildren: HTMLElement[],
 *    leftMidChildren: HTMLElement[],
 *    midChildren: HTMLElement[],
 *    rightMidChildren: HTMLElement[],
 *    rightChildren: HTMLElement[]
 * ) => HTMLElement} */
const newContBar = (
  extraStyles,
  leftChildren,
  leftMidChildren,
  midChildren,
  rightMidChildren,
  rightChildren
) => {
  /** @type { (children: HTMLElement[]) => HTMLElement} */
  const newHbox = (children) =>
    e("div", {
      styles_: [
        contHBoxStyle,
        ss(uniq("cont_bar_hbox"), {
          "": (s) => {
            s.alignItems = "center";
            s.gap = "0.2cm";
            s.margin = `0 0.2cm`;
          },
        }),
      ],
      children_: children,
    });

  return e("div", {
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
      e("div", {
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
      }),
      newHbox(rightChildren),
    ],
  });
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

/** @type { (
 *    leftChildren: HTMLElement[],
 *    leftMidChildren: HTMLElement[],
 *    midChildren: HTMLElement[],
 *    rightMidChildren: HTMLElement[],
 *    rightChildren: HTMLElement[]
 * ) => HTMLElement} */
const newContBarMainForm = (
  leftChildren,
  leftMidChildren,
  midChildren,
  rightMidChildren,
  rightChildren
) =>
  newContBar(
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

/** @type { (
 *    leftChildren: HTMLElement[],
 *    leftMidChildren: HTMLElement[],
 *    midChildren: HTMLElement[],
 *    rightMidChildren: HTMLElement[],
 *    rightChildren: HTMLElement[]
 * ) => HTMLElement} */
const newContBarMainTransport = (
  leftChildren,
  leftMidChildren,
  midChildren,
  rightMidChildren,
  rightChildren
) =>
  newContBar(
    [
      classMenuWantStateOpen,
      contBarMainStyle,
      ss(uniq("cont_bar_main_transport"), {
        "": (s) => {
          s.backdropFilter = "blur(0.2cm)";
        },
      }),
    ],
    leftChildren,
    leftMidChildren,
    midChildren,
    rightMidChildren,
    rightChildren
  );

/** @type { (children: HTMLElement[]) => HTMLElement} */
const newContBarMenu = (children) =>
  newContBar(
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

const leafSpinnerStyle = s(uniq("leaf_spinner"), {
  "": (s) => {
    s.color = varCHighlightNode;
  },
});
/** @type { () => HTMLElement} */
const newContSpinner = () =>
  et(
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
      styles_: [leafSpinnerStyle],
    }
  );

const leafSpaceStyle = s(uniq("leaf_space"), {
  "": (s) => {
    s.flexGrow = "1";
  },
});
/** @type { () => HTMLElement} */
const newLeafSpace = () => e("div", { styles_: [leafSpaceStyle] });

/** @type { (title: string, text: string, extraStyles: string[], onclick?: ()=>void) => HTMLElement} */
const newLeafButton = (title, text, extraStyles, onclick) =>
  e("button", {
    styles_: [
      ss(uniq("leaf_button"), {
        ":hover": (s) => {
          s.backgroundColor = varCButtonHover;
        },
        ":hover:active": (s) => {
          s.backgroundColor = varCButtonClick;
        },
      }),
      ...extraStyles,
    ],
    title: title,
    textContent: text,
    onclick: onclick,
  });

/** @type { (title: string, icon: string) => HTMLElement} */
const newLeafBarButtonBig = (title, text) =>
  newLeafButton(title, text, [
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
/** @type { (label: string, inputId: string, input: HTMLElement) => HTMLElement} */
const newLeafInputPair = (label, inputId, input) =>
  e("div", {
    styles_: [leafInputPairStyle],
    children_: [
      e("label", {
        htmlFor: inputId,
        textContent: label,
      }),
      input,
    ],
  });

const leafInputStyle = s(uniq("leaf_form_input_text"), {
  "": (s) => {
    s.borderBottom = `0.04cm solid ${varCInputBorder}`;
    s.padding = "0.1cm";
    s.maxWidth = "9999cm";
  },
});

/** @type { (id: string, title: string, value: string) => HTMLElement} */
const newLeafInputText = (id, title, value) =>
  e("input", {
    styles_: [leafInputStyle],
    type: "text",
    placeholder: title,
    title: title,
    name: id,
    value: value,
  });
/** @type { (id: string, title: string, value: string) => HTMLElement} */
const newLeafInputPairText = (id, title, value) =>
  newLeafInputPair(title, id, newLeafInputText(id, title, value));

/** @type { (id: string, children: HTMLElement[]) => HTMLElement} */
const newLeafInputSelect = (id, children) =>
  e("select", {
    styles_: [leafInputStyle],
    type: "text",
    name: id,
    children_: children,
  });

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: page, view

const classViewTransverseStart = "trans_start";
const classViewTransverseMiddle = "trans_middle";
const classViewTransverseEnd = "trans_end";
const classViewConverseStart = "conv_start";
const classViewConverseMiddle = "conv_middle";
const classViewConverseEnd = "conv_end";

/** @type { (entries: HTMLElement[]) => HTMLElement} */
const newContPageView = (entries) =>
  e("div", {
    styles_: [classMenuWantStateOpen, contVBoxStyle],
    children_: entries,
  });

/** @type { (title: string, icon: string) => HTMLElement} */
const newLeafTransportButton = (title, icon) =>
  newLeafButton(title, icon, [
    leafIconStyle,
    ss(uniq("leaf_transport_button"), {
      "": (s) => {
        s.fontSize = "24pt";
        s.fontWeight = "100";
        const size = "1cm";
        s.width = size;
        s.height = size;
      },
    }),
  ]);

const contViewListStyle = ss(uniq("cont_view_list"), {
  "": (s) => {
    s.display = "flex";
  },
  [`>.${classViewTransverseStart}`]: (s) => {
    s.alignSelf = "first baseline";
  },
  [`>.${classViewTransverseMiddle}`]: (s) => {
    s.alignSelf = "middle";
  },
  [`>.${classViewTransverseEnd}`]: (s) => {
    s.alignSelf = "last baseline";
  },
});
/** @type { (args: {direction: Direction, xScroll?: boolean, children: HTMLElement[] }) => HTMLElement} */
const newContViewList = (args) => {
  const out = e("div", {
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
                  [`>.${classViewTransverseMiddle}`]: (s) => {
                    s.alignSelf = "middle";
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
                  [`>.${classViewTransverseMiddle}`]: (s) => {
                    s.alignSelf = "middle";
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
  });
  if (args.xScroll) {
    out.style.overflowX = "scroll";
  }
  return out;
};

/** @type { (args: {orientation: Orientation, xScroll?: boolean, children: HTMLElement[][] }) => HTMLElement} */
const newContViewTable = (args) => {
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
  }
  const out = e("div", {
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
        uniq("cont_view_table", args.orientation),
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
                  [`>.${classViewTransverseMiddle}`]: (s) => {
                    s.alignSelf = "middle";
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
                  [`>.${classViewTransverseMiddle}`]: (s) => {
                    s.justifySelf = "middle";
                  },
                  [`>.${classViewTransverseEnd}`]: (s) => {
                    s.justifySelf = "last baseline";
                  },
                };
            }
          }
        )()
      ),
    ],
    children_: children1,
  });
  if (args.xScroll) {
    out.style.overflowX = "scroll";
  }
  return out;
};

/** @type { (args: {align: Align, url: string, text?: string, width?: string, height?: string }) => HTMLElement} */
const newLeafViewImage = (args) => {
  const out = e("img", {
    src: args.url,
    alt: args.text,
    styles_: [
      ss(uniq("leaf_view_image"), {
        "": (s) => {
          s.objectFit = "contain";
        },
      }),
      (() => {
        switch (args.align) {
          case "start":
            return classViewTransverseStart;
          case "middle":
            return classViewTransverseMiddle;
          case "end":
            return classViewTransverseEnd;
        }
      })(),
    ],
  });
  if (args.width) {
    out.style.width = args.width;
  }
  if (args.height) {
    out.style.height = args.height;
  }
  return out;
};

/** @type { (args: {align: Align, orientation: Orientation, text: string, maxSize?: number, url?: string }) => HTMLElement} */
const newLeafViewText = (args) => {
  const baseStyle = ss(uniq("leaf_view_text"), {
    "": (s) => {
      s.pointerEvents = "initial";
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
  const alignStyle = (() => {
    switch (args.align) {
      case "start":
        return classViewTransverseStart;
      case "middle":
        return classViewTransverseMiddle;
      case "end":
        return classViewTransverseEnd;
    }
  })();
  const out = (() => {
    if (args.url == null) {
      return e("span", {
        styles_: [
          baseStyle,
          dirStyle,
          alignStyle,
          ss(uniq("leaf_view_text_url"), { "": (s) => {} }),
        ],
        textContent: args.text,
      });
    } else {
      return e("a", {
        styles_: [
          baseStyle,
          dirStyle,
          alignStyle,
          ss(uniq("leaf_view_text_nourl"), { "": (s) => {} }),
        ],
        textContent: args.text,
        href: args.url,
      });
    }
  })();
  return out;
};

/** @type { (args: {align: Align, direction: Direction }) => HTMLElement} */
const newLeafViewPlay = (args) => {
  const out = e("button", {
    styles_: [
      leafIconStyle,
      ss(uniq("leaf_view_play"), {
        "": (s) => {
          s.fontSize = "24pt";
          s.fontWeight = "100";
          const size = "1cm";
          s.width = size;
          s.height = size;
        },
      }),
      ss(uniq("leaf_view_play", args.direction), {
        "": /** @type { () => ((s: CSSStyleDeclaration) => void) } */ (
          () => {
            switch (args.direction) {
              case "up":
                return (s) => {
                  s.rotate = "270deg";
                };
              case "down":
                return (s) => {
                  s.rotate = "90deg";
                };
              case "left":
                return (s) => {
                  s.rotate = "180deg";
                };
              case "right":
                return (s) => {
                  s.rotate = "0deg";
                };
            }
          }
        )(),
      }),
      (() => {
        switch (args.align) {
          case "start":
            return classViewTransverseStart;
          case "middle":
            return classViewTransverseMiddle;
          case "end":
            return classViewTransverseEnd;
        }
      })(),
    ],
    textContent: "\ue037",
  });
  return out;
};

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: page, form

const contBodyNarrowStyle = s(uniq("cont_body_narrow"), {
  "": (s) => {
    s.gridRow = "2";
    s.gridColumn = "1/4";
    s.width = `min(20cm, 100% - ${varSCol1Width} * 2)`;
    s.justifySelf = "center";
    s.marginBottom = "2.5cm";
  },
  [`.${classMenuStateOpen}`]: (s) => {
    s.display = "none";
  },
});
/** @type { (entries: HTMLElement[]) => HTMLElement} */
const newContPageForm = (entries) =>
  e("div", {
    styles_: [classMenuWantStateOpen, contBodyNarrowStyle, contVBoxStyle],
    children_: [
      e("div", {
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
      }),
      newLeafSpace(),
    ],
  });

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: page, edit

var varSEditGap = v("0.5cm");

/** @type { (children: HTMLElement[]) => HTMLElement} */
const newContPageEdit = (children) =>
  e("div", {
    styles_: [
      classMenuWantStateOpen,
      contBodyNarrowStyle,
      contVBoxStyle,
      ss(uniq("page_edit"), {
        "": (s) => {
          s.gap = varSEditGap;
        },
      }),
    ],
    children_: children,
  });

/** @type { (children: HTMLElement[]) => HTMLElement} */
const newContPageEditSectionRel = (children) =>
  e("div", {
    styles_: [
      contVBoxStyle,
      ss(uniq("cont_page_edit_section_rel"), {
        "": (s) => {
          s.gap = varSEditGap;
        },
      }),
    ],
    children_: children,
  });

/** @type { (icon: string, hint: string) => HTMLElement} */
const newLeafButtonEditFree = (icon, hint) =>
  newLeafButton(hint, icon, [
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
/** @type { (id: string, nodeHint: string, nodeType: string, node: string) => HTMLElement} */
const newLeafEditNode = (id, nodeHint, nodeType, node) => {
  const inputSelect = /** @type {HTMLSelectElement} */ (
    newLeafInputSelect(`${id}_type`, [
      e("option", { textContent: "Value", value: "value" }),
      e("option", { textContent: "File", value: "file" }),
    ])
  );
  inputSelect.value = nodeType;
  const inputText = /** @type {HTMLInputElement} */ (
    newLeafInputText(id, nodeHint, node)
  );
  return e("div", {
    styles_: [contVBoxStyle, contEditNodeVboxStyle],
    children_: [
      e("div", {
        styles_: [contHBoxStyle, contEditNodeHboxStyle],
        children_: [
          inputSelect,
          newLeafSpace(),
          newLeafButtonEditFree("\ue15b", "Delete"),
          newLeafButtonEditFree("\ue166", "Revert"),
        ],
      }),
      inputText,
    ],
  });
};

/** @type { ( id: string, value: string) => HTMLElement} */
const newLeafEditPredicate = (id, value) =>
  newLeafInputText(id, "Predicate", value);

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

/** @type { (children: HTMLElement[]) => HTMLElement} */
const newLeafEditRowIncoming = (children) =>
  e("div", {
    styles_: [contHBoxStyle, leafEditHBoxStyle],
    children_: [
      e("div", {
        styles_: [contVBoxStyle, leafEditVBoxStyle],
        children_: children,
      }),
      e("div", {
        textContent: "\uf72d",
        styles_: [leafIconStyle, leafEditRelStyle, leafEditRelIncomingStyle],
      }),
    ],
  });

/** @type { (children: HTMLElement[]) => HTMLElement} */
const newLeafEditRowOutgoing = (children) =>
  e("div", {
    styles_: [contHBoxStyle, leafEditHBoxStyle],
    children_: [
      e("div", {
        textContent: "\uf72e",
        styles_: [leafIconStyle, leafEditRelStyle, leafEditRelOutgoingStyle],
      }),
      e("div", {
        styles_: [contVBoxStyle, leafEditVBoxStyle],
        children_: children,
      }),
    ],
  });

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: menu

const contMenuGroupVBoxStyle = s(uniq("cont_menu_group0"), {
  "": (s) => {
    s.marginLeft = "0.6cm";
    s.gap = "0.3cm";
  },
});

/** @type { () => HTMLElement} */
const newContBodyMenu = () =>
  e("div", {
    styles_: [
      contVBoxStyle,
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
  });

/** @type { (title: string, children: HTMLElement[]) => HTMLElement} */
const newContMenuGroup = (title, children) =>
  e("details", {
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
      e("summary", {
        children_: [
          e("div", {
            styles_: ["closed", leafIconStyle],
            textContent: "\ue316",
          }),
          e("div", {
            styles_: ["open", leafIconStyle],
            textContent: "\ue313",
          }),
          e("span", { textContent: title }),
        ],
      }),
      e("div", {
        styles_: [contVBoxStyle, contMenuGroupVBoxStyle],
        children_: children,
      }),
    ],
  });

/** @type { (title: string, href: string) => HTMLElement} */
const newLeafMenuLink = (title, href) =>
  e("div", {
    children_: [
      e("a", {
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
        href: href,
        children_: [
          e("span", { textContent: title }),
          e("div", {
            styles_: [leafIconStyle],
            textContent: "\ue5c8",
          }),
        ],
      }),
    ],
  });

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: staging
addEventListener("DOMContentLoaded", (_) => {
  const htmlStyle = s(uniq("html"), {
    "": (s) => {
      s.fontFamily = "X";
      s.backgroundColor = varCBackground;
      s.color = varCForeground;
    },
  });
  notnull(document.body.parentElement).classList.add(htmlStyle);
  const bodyStyle = s(uniq("body"), {
    "": (s) => {
      s.display = "grid";
      s.overflowX = "hidden";
      s.maxWidth = "100dvw";
      s.gridTemplateColumns = `${varSCol1Width} 1fr auto`;
      s.gridTemplateRows = "auto 1fr";
    },
  });
  document.body.classList.add(bodyStyle);
  document.body.appendChild(
    newContTitle({
      left: newLeafTitle("Music"),
      right: newLeafButton(
        "Menu",
        "\ue5d2",
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
      ),
    })
  );

  const hash = location.hash;
  if (hash == "#view") {
    document.body.appendChild(
      newContBarMainTransport(
        [newLeafTransportButton("Share", "\ue80d")],
        [],
        [
          newLeafTransportButton("Previous", "\ue5cb"),
          e("div", {
            styles_: [
              contStackStyle,
              ss(uniq("transport_seekbar"), {
                "": (s) => {
                  // Hack around https://github.com/w3c/csswg-drafts/issues/12081 to
                  // set a default size without affecting min-content
                  s.gridTemplateColumns = "minmax(min-content, 8cm)";

                  s.alignItems = "center";
                },
              }),
            ],
            children_: [
              e("div", {
                styles_: [
                  ss(uniq("transport_gutter"), {
                    "": (s) => {
                      s.height = "0.15cm";
                      s.borderRadius = "0.05cm";
                      s.backgroundColor = varCSeekbarEmpty;
                    },
                  }),
                ],
                children_: [
                  e("div", {
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
                  }),
                ],
              }),
              e("span", {
                textContent: "00:00",
                styles_: [
                  ss(uniq("transport_time"), {
                    "": (s) => {
                      s.opacity = "50%";
                      s.justifySelf = "end";
                      s.margin = "0 0.2cm";
                    },
                  }),
                ],
              }),
            ],
          }),
          newLeafTransportButton("Next", "\ue5cc"),
        ],
        [newLeafTransportButton("Play", "\ue037")],
        []
      )
    );
    document.body.appendChild(
      e("div", {
        styles_: [
          contStackStyle,
          classMenuWantStateOpen,
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
        children_: [
          newContPageView([
            newContViewList({
              direction: "down",
              children: [
                newContViewList({
                  direction: "right",
                  children: [
                    newLeafViewImage({
                      align: "start",
                      url: "testcover.jpg",
                      width: "6cm",
                    }),
                    newContViewList({
                      direction: "down",
                      children: [
                        newLeafViewText({
                          align: "start",
                          orientation: "right_down",
                          text: "Harmônicos",
                        }),
                        newContViewTable({
                          orientation: "right_down",
                          xScroll: true,
                          children: [
                            [
                              newLeafViewPlay({
                                align: "start",
                                direction: "down",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: "1. ",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: "Fabiano do Nascimento and Shin Sasakubo",
                                url: "abcd-xyzg",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: " - ",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: "Primeiro Encontro",
                              }),
                            ],
                            [
                              newLeafViewPlay({
                                align: "start",
                                direction: "down",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: "2. ",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: "Fabiano do Nascimento and Shin Sasakubo",
                                url: "abcd-xyzg",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: " - ",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: "Primeiro Encontro",
                              }),
                            ],
                          ],
                        }),
                      ],
                    }),
                  ],
                }),

                newContViewList({
                  direction: "right",
                  children: [
                    newLeafViewImage({
                      align: "start",
                      url: "testcover.jpg",
                      width: "6cm",
                    }),
                    newContViewList({
                      direction: "down",
                      children: [
                        newLeafViewText({
                          align: "start",
                          orientation: "right_down",
                          text: "Harmônicos",
                        }),
                        newContViewTable({
                          orientation: "right_down",
                          xScroll: true,
                          children: [
                            [
                              newLeafViewPlay({
                                align: "start",
                                direction: "down",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: "1. ",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: "Fabiano do Nascimento and Shin Sasakubo",
                                url: "abcd-xyzg",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: " - ",
                              }),
                              newLeafViewText({
                                align: "start",
                                orientation: "down_left",
                                text: "Primeiro Encontro",
                              }),
                            ],
                          ],
                        }),
                      ],
                    }),
                  ],
                }),
              ],
            }),
          ]),
        ],
      })
    );
  } else if (hash == "#form") {
    document.body.appendChild(
      newContBarMainForm([], [], [], [], [newLeafBarButtonBig("Save", "Save")])
    );
    document.body.appendChild(
      newContPageForm([
        newLeafInputPairText("item1", "Title", "ABCD"),
        newLeafInputPairText("item2", "Text", "WXYC"),
        newLeafSpace(),
      ])
    );
  } else if (hash == "#edit") {
    document.body.appendChild(
      newContBarMainForm([], [], [], [], [newLeafBarButtonBig("Save", "Save")])
    );
    document.body.appendChild(
      newContPageEdit([
        newLeafEditRowIncoming([
          newLeafButtonEditFree("\ue145", "Add incoming triple"),
        ]),
        newContPageEditSectionRel([
          newLeafEditRowIncoming([
            newLeafEditNode(uniq(), "Subject", "value", "WXYZ-9999"),
            newLeafEditPredicate(uniq(), "sunwet/1/is"),
          ]),
          newLeafEditRowIncoming([
            newLeafEditNode(uniq(), "Subject", "file", "LMNO-4567"),
            newLeafEditPredicate(uniq(), "sunwet/1/has"),
          ]),
        ]),
        e("div", {
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
            newLeafEditNode(uniq(), "Current node", "value", "ABCD-01234"),
          ],
        }),
        newContPageEditSectionRel([
          newLeafEditRowOutgoing([
            newLeafEditNode(uniq(), "Subject", "file", "WXYZ-9999"),
            newLeafEditPredicate(uniq(), "sunwet/1/is"),
          ]),
          newLeafEditRowOutgoing([
            newLeafEditNode(uniq(), "Subject", "value", "LMNO-4567"),
            newLeafEditPredicate(uniq(), "sunwet/1/has"),
          ]),
        ]),
        newLeafEditRowOutgoing([
          newLeafButtonEditFree("\ue145", "Add outgoing triple"),
        ]),
      ])
    );
  } else {
    throw new Error();
  }

  document.body.appendChild(
    e("div", {
      id: "menu",
      styles_: [
        classMenuWantStateOpen,
        s(uniq("cont_menu"), {
          "": (s) => {
            s.zIndex = "3";
            s.gridRow = "1/4";
            s.gridColumn = "1/3";
            s.backgroundColor = varCBackgroundMenu;
            s.filter = "drop-shadow(0.05cm 0px 0.05cm rgba(0, 0, 0, 0.06))";
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
        newContTitle({
          left: newLeafTitle("Menu"),
        }),
        e("div", {
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
            e("div", {
              styles_: [
                classMenuWantStateOpen,
                contVBoxStyle,
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
                newLeafMenuLink("Thing 1", "x"),
                newLeafMenuLink("Thing 2", "x"),
                newLeafMenuLink("Thing 3", "x"),
                newContMenuGroup("Group 1", [
                  newLeafMenuLink("Thing 1", "x"),
                  newLeafMenuLink("Thing 2", "x"),
                  newLeafMenuLink("Thing 3", "x"),
                ]),
              ],
            }),
            newContBarMenu([
              e("span", {
                styles_: [
                  ss(uniq("cont_bar_menu_user"), {
                    "": (s) => {
                      s.opacity = "0.5";
                    },
                  }),
                ],
                textContent: "Guest",
              }),
              newLeafBarButtonBig("Login", "Login"),
            ]),
            newLeafSpace(),
          ],
        }),
      ],
    })
  );
});
