/// <reference path="style_export.d.ts" />
/// <reference path="style_export2.d.ts" />
{
  const presentation = window.sunwetPresentation;
  addEventListener("DOMContentLoaded", async (_) => {
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
    const stagingPageView = presentation.appMain({
      mainTitle: presentation.leafTitle({ text: "Music" }).root,
      mainBody: presentation.contPageViewList({
        transport: presentation.contBarViewTransport({}).root,
        rows: presentation.contViewList({
          direction: "down",
          xScroll: false,
          children: [
            presentation.contViewList({
              direction: "right",
              xScroll: false,
              children: [
                presentation.leafViewImage({
                  transAlign: "start",
                  url: "testcover.jpg",
                  width: "min(6cm, 40%)",
                }).root,
                presentation.contViewList({
                  direction: "down",
                  xScroll: false,
                  children: [
                    presentation.leafViewText({
                      transAlign: "start",
                      orientation: "right_down",
                      text: "Harmônicos",
                      fontSize: "20pt",
                    }).root,
                    presentation.contViewTable({
                      orientation: "right_down",
                      xScroll: true,
                      children: [
                        [
                          presentation.leafViewPlayButton({
                            transAlign: "start",
                            direction: "down",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: "1. ",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: "Fabiano do Nascimento and Shin Sasakubo",
                            url: "abcd-xyzg",
                            maxSize: "6cm",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: " - ",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: "Primeiro Encontro",
                            maxSize: "6cm",
                          }).root,
                        ],
                        [
                          presentation.leafViewPlayButton({
                            transAlign: "start",
                            direction: "down",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: "2. ",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: "Fabiano do Nascimento and Shin Sasakubo",
                            url: "abcd-xyzg",
                            maxSize: "6cm",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: " - ",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
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
              xScroll: false,
              children: [
                presentation.leafViewImage({
                  transAlign: "start",
                  url: "testcover.jpg",
                  width: "6cm",
                }).root,
                presentation.contViewList({
                  direction: "down",
                  xScroll: false,
                  children: [
                    presentation.leafViewText({
                      transAlign: "start",
                      orientation: "right_down",
                      text: "Harmônicos",
                    }).root,
                    presentation.contViewTable({
                      orientation: "right_down",
                      xScroll: true,
                      children: [
                        [
                          presentation.leafViewPlayButton({
                            transAlign: "start",
                            direction: "down",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: "1. ",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: "Fabiano do Nascimento and Shin Sasakubo",
                            url: "abcd-xyzg",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
                            orientation: "down_left",
                            text: " - ",
                          }).root,
                          presentation.leafViewText({
                            transAlign: "start",
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
      }).root,
      menuBody: stagingMenu,
    }).root;

    const hash = location.hash;
    if (hash == "#view") {
      document.body.appendChild(stagingPageView);
    } else if (hash == "#view_modal_share") {
      document.body.appendChild(stagingPageView);
      document.body.appendChild(
        presentation.contModalViewShare({
          qr: /** @type {HTMLElement} */ (
            new DOMParser().parseFromString(
              `
            <svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 580 580">
    <path d="M240 80h20v20h-20zm20 0h20v20h-20zm0 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm0 20h20v20h-20zm-80 20h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm-80 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm-80 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm-60 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm-80 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zM80 240h20v20H80zm80 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm-380 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm100 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm-360 20h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zM80 300h20v20H80zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zM80 320h20v20H80zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-240 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-240 20h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-200 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm100 0h20v20h-20zm-240 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-200 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm100 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm-220 20h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm-180 20h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-240 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm40 0h20v20h-20z" shape-rendering="crispEdges"/>
    <path fill-rule="evenodd" d="M80 80h140v140H80Zm20 20h100v100H100Zm260-20h140v140H360Zm20 20h100v100H380Z" shape-rendering="crispEdges"/>
    <path d="M120 120h60v60h-60zm280 0h60v60h-60z" shape-rendering="crispEdges"/>
    <path fill-rule="evenodd" d="M80 360h140v140H80Zm20 20h100v100H100Z" shape-rendering="crispEdges"/>
    <path d="M120 400h60v60h-60z" shape-rendering="crispEdges"/>
  </svg>
            `,
              "text/html"
            ).body.firstElementChild
          ),
          link: "https://a.b.c",
        }).root
      );
    } else if (hash == "#fullscreen") {
      const media = document.createElement("div");
      media.style.border = "1px solid blue";
      document.body.appendChild(
        presentation.contMediaFullscreen({
          media: media,
        }).root
      );
    } else if (hash == "#form") {
      const errInput = presentation.leafInputPairText({
        id: "item2",
        title: "Text",
        value: "WXYC",
      });
      errInput.input.classList.add(presentation.classStateInvalid({}).value);
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
              presentation.leafButtonBig({ title: "Save", text: "Save" }).root,
            ],
          }).root,
          menuBody: stagingMenu,
        }).root
      );
    } else if (hash == "#edit") {
      /** @type { (args: {hint: string, value: string})=> HTMLSelectElement} */
      const nodeTypeSel = (args) =>
        window.sunwetPresentation.leafInputEnum({
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
              presentation.leafButtonBig({ title: "Save", text: "Save" }).root,
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
}
