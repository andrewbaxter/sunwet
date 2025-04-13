///////////////////////////////////////////////////////////////////////////////
// xx Utility, globals

const uniq = (args) => {
  const uniq = [];
  for (const e of new Error().stack.matchAll(/(\d+):\d+/g)) {
    uniq.push(e[1]);
  }
  uniq.push(args);
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
const varCBackground = vs("rgb(205, 207, 212)", "rgb(0,0,0)");
const varCBg2 = vs("rgb(218, 220, 226)", "rgb(0,0,0)");
const varCBackgroundMenuButtonHover = vs(varCBg2, "rgb(0,0,0)");
const varCBackgroundMenuButtonClick = vs("rgb(226, 229, 237)", "rgb(0,0,0)");
const varCBackgroundMenu = vs("rgb(173, 177, 188)", "rgb(0,0,0)");
const varCBackgroundMenuButtons = vs("rgb(183, 187, 199)", "rgb(0,0,0)");
const varCBackgroundMenuButtonsHover = vs("rgb(196, 200, 213)", "rgb(0,0,0)");
const varCBackgroundMenuButtonsClick = vs("rgb(202, 206, 219)", "rgb(0,0,0)");
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

/** @type { (extraStyles: string[], children: HTMLElement[]) => HTMLElement} */
const newContBar = (extraStyles, children) =>
  e("div", {
    styles_: [
      ss(uniq("cont_bar"), {
        "": (s) => {
          s.zIndex = "2";
          s.display = "grid";
        },
      }),
      ...extraStyles,
    ],
    children_: [
      e("div", {
        styles_: [
          contHBoxStyle,
          ss(uniq(), {
            "": (s) => {
              s.gridColumn = "2";
              s.justifyContent = "flex-end";
              s.alignItems = "center";
            },
            ">button": (s) => {
              s.padding = "0.3cm";
              s.backgroundColor = "rgba(255, 255, 255, 0)";
            },
            ">button:hover": (s) => {
              s.backgroundColor = "rgba(255, 255, 255, 0.7)";
            },
            ">button:hover:active": (s) => {
              s.backgroundColor = "rgba(255, 255, 255, 1)";
            },
          }),
        ],
        children_: children,
      }),
    ],
  });

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
    s.border = `0.04cm solid ${varCInputBorder}`;
    //s.backgroundColor = varCInputBackground;
    s.padding = "0.1cm";
    s.borderRadius = "0.2cm";
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
// xx Components, styles: page, form

const contPageFormStyle = s(uniq("cont_page_form"), {
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
});
const contBodyNarrowStyle = s(uniq("cont_body_narrow"), {
  "": (s) => {
    s.gridRow = "2";
    s.gridColumn = "1/4";
    s.width = `min(20cm, 100% - ${varSCol1Width} * 2)`;
    s.justifySelf = "center";
    s.marginBottom = "1cm";
  },
  [`.${classMenuStateOpen}`]: (s) => {
    s.display = "none";
  },
});
/** @type { (entries: HTMLElement[]) => HTMLElement} */
const newContPageForm = (entries) =>
  e("div", {
    styles_: [contPageFormStyle, contBodyNarrowStyle],
    children_: entries,
  });

///////////////////////////////////////////////////////////////////////////////
// xx Components, styles: page, edit

const contPageEditStyle = s(uniq("page_edit"), {
  "": (s) => {
    s.gap = "0.5cm";
  },
});
/** @type { (children: HTMLElement[]) => HTMLElement} */
const newContPageEdit = (children) =>
  e("div", {
    styles_: [
      classMenuWantStateOpen,
      contBodyNarrowStyle,
      contVBoxStyle,
      contPageEditStyle,
    ],
    children_: children,
  });

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
const leafButtonFreeStyle = s(uniq("leaf_button_free"), {
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
    s.border = `0.06cm solid ${varCBackground}`;
    s.backgroundColor = varCEditButtonFreeHover;
    s.color = varCForeground;
  },
  ":hover:active": (s) => {
    s.border = `0.06cm solid ${varCBackground}`;
    s.backgroundColor = varCEditButtonFreeClick;
    s.color = varCForeground;
  },
});
const leafButtonFreeEndStyle = s(uniq("leaf_button_free_incoming"), {
  "": (s) => {
    s.alignSelf = "center";
  },
});
/** @type { (id: string, nodeHint: string, node: string, revert: ()=>void) => HTMLElement} */
const newLeafEditNode = (id, nodeHint, node, revert) => {
  /** @type { (icon: string, hint: string, click?: ()=>void) => HTMLElement} */
  const newButton = (icon, hint, click) =>
    e("button", {
      title: hint,
      styles_: [leafButtonFreeStyle, leafIconStyle],
      textContent: icon,
      onclick: click,
    });
  const inputSelect = newLeafInputSelect(`${id}_type`, [
    e("option", { textContent: "Value" }),
    e("option", { textContent: "File" }),
  ]);
  const inputText = newLeafInputText(id, nodeHint, node);
  return e("div", {
    styles_: [contVBoxStyle, contEditNodeVboxStyle],
    children_: [
      e("div", {
        styles_: [contHBoxStyle, contEditNodeHboxStyle],
        children_: [
          inputSelect,
          newLeafSpace(),
          newButton("\ue15b", "Delete"),
          newButton("\ue166", "Revert", () => {
            inputSelect.value = originalType;
            inputText.value = node;
            revert();
          }),
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
  document.body.parentElement.classList.add(htmlStyle);
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
      right: e("button", {
        styles_: [
          leafIconStyle,
          s(uniq("cont_main_title_admenu"), {
            "": (s) => {
              s.gridColumn = "3";
              s.gridRow = "1";
              s.fontSize = varSFontAdmenu;
              s.width = varSCol3Width;
              s.height = varSCol3Width;
            },
            ":hover": (s) => {
              s.backgroundColor = varCBackgroundMenuButtonHover;
            },
            ":hover:active": (s) => {
              s.backgroundColor = varCBackgroundMenuButtonClick;
            },
          }),
        ],
        textContent: "\ue5d2",
        onclick: (() => {
          let state = false;
          return () => {
            state = !state;
            for (const e of document.getElementsByClassName(
              classMenuWantStateOpen
            )) {
              e.classList.toggle(classMenuStateOpen, state);
            }
          };
        })(),
      }),
    })
  );
  document.body.appendChild(
    newContBar(
      [
        classMenuWantStateOpen,
        ss(uniq("cont_bar_main"), {
          "": (s) => {
            s.position = "fixed";
            s.bottom = "0.7cm";
            s.backdropFilter = "brightness(1.1) blur(0.2cm)";
            s.transition = "0.03s opacity";
            s.opacity = "1";
            s.width = "100%";
            s.gridTemplateColumns = `${varSCol1Width} auto ${varSCol3Width}`;
          },
          [`.${classMenuStateOpen}`]: (s) => {
            s.opacity = "0";
          },
        }),
      ],
      [e("button", { textContent: "Save" })]
    )
  );
  //   document.body.appendChild(
  //     newContPageForm([
  //       newLeafInputPairText("item1", "Title", "ABCD"),
  //       newLeafInputPairText("item2", "Text", "WXYC"),
  //     ])
  //   );
  document.body.appendChild(
    newContPageEdit([
      newLeafEditRowIncoming([
        e("button", {
          styles_: [leafIconStyle, leafButtonFreeStyle, leafButtonFreeEndStyle],
          textContent: "\ue145",
        }),
      ]),
      e("div", {
        styles_: [contVBoxStyle, contPageEditStyle],
        children_: [
          newLeafEditRowIncoming([
            newLeafEditNode(uniq(), "Subject", "WXYZ-9999"),
            newLeafEditPredicate(uniq(), "sunwet/1/is"),
          ]),
          newLeafEditRowIncoming([
            newLeafEditNode(uniq(), "Subject", "LMNO-4567"),
            newLeafEditPredicate(uniq(), "sunwet/1/has"),
          ]),
        ],
      }),
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
        children_: [newLeafEditNode(uniq(), "Current node", "ABCD-01234")],
      }),
      e("div", {
        styles_: [contVBoxStyle, contPageEditStyle],
        children_: [
          newLeafEditRowOutgoing([
            newLeafEditNode(uniq(), "Subject", "WXYZ-9999"),
            newLeafEditPredicate(uniq(), "sunwet/1/is"),
          ]),
          newLeafEditRowOutgoing([
            newLeafEditNode(uniq(), "Subject", "LMNO-4567"),
            newLeafEditPredicate(uniq(), "sunwet/1/has"),
          ]),
        ],
      }),
      newLeafEditRowOutgoing([
        e("button", {
          styles_: [leafIconStyle, leafButtonFreeStyle, leafButtonFreeEndStyle],
          textContent: "\ue145",
        }),
      ]),
    ])
  );

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
            s.filter = "drop-shadow(0.05cm 0px 0.05cm rgba(0, 0, 0, 0.1))";
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
            newContBar(
              [
                ss(uniq("cont_bar_menu"), {
                  "": (s) => {
                    s.gridColumn = "1/3";

                    s.backgroundColor = varCBackgroundMenuButtons;
                    s.margin = "0.5cm 0";

                    s.display = "grid";
                    s.gridTemplateColumns = "subgrid";
                  },
                }),
              ],
              [
                e("span", {
                  styles_: [
                    s(uniq("cont_bar_menu_user"), {
                      "": (s) => {
                        s.opacity = "0.5";
                      },
                    }),
                  ],
                  textContent: "Guest",
                }),
                e("button", {
                  styles_: [
                    s(uniq("cont_bar_menu_button"), {
                      "": (s) => {
                        s.backgroundColor = varCBackgroundMenuButtons;
                      },
                      ":hover": (s) => {
                        s.backgroundColor = varCBackgroundMenuButtonsHover;
                      },
                      ":hover:active": (s) => {
                        s.backgroundColor = varCBackgroundMenuButtonsClick;
                      },
                    }),
                  ],
                  textContent: "Login",
                }),
              ]
            ),
            newLeafSpace(),
          ],
        }),
      ],
    })
  );
});
