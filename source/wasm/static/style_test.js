/// <reference path="style_export.d.ts" />
/// <reference path="style_export2.d.ts" />
/// <reference path="style_shared.d.ts" />
addEventListener("DOMContentLoaded", async (_) => {
  const htmlStyle = ss(uniq("html"), {
    "": (s) => {
      s.fontFamily = "X";
      s.backgroundColor = varCBackground;
      s.color = varCForeground;
    },
  });
  notnull(document.body.parentElement).classList.add(htmlStyle);
  document.body.classList.add(contStackStyle);

  const stagingMenu = presentation.contMenuBody({
    children: [
      presentation.leafMenuLink({
        title: "Thing 1",
        href: "x",
      }).root,
      presentation.leafMenuLink({
        title: "Thing 2",
        href: "x",
      }).root,
      presentation.leafMenuLink({
        title: "Thing 3",
        href: "x",
      }).root,
      presentation.contMenuGroup({
        title: "Group 1",
        children: [
          presentation.leafMenuLink({
            title: "Thing 1",
            href: "x",
          }).root,
          presentation.leafMenuLink({
            title: "Thing 2",
            href: "x",
          }).root,
          presentation.leafMenuLink({
            title: "Thing 3",
            href: "x",
          }).root,
        ],
      }).root,
    ],
  }).root;

  const hash = location.hash;
  if (hash == "#view") {
    window.sunwet = {
      query: async (id, data) => {
        return [];
      },
      fileUrl: (file) => {
        return `http-test-${file}`;
      },
      editUrl: (node) => {
        return `http-test-edit-${JSON.stringify(node)}`;
      },
      setPlaylist: (playlist) => {},
      togglePlay: (index) => {},
    };
    document.body.appendChild(
      presentation.appMain({
        mainTitle: presentation.leafTitle({ text: "Music" }).root,
        mainBody: await presentation.buildView({
          pluginPath: "plugin_view_list.js",
          arguments: {},
        }).root,
        menuBody: stagingMenu,
      }).root
    );
  } else if (hash == "#fullscreen") {
    document.body.appendChild(
      presentation.contMediaFullscreen({
        media: e(
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
        ),
      }).root
    );
  } else if (hash == "#form") {
    const errInput = presentation.leafInputPairText({
      id: "item2",
      title: "Text",
      value: "WXYC",
    });
    errInput.input.classList.add(classInputStateInvalid);
    document.body.appendChild(
      presentation.appMain({
        mainTitle: presentation.leafTitle({ text: "Music" }).root,
        mainBody: presentation.contPageForm({
          entries: [
            presentation.leafInputPairText({
              id: "item1",
              title: "Title",
              value: "ABCD",
            }).root,
            errInput.root,
            presentation.leafInputPairNumber({
              id: "item2",
              title: "Text",
              value: "44",
            }).root,
            presentation.leafFormComment({
              text: "This next item is a checkbox.\n\nThis text has multiple paragraphs.",
            }).root,
            presentation.leafInputPairBool({
              id: "item2",
              title: "Text",
              value: true,
            }).root,
            presentation.leafInputPairDate({
              id: "item2",
              title: "Text",
              value: "2024-08-23",
            }).root,
            presentation.leafInputPairTime({
              id: "item2",
              title: "Text",
              value: "22:10:10",
            }).root,
            presentation.leafInputPairDatetime({
              id: "item2",
              title: "Text",
              value: "2024-08-23 22:10:10",
            }).root,
            presentation.leafInputPairColor({
              id: "item2",
              title: "Text",
              value: "#445566",
            }).root,
            presentation.leafSpace({}).root,
          ],
          barChildren: [
            presentation.leafBarButtonBig({ title: "Save", text: "Save" }).root,
          ],
        }).root,
        menuBody: stagingMenu,
      }).root
    );
  } else if (hash == "#edit") {
    /** @type { (args: {hint: string, value: string})=> HTMLSelectElement} */
    const nodeTypeSel = (args) =>
      window.sunwet_presentation.leafInputEnum({
        title: `${args.hint} type`,
        value: args.value,
        options: {
          file: "File",
          string: "String",
          bool: "Boolean",
          number: "Number",
          json: "JSON",
        },
      }).root;
    document.body.appendChild(
      presentation.appMain({
        mainTitle: presentation.leafTitle({ text: "Music" }).root,
        mainBody: presentation.contPageEdit({
          children: [
            presentation.contEditRowIncoming({
              children: [
                presentation.leafButtonEditAdd({
                  hint: "Add incoming triple",
                }).root,
              ],
            }).root,
            presentation.contPageEditSectionRel({
              children: [
                presentation.contEditRowIncoming({
                  children: [
                    presentation.leafEditNode({
                      inputType: nodeTypeSel({
                        hint: "Subject",
                        value: "file",
                      }),
                      inputValue: presentation.leafInputText({
                        id: uniq(),
                        title: "Subject",
                        value: "ABCD-1234",
                      }).root,
                    }).root,
                    presentation.leafEditPredicate({
                      value: "sunwet/1/is",
                    }).root,
                  ],
                }).root,
                presentation.contEditRowIncoming({
                  children: [
                    presentation.leafEditNode({
                      inputType: nodeTypeSel({
                        hint: "Subject",
                        value: "file",
                      }),
                      inputValue: presentation.leafInputText({
                        id: uniq(),
                        title: "Subject",
                        value: "ABCD-1234",
                      }).root,
                    }).root,
                    presentation.leafEditPredicate({
                      value: "sunwet/1/has",
                    }).root,
                  ],
                }).root,
              ],
            }).root,
            presentation.contEditSectionCenter({
              child: presentation.leafEditNode({
                inputType: nodeTypeSel({
                  hint: "Subject",
                  value: "file",
                }),
                inputValue: presentation.leafInputText({
                  id: uniq(),
                  title: "Subject",
                  value: "ABCD-1234",
                }).root,
              }).root,
            }).root,
            presentation.contPageEditSectionRel({
              children: [
                presentation.contEditRowOutgoing({
                  children: [
                    presentation.leafEditNode({
                      inputType: nodeTypeSel({
                        hint: "Subject",
                        value: "file",
                      }),
                      inputValue: presentation.leafInputText({
                        id: uniq(),
                        title: "Subject",
                        value: "ABCD-1234",
                      }).root,
                    }).root,
                    presentation.leafEditPredicate({
                      value: "sunwet/1/is",
                    }).root,
                  ],
                }).root,
                presentation.contEditRowOutgoing({
                  children: [
                    presentation.leafEditNode({
                      inputType: nodeTypeSel({
                        hint: "Subject",
                        value: "file",
                      }),
                      inputValue: presentation.leafInputText({
                        title: "Subject",
                        value: "ABCD-1234",
                      }).root,
                    }).root,
                    presentation.leafEditPredicate({
                      value: "sunwet/1/has",
                    }).root,
                  ],
                }).root,
              ],
            }).root,
            presentation.contEditRowOutgoing({
              children: [
                presentation.leafButtonEditAdd({
                  hint: "Add outgoing triple",
                }).root,
              ],
            }).root,
          ],
          barChildren: [
            presentation.leafBarButtonBig({ title: "Save", text: "Save" }).root,
          ],
        }).root,
        menuBody: stagingMenu,
      }).root
    );
  } else if (hash == "#link") {
    document.body.appendChild(presentation.appLink({}).root);
  } else {
    throw new Error();
  }
});
