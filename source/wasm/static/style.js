const uniq = (args) => {
    const uniq = [];
    for (const e of (new Error()).stack.matchAll(/(\d+):\d+/)) {
        uniq.push(e[1]);
    }
    uniq.push(args);
    return uniq.join("_")
};

/** @type { (
 *   name: keyof HTMLElementTagNameMap,
 *   args?: Partial<HTMLElementTagNameMap[name]> | {
 *     children?: HTMLElement[],
 *   },
 * ) => HTMLElement } 
 */
const e = (name, args) => {
    const out = document.createElement(name);
    if (args != null) {
        for (const [k, v] of Object.entries(args)) {
            if (k == "children") {
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
 *   stylesCtx: Map<String, CSSStyleSheet>,
 *   name: keyof HTMLElementTagNameMap,
 *   args?: Partial<HTMLElementTagNameMap[name]> | {
 *     styles?: {id: string, style: CSSStyleSheet}[],
 *     children?: HTMLElement[],
 *   },
 * ) => HTMLElement } 
 */
const es = (stylesCtx, name, args) => {
    const out = document.createElement(name);
    if (args != null) {
        for (const [k, v] of Object.entries(args)) {
            if (k == "styles") {
                if (stylesCtx == null) {
                    throw new Error("e without stylesCtx");
                }
                for (const c of v) {
                    out.classList.add(c.id);
                    stylesCtx.set(c.id, c.style);
                }
            } else if (k == "children") {
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

const resetStyle = e("link", { rel: "stylesheet", href: "style_reset.css" });

const globalStyle = new CSSStyleSheet();
globalStyle.insertRule(`:root {}`);
const globalStyleRoot = /** @type { CSSStyleRule } */ (globalStyle.cssRules[globalStyle.cssRules.length - 1]).style;

/** @type { (v: string) => string } */
const v = val => {
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
const varSPadTitle = v('0.4cm');
const varSFontTitle = v("24pt");
const varSFontIconMenu = v('20pt');
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
const varCInputBackground = vs("var(--c-bg2)", "rgb(0,0,0)");

/** @type { (f: (r: CSSStyleDeclaration) => void) => {id: string, style: CSSStyleSheet} } */
const s = f => {
    const sheet = new CSSStyleSheet();
    const id = uniq();
    sheet.insertRule(`.${id} {}`, 0);
    f((/** @type { CSSStyleRule } */(sheet.cssRules[0])).style);
    return { id: id, style: sheet };
};
/** @type { (suffix: string, f: (r: CSSStyleDeclaration) => void) => {id: string, style: CSSStyleSheet} } */
const sw = (suffix, f) => {
    const sheet = new CSSStyleSheet();
    const id = uniq();
    sheet.insertRule(`.${id}${suffix} {}`, 0);
    f((/** @type { CSSStyleRule } */(sheet.cssRules[0])).style);
    return { id: id, style: sheet };
};

const et = t => {
    const out = document.createElement("div");
    out.outerHTML = t;
    return out;
};

/** @type { <A>(f: (styleCtx: Map<string, CSSStyleSheet>) => HTMLElement ) => HTMLElement} */
const newShadow = (f) => {
    const out = e("div");
    const s = out.attachShadow({ mode: "open" });
    s.appendChild(resetStyle);
    s.adoptedStyleSheets.push(globalStyle)
    const styleCtx = new Map();
    s.appendChild(f(styleCtx));
    for (const style of styleCtx.values()) {
        s.adoptedStyleSheets.push(style)
    }
    return out;
};

const newContTitle = (
    /** @type { () => (args: {left: HTMLElement, right: HTMLElement}) => HTMLElement} */
    () => {
        const style = s(s => {
            s.margin = `${varSPadTitle} 0`;
            s.alignItems = "center";
            s.display = "grid";
            s.gridTemplateColumns = "subgrid";
        });
        return args => newShadow(styleCtx => {
            return es(styleCtx, "div", {
                styles: [style], children: [args.left, args.right]
            });
        },
        );
    })();

const newLeafTitle = (
    /** @type { () => (col: number, row: number, text: string) => HTMLElement} */
    () => {
        const style = s(s => {
            s.fontSize = varSFontTitle;
        });
        return (col, row, text) => newShadow(
            styleCtx => es(styleCtx, "h1", {
                styles: s(s => {
                    s.gridColumn = `${col}`;
                    s.gridRow = `${row}`;
                }),
                textContent: text,
            })
        );
    }
)();

const newContBar = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newContPageForm = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newContBodyNarrow = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newContPageEdit = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newContSpinner = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newLeafEditNode = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newLeafEditRowIncoming = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newLeafEditRowOutgoing = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newContMenuGroup = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newLeafMenuGroup = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();
const newLeafMenuLink = (
    /** @type { () => () => HTMLElement} */
    () => {
        return () => newShadow(styleCtx => es(styleCtx, "", {}));
    }
)();

const iconStyle = s(s => {
    s.display = "grid";
    s.fontFamily = "I";
    s.gridTemplateColumns = "1fr";
    s.gridTemplateRows = "1fr";
    s.justifyItems = "center";
    s.alignItems = "center";
});

const newMenuButton = (
    /** @type { () => ((args: {addStyles: {id:string,style:CSSStyleSheet}[]}) => HTMLElement) } */
    () => {
        const menuButtonStyle = s(s => {
            s.gridColumn = "3";
            s.gridRow = "1";
            s.fontSize = varSFontIconMenu;
            const size = "1.4cm";
            s.width = size;
            s.height = size;
        });
        return args => newShadow(styleCtx => {
            return es(styleCtx, "button", {
                styles: [iconStyle, menuButtonStyle, ...args.addStyles],
                textContent: "\ue5d2",
                onclick: () => {
                    document.getElementById('menu').classList.toggle('menu_open')
                }
            });
        });
    }
)();

newContTitle({
    left: newLeafTitle(3, 1, ""), right: newMenuButton({
        addStyles: [
            sw(":hover", s => {
                s.backgroundColor = varCBackgroundMenuButtonHover;
            }),
            sw(":active", s => {
                s.backgroundColor = varCBackgroundMenuButtonClick;
            })]
    })
});

newContTitle({
    left: newLeafTitle(3, 1, "Menu"), right: newMenuButton({
        addStyles: [
            sw(":hover", s => {
                s.backgroundColor = varCBackgroundMenuButtonHover;
            }),
            sw(":active", s => {
                s.backgroundColor = varCBackgroundMenuButtonClick;
            })]
    })
});