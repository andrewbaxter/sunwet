/// <reference path="style_export.d.ts" />
/// <reference path="plugin_view_list.d.ts" />
/// <reference path="plugin.d.ts" />

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
const classViewConverseStart = "conv_start";
const classViewConverseEnd = "conv_end";

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

const newContViewList = /** @type {
    (args: {
    direction: Direction;
    xScroll?: boolean;
    children: HTMLElement[];
    gap?: string;
  }) => { root: HTMLElement }
} */ (args) => {
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

const newContViewTable = /** @type {
    (args: {
    orientation: Orientation;
    xScroll?: boolean;
    children: HTMLElement[][];
  }) => { root: HTMLElement }
} */ (args) => {
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

const newLeafViewImage = /** @type {
(args: {
    align: TransAlign;
    url: string;
    text?: string;
    width?: string;
    height?: string;
  }) => { root: HTMLElement }
} */ (args) => {
  const out = e(
    "img",
    {
      src: args.url,
      alt: args.text,
    },
    {
      styles_: [
        ss(uniq("leaf_view_image"), {
          "": (s) => {
            s.objectFit = "contain";
            s.aspectRatio = "auto";
            s.flexShrink = "0";
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
    }
  );
  if (args.width) {
    out.style.width = args.width;
  }
  if (args.height) {
    out.style.height = args.height;
  }
  return { root: out };
};

const newLeafViewText = /** @type {
    (args: {
    align: TransAlign;
    orientation: Orientation;
    text: string;
    fontSize?: string;
    maxSize?: string;
    url?: string;
  }) => { root: HTMLElement }
} */ (args) => {
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
              if (args.maxSize != null) {
                s.maxHeight = args.maxSize;
              }
            };
          case "up_right":
          case "down_right":
            return (s) => {
              s.writingMode = "vertical-lr";
              if (args.maxSize != null) {
                s.maxHeight = args.maxSize;
              }
            };
          case "left_up":
          case "left_down":
          case "right_up":
          case "right_down":
            return (s) => {
              s.writingMode = "horizontal-tb";
              if (args.maxSize != null) {
                s.maxWidth = args.maxSize;
              }
            };
        }
      }
    )(),
  });
  /** @type {string} */
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
      return e(
        "span",
        {
          textContent: args.text,
        },
        {
          styles_: [
            baseStyle,
            dirStyle,
            alignStyle,
            ss(uniq("leaf_view_text_url"), { "": (s) => {} }),
          ],
        }
      );
    } else {
      return e(
        "a",
        {
          textContent: args.text,
          href: args.url,
        },
        {
          styles_: [
            baseStyle,
            dirStyle,
            alignStyle,
            ss(uniq("leaf_view_text_nourl"), { "": (s) => {} }),
          ],
        }
      );
    }
  })();
  if (args.fontSize != null) {
    out.style.fontSize = args.fontSize;
  }
  return { root: out };
};

/** @type {<T, U>(x: T|undefined, f: (x: T)=>U)=>U|undefined} */
const maybe = (x, f) => {
  if (x == null) {
    return undefined;
  }
  return f(x);
};

const newLeafViewPlay = /** @type {
    (args: { align: TransAlign; direction: Direction }) => {
    root: HTMLElement;
    button: HTMLElement;
  }
} */ (args) => {
  const size = "1cm";
  const out = e(
    "button",
    {},
    {
      styles_: [
        leafButtonStyle,
        ss(uniq("leaf_view_play"), {
          "": (s) => {
            s.width = size;
            s.height = size;
            // Hack to override baseline using inline-block weirdness...
            // https://stackoverflow.com/questions/39373787/css-set-baseline-of-inline-block-element-manually-and-have-it-take-the-expected
            s.display = "inline-block";
            s.textWrapMode = "nowrap";
          },
        }),
        ss(
          uniq("leaf_view_play", args.direction),
          /** @type { () => ({[suffix: string]: (s: CSSStyleDeclaration) => void}) } */ (
            () => {
              switch (args.direction) {
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
      children_: [
        e(
          "div",
          {
            textContent: textIconPlay,
          },
          {
            styles_: [
              leafIconStyle,
              ss(uniq("leaf_view_play_inner"), {
                "": (s) => {
                  s.width = "100%";
                  s.height = "100%";
                  s.writingMode = "horizontal-tb";
                  s.fontSize = "24pt";
                  s.fontWeight = "100";
                },
              }),
            ],
          }
        ),
      ],
    }
  );
  return { root: out, button: out };
};

class Build1 {
  /** @type { (data: TreeNode, q: QueryOrField) => Promise<TreeNode[]> } */
  async queryOrField(data, q) {
    switch (q.type) {
      case "field":
        if (data.type != "record") {
          throw new Error(
            `Data isn't a record, can't resolve field \`${q.value}\``
          );
        }
        const found = data.value[q.value];
        if (found.type != "array") {
          throw new Error(`Specified field must be an array`);
        }
        return found.value;
      case "query":
        const params = new Map();
        for (const [paramId, paramDef] of Object.entries(q.value.params)) {
          params.set(paramId, this.fieldOrLiteral(data, paramDef));
        }
        return await window.sunwet.query(q.value.query, params);
    }
  }

  /** @type { (data: TreeNode, q: FieldOrLiteral) => TreeNode} */
  fieldOrLiteral(data, def) {
    switch (def.type) {
      case "field":
        if (data.type != "record") {
          throw new Error(
            `Data isn't a record, can't resolve field \`${def.value}\``
          );
        }
        const found = data.value[def.value];
        if (found == null) {
          throw new Error(`No field ${def.value} in data.`);
        }
        return found;
      case "literal":
        return { type: "scalar", value: { t: "v", v: def.value } };
    }
  }

  /** @type { (data: TreeNode) => string } */
  valueToString(data) {
    switch (data.type) {
      case "scalar":
        const v = data.value;
        switch (v.t) {
          case "f":
            return v.v;
          case "v":
            if (typeof v.v == "string") {
              return v.v;
            }
            return JSON.stringify(v.v);
        }
      case "array":
        return JSON.stringify(data.value);
      case "record":
        return JSON.stringify(data.value);
    }
  }

  /** @type { (data: TreeNode) => string } */
  valueToUrl(data) {
    switch (data.type) {
      case "scalar":
        const v = data.value;
        switch (v.t) {
          case "f":
            return window.sunwet.fileUrl(v.v);
          case "v":
            if (typeof v.v == "string") {
              return v.v;
            }
            return JSON.stringify(v.v);
        }
      case "array":
        return JSON.stringify(data.value);
      case "record":
        return JSON.stringify(data.value);
    }
  }

  /** @type { (data: TreeNode, view: Widget )=>HTMLElement} */
  widget(data, view) {
    switch (view.type) {
      case "layout":
        return newContViewList({
          direction: view.widget.direction,
          xScroll: view.widget.x_scroll || false,
          children: view.widget.elements.map((e) => this.widget(data, e)),
          gap: view.widget.gap,
        }).root;
      case "data_rows":
        return window.sunwet_presentation.leafAsyncBlock(async () => {
          const data1 = await this.queryOrField(data, view.widget.data);
          const row_widget = view.widget.row_widget;
          switch (row_widget.type) {
            case "other":
              return newContViewList({
                gap: row_widget.gap,
                direction: row_widget.direction,
                xScroll: view.widget.x_scroll,
                children: data1.map((row) =>
                  this.widget(row, row_widget.widget)
                ),
              }).root;
            case "table":
              return newContViewTable({
                orientation: row_widget.orientation,
                children: data1.map((row) =>
                  row_widget.elements.map((e) => this.widget(row, e))
                ),
              }).root;
          }
        }).root;
      case "text":
        const text = this.valueToString(
          this.fieldOrLiteral(data, view.widget.data)
        );
        return newLeafViewText({
          align: view.widget.trans_align || "start",
          orientation: view.widget.orientation,
          text: `${view.widget.prefix || ""}${text}${view.widget.suffix || ""}`,
          url: maybe(view.widget.link, (v) =>
            this.valueToUrl(this.fieldOrLiteral(data, v))
          ),
          maxSize: view.widget.size_max,
          fontSize: view.widget.size,
        }).root;
      case "image":
        return newLeafViewImage({
          align: view.widget.trans_align || "start",
          url: this.valueToUrl(this.fieldOrLiteral(data, view.widget.data)),
          width: view.widget.width,
          height: view.widget.height,
        }).root;
      case "play":
        return newLeafViewPlay({
          align: view.widget.trans_align || "start",
          direction: view.widget.direction || "right",
        }).root;
    }
  }
}
const build1 = new Build1();

/** @type {() => TreeNode} */
const testDataTrack = () => ({
  type: "record",
  value: {
    index: { type: "scalar", value: { t: "v", v: "1" } },
    file: { type: "scalar", value: { t: "v", v: "" } },
    artist: {
      type: "scalar",
      value: {
        t: "v",
        v: "Fabiano do Nascimento and Shin Sasakubo",
      },
    },
    name: {
      type: "scalar",
      value: { t: "v", v: "Primeiro Encontro" },
    },
  },
});
/** @type {() => TreeNode} */
const testDataAlbum = () => ({
  type: "record",
  value: {
    cover: {
      type: "scalar",
      value: { t: "v", v: "testcover.jpg" },
    },
    title: { type: "scalar", value: { t: "v", v: "Harmônicos" } },
    tracks: {
      type: "array",
      value: [testDataTrack(), testDataTrack()],
    },
  },
});
/** @type {TreeNode} */
const testData = {
  type: "record",
  value: {
    test_data: {
      type: "array",
      value: [testDataAlbum(), testDataAlbum()],
    },
  },
};
/** @type {WidgetDataRows} */
const testDef = {
  data: { type: "field", value: "test_data" },
  key_field: "id",
  row_widget: {
    type: "other",
    gap: "0.8cm",
    direction: "down",
    widget: {
      type: "layout",
      widget: {
        direction: "right",
        elements: [
          {
            type: "image",
            widget: {
              data: { type: "field", value: "cover" },
              width: "min(6cm, 40%)",
            },
          },
          {
            type: "layout",
            widget: {
              direction: "down",
              elements: [
                {
                  type: "text",
                  widget: {
                    data: {
                      type: "field",
                      value: "title",
                    },
                    size: "20pt",
                    orientation: "right_down",
                  },
                },
                {
                  type: "data_rows",
                  widget: {
                    data: { type: "field", value: "tracks" },
                    key_field: "id",
                    row_widget: {
                      type: "table",
                      orientation: "right_down",
                      elements: [
                        {
                          type: "play",
                          widget: {
                            direction: "down",
                            media_field: { type: "field", value: "file" },
                          },
                        },
                        {
                          type: "text",
                          widget: {
                            orientation: "down_left",
                            data: { type: "field", value: "index" },
                            suffix: ". ",
                          },
                        },
                        {
                          type: "text",
                          widget: {
                            orientation: "down_left",
                            data: { type: "field", value: "artist" },
                            size_max: "6cm",
                          },
                        },
                        {
                          type: "text",
                          widget: {
                            orientation: "down_left",
                            data: { type: "literal", value: " - " },
                          },
                        },
                        {
                          type: "text",
                          widget: {
                            orientation: "down_left",
                            data: { type: "field", value: "name" },
                            size_max: "6cm",
                          },
                        },
                      ],
                    },
                  },
                },
              ],
            },
          },
        ],
      },
    },
  },
};

export const build = /** @type {BuildFn} */ (args0) => {
  const args = /** @type {WidgetDataRows} */ (args0);
  return window.sunwet_presentation.contPageView([
    window.sunwet_presentation.contBarViewTransport().root,
    e(
      "div",
      {},
      {
        styles_: [
          window.sunwet_presentation.contStackStyle,
          window.sunwet_presentation.ss(
            window.sunwet_presentation.uniq("view_list_body"),
            {
              "": (s) => {
                s.padding = `0 max(0.3cm, min(${varSCol1Width}, 100dvw / 20))`;
                s.paddingBottom = "2cm";
                s.flexGrow = "1";
              },
            }
          ),
        ],
        children_: [
          build1.widget(testData, { type: "data_rows", widget: testDef }),
        ],
      }
    ),
  ]).root;
};
