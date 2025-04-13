///////////////////////////////////////////////////////////////////////////////
// Utility, globals

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

///////////////////////////////////////////////////////////////////////////////
// Variables

const varSPadTitle = v("0.4cm");
const varSFontTitle = v("24pt");
const varSFontAdmenu = v("20pt");
const varSFontMenu = v("20pt");
const varSColWidth = v("min(100dvw, 12cm)");
const varSEditRelWidth = v("1.5cm");
const varCBackground = vs("rgb(205, 207, 212)", "rgb(0,0,0)");
const varCBg2 = vs("rgb(218, 220, 226)", "rgb(0,0,0)");
const varCBackgroundMenuButtonHover = vs("var(--c-bg2)", "rgb(0,0,0)");
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

// State classes

const classMenuWantStateOpen = "want_state_open";
const classMenuStateOpen = "state_open";

///////////////////////////////////////////////////////////////////////////////
// Components, styles: all

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

const contBarStyle = s(uniq("cont_bar"), {
  "": (s) => {
    s.width = "100%";
    s.zIndex = "2";
  },
});
const contBarMainStyle = s(uniq("cont_bar_main"), {
  "": (s) => {
    s.gridColumn = "1/4";
    s.position = "fixed";
    s.bottom = "0.7cm";
    s.backdropFilter = "brightness(1.1) blur(0.2cm)";
  },
  ">button": (s) => {
    s.padding = "0.3cm";
    s.backgroundColor = "rgba(255, 255, 255, 0)";
  },
  ">button:hover": (s) => {
    s.backgroundColor = "rgba(255, 255, 255, 0.7)";
  },
  ">button:active": (s) => {
    s.backgroundColor = "rgba(255, 255, 255, 1)";
  },
});
/** @type { (extraStyle: string, children: HTMLElement[]) => HTMLElement} */
const newContBar = (extraStyle, children) =>
  e("div", {
    styles_: [contBarStyle, extraStyle],
    children_: children,
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
// Components, styles: page, form + edit
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
    s.backgroundColor = varCInputBackground;
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
// Components, styles: page, form

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
    s.gridColumn = "2";
    s.width = "min(12cm, 100dvw)";
    s.justifySelf = "center";
    s.marginBottom = "2cm";
  },
});
/** @type { (entries: HTMLElement[]) => HTMLElement} */
const newContPageForm = (entries) =>
  e("div", {
    styles_: [contPageFormStyle, contBodyNarrowStyle],
    children_: entries,
  });

///////////////////////////////////////////////////////////////////////////////
// Components, styles: page, edit

const contPageEditStyle = s(uniq("page_edit"), {
  "": (s) => {
    s.gap = "0.5cm";
  },
});
/** @type { (children: HTMLElement[]) => HTMLElement} */
const newContPageEdit = (children) =>
  e("div", {
    styles_: [contBodyNarrowStyle, contVBoxStyle, contPageEditStyle],
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
  },
  ":hover": (s) => {
    s.backgroundColor = varCEditCenter;
  },
  ":active": (s) => {
    s.backgroundColor = varCEditCenter;
  },
});
/** @type { (id: string, nodeHint: string, node: string) => HTMLElement} */
const newLeafEditNode = (id, nodeHint, node) => {
  /** @type { (icon: string, hint: string) => HTMLElement} */
  const newButton = (icon, hint) =>
    e("button", {
      title: hint,
      styles_: [leafButtonFreeStyle, leafIconStyle],
      textContent: icon,
    });
  return e("div", {
    styles_: [contVBoxStyle, contEditNodeVboxStyle],
    children_: [
      e("div", {
        styles_: [contHBoxStyle, contEditNodeHboxStyle],
        children_: [
          newLeafInputSelect(`${id}_type`, [
            e("option", { textContent: "Value" }),
            e("option", { textContent: "File" }),
          ]),
          newLeafSpace(),
          newButton("\ue15b", "Delete"),
          newButton("\ue166", "Revert"),
        ],
      }),
      newLeafInputText(id, nodeHint, node),
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
const leafEditHBoxIncomingStyle = s(uniq("leaf_edit_incoming_hbox_incoming"), {
  "": (s) => {
    s.paddingRight = varSEditRelWidth;
  },
});
const leafEditHBoxOutgoingStyle = s(uniq("leaf_edit_incoming_hbox_outgoing"), {
  "": (s) => {
    s.paddingLeft = varSEditRelWidth;
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
    s.position = "absolute";
    s.zIndex = "-1";
  },
});
const varSEditRelOverlap = v("1.2cm");
const leafEditRelIncomingStyle = s(uniq("leaf_edit_rel_incoming"), {
  "": (s) => {
    s.gridColumn = "2";
    s.alignSelf = "end";
    s.width = "1.5cm";
    s.right = "0";
    s.bottom = `calc(-1 * ${varSEditRelOverlap})`;
    //s.marginBottom = "-1.2cm";
  },
});
const leafEditRelOutgoingStyle = s(uniq("leaf_edit_rel_incoming"), {
  "": (s) => {
    s.gridColumn = "1";
    s.alignSelf = "start";
    s.width = "1.25cm";
    s.left = "0";
    s.top = `calc(-1 * ${varSEditRelOverlap})`;
    //s.marginTop = "-1.2cm";
  },
});

/** @type { (children: HTMLElement[]) => HTMLElement} */
const newLeafEditRowIncoming = (children) =>
  e("div", {
    styles_: [contHBoxStyle, leafEditHBoxStyle, leafEditHBoxIncomingStyle],
    children_: [
      e("div", {
        styles_: [contVBoxStyle, leafEditVBoxStyle],
        children_: children,
      }),
      et(
        `
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 30 50">
          <path d="M0 0l21 15v17h9L15 50 0 32h9V22l-9-6z" fill="currentColor" />
        </svg>
        `,
        {
          styles_: [leafEditRelStyle, leafEditRelIncomingStyle],
        }
      ),
    ],
  });

/** @type { (children: HTMLElement[]) => HTMLElement} */
const newLeafEditRowOutgoing = (children) =>
  e("div", {
    styles_: [contHBoxStyle, leafEditHBoxStyle, leafEditHBoxOutgoingStyle],
    children_: [
      et(
        `
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 25 40">
          <path d="M0 0h12v20l6 4 7-7v23H2l7-7-9-5.728z" fill="currentColor" />
        </svg>
        `,
        {
          styles_: [leafEditRelStyle, leafEditRelOutgoingStyle],
        }
      ),
      e("div", {
        styles_: [contVBoxStyle, leafEditVBoxStyle],
        children_: children,
      }),
    ],
  });

///////////////////////////////////////////////////////////////////////////////
// Components, styles: menu

const contMenuGroupVBoxStyle = s(uniq("cont_menu_group0"), {
  "": (s) => {
    s.marginLeft = "0.6cm";
    s.gap = "0.3cm";
  },
});
const contBodyMenuStyle = s(uniq("cont_body_menu"), {
  "": (s) => {
    s.gridColumn = "2";
    s.gridRow = "2";
    s.columns = "min(100dvw, 12cm)";
    s.columnGap = "0.5cm";
    s.justifyContent = "start";
  },
});
/** @type { () => HTMLElement} */
const newContBodyMenu = () =>
  e("div", {
    styles_: [contVBoxStyle, contMenuGroupVBoxStyle, contBodyMenuStyle],
  });

const contMenuGroupStyle = s(uniq("cont_menu_group"), {
  [`>.${contMenuGroupVBoxStyle}`]: (s) => {
    s.padding = "0.5cm 0";
  },
  ">summary": (s) => {
    s.listStyle = "none";
    s.position = "relative";
    s.fontSize = varSFontMenu;
    s.opacity = "0.5";
  },
  ">summary:hover": (s) => {
    s.opacity = "0.8";
  },
  ">summary::before": (s) => {
    s.position = "absolute";
    s.left = "-0.6cm";
    s.bottom = "0";
    s.content = "\u0316";
    s.fontSize = "14pt";
  },
  "[open] > summary::before": (s) => {
    s.content = "\u0313";
  },
});
/** @type { (title: string, children: HTMLElement[]) => HTMLElement} */
const newContMenuGroup = (title, children) =>
  e("details", {
    styles_: [contMenuGroupStyle],
    children_: [
      e("summary", { textContent: title }),
      e("div", {
        styles_: [contVBoxStyle, contMenuGroupVBoxStyle],
        children_: children,
      }),
    ],
  });

const leafMenuLinkStyle = s(uniq("leaf_menu_link"), {
  "": (s) => {
    s.fontSize = varSFontMenu;
  },
  "::after": (s) => {
    s.content = "\u05c8";
    s.opacity = "0.3";
    s.paddingLeft = "0.5cm";
    s.fontSize = "14pt";
  },
  ":hover::after": (s) => {
    s.opacity = "1";
  },
  ":active::after": (s) => {
    s.opacity = "1";
  },
});
/** @type { (title: string, href: string) => HTMLElement} */
const newLeafMenuLink = (title, href) =>
  e("div", {
    children_: [
      e("a", { styles_: [leafMenuLinkStyle], href: href, textContent: title }),
    ],
  });

///////////////////////////////////////////////////////////////////////////////
// Components, styles: staging
addEventListener("DOMContentLoaded", (_) => {
  const htmlStyle = s(uniq("html"), {
    "": (s) => {
      s.fontFamily = "X";
      s.backgroundColor = varCBackground;
      s.color = varCForeground;
      s.height = "100vh";
      s.width = "100dvw";
      s.maxWidth = "100dvw";
      s.overflowX = "hidden";
      s.display = "grid";
      s.gridTemplateColumns = "1fr";
    },
    ">*": (s) => {
      s.gridColumn = "1";
      s.gridRow = "1";
    },
  });
  document.body.parentElement.classList.add(htmlStyle);
  const bodyStyle = s(uniq("body"), {
    "": (s) => {
      s.display = "grid";
      s.gridTemplateColumns = "min(0.8cm, 5dvw) 1fr auto";
      s.gridTemplateRows = "auto 1fr auto";
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
              const size = "1.4cm";
              s.width = size;
              s.height = size;
            },
            ":hover": (s) => {
              s.backgroundColor = varCBackgroundMenuButtonHover;
            },
            ":active": (s) => {
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
    newContBar(contBarMainStyle, [e("button", { textContent: "Save" })])
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
          styles_: [
            leafIconStyle,
            leafButtonFreeStyle,
            s(uniq("leaf_button_free_incoming"), {
              "": (s) => {
                s.alignSelf = "flex-end";
              },
            }),
          ],
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
          styles_: [leafIconStyle, leafButtonFreeStyle],
          textContent: "\ue145",
        }),
      ]),
    ])
  );

  document.body.appendChild(
    e("div", {
      id: "menu",
      styles_: [
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
          },
          [`.${classMenuStateOpen}`]: (s) => {
            s.left = "0";
          },
          [`:not(.${classMenuStateOpen})`]: (s) => {
            s.left = "-100dvw";
          },
        }),
      ],
      children_: [
        newContTitle({
          left: newLeafTitle("Menu"),
        }),
        e("div", {
          styles_: [
            classMenuWantStateOpen,
            contVBoxStyle,
            contMenuGroupVBoxStyle,
            s(uniq("cont_menu_body"), {
              "": (s) => {
                s.gridColumn = "2";
                s.gridRow = "2";
                s.columns = "min(100dvw, 12cm)";
                s.columnGap = "0.5cm";
                s.justifyContent = "start";
              },
              ">*": (s) => {
                s.maxWidth = varSColWidth;
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
          s(uniq("cont_bar_menu"), {
            "": (s) => {
              s.gridColumn = "1/3";
              s.gridRow = "3";

              s.backgroundColor = varCBackgroundMenuButtons;
              s.margin = "0.5cm 0";

              s.display = "grid";
              s.gridTemplateColumns = "subgrid";
            },
          }),
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
                  ":active": (s) => {
                    s.backgroundColor = varCBackgroundMenuButtonsClick;
                  },
                }),
              ],
              textContent: "Login",
            }),
          ]
        ),
      ],
    })
  );
});
