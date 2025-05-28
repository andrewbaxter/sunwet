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
      user: "Guest",
      barChildren: [presentation.leafMenuBarButtonLogin({}).root],
    }).root;
    const stagingPageView = presentation.appMain({
      mainTitle: presentation.leafTitle({ text: "Music" }).root,
      mainBody: presentation.contPageView({
        transport: presentation.contBarViewTransport({}).root,
        rows: presentation.contViewRootRows({
          rows: [
            presentation.contViewRow({
              blocks: [
                presentation.contViewBlock({
                  width: "6cm",
                  children: [
                    presentation.leafViewImage({
                      transAlign: "start",
                      src: "testcover.jpg",
                      width: "100%",
                    }).root,
                  ],
                }).root,
                presentation.contViewBlock({
                  children: [
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
                          gap: "0.2cm",
                          children: [
                            [
                              presentation.leafViewPlayButton({
                                transAlign: "start",
                                orientation: "down_left",
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
                                link: "abcd-xyzg",
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
                                orientation: "down_left",
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
                                link: "abcd-xyzg",
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
              ],
            }).root,
            presentation.contViewRow({
              blocks: [
                presentation.contViewBlock({
                  width: "6cm",
                  children: [
                    presentation.leafViewImage({
                      transAlign: "start",
                      src: "testcover.jpg",
                      width: "100%",
                    }).root,
                  ],
                }).root,
                presentation.contViewBlock({
                  children: [
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
                          gap: "0.2cm",
                          children: [
                            [
                              presentation.leafViewPlayButton({
                                transAlign: "start",
                                orientation: "down_left",
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
                                link: "abcd-xyzg",
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
          ],
        }).root,
      }).root,
      menuBody: stagingMenu,
    }).root;

    const hash = location.hash;
    switch (hash) {
      case "#main_async":
        {
          document.body.appendChild(
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "" }).root,
              mainBody: presentation.leafAsyncBlock({
                inRoot: true,
              }).root,
              menuBody: stagingMenu,
            }).root
          );
        }
        break;
      case "#main_err":
        {
          document.body.appendChild(
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "" }).root,
              mainBody: presentation.leafErrBlock({
                data: "This be error 503",
                inRoot: true,
              }).root,
              menuBody: stagingMenu,
            }).root
          );
        }
        break;
      case "#home":
        {
          document.body.appendChild(
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "" }).root,
              mainBody: presentation.contPageHome({}).root,
              menuBody: stagingMenu,
            }).root
          );
        }
        break;
      case "#view":
        {
          document.body.appendChild(stagingPageView);
        }
        break;
      case "#view_modal_share":
        {
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
        }
        break;
      case "#fullscreen":
        {
          const media = document.createElement("div");
          media.style.border = "1px solid blue";
          document.body.appendChild(
            presentation.contMediaFullscreen({
              media: media,
            }).root
          );
        }
        break;
      case "#form":
        {
          const errInput = presentation.leafInputPairText({
            id: "item2",
            title: "Text",
            value: "WXYC",
          });
          errInput.input.classList.add(
            presentation.classStateInvalid({}).value
          );
          const modInput = presentation.leafInputPairText({
            id: "item1",
            title: "Title",
            value: "ABCD",
          });
          modInput.input.classList.add(
            presentation.classStateModified({}).value
          );
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
                  modInput.root,
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
                  presentation.leafInputPairFile({
                    id: "item3",
                    title: "Image",
                  }).root,
                  presentation.leafSpace({}).root,
                ],
                barChildren: [presentation.leafButtonBigSave({}).root],
              }).root,
              menuBody: stagingMenu,
            }).root
          );
        }
        break;
      case "#node_view":
        {
          document.body.appendChild(
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Music" }).root,
              mainBody: presentation.contPageNodeViewAndHistory({
                pageButtonChildren: [
                  presentation.leafButtonSmallEdit({ link: "abcd" }).root,
                ],
                children: [
                  presentation.contPageNodeSectionRel({
                    children: [
                      presentation.contNodeRowIncoming({
                        children: [
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                            link: "abcd",
                          }).root,
                          presentation.leafNodeViewPredicate({
                            value: "sunwet/1/is",
                          }).root,
                        ],
                        new: false,
                      }).root,
                      presentation.contNodeRowIncoming({
                        children: [
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                            link: "abcd",
                          }).root,
                          presentation.leafNodeViewPredicate({
                            value: "sunwet/1/has",
                          }).root,
                        ],
                        new: false,
                      }).root,
                    ],
                  }).root,
                  presentation.contNodeSectionCenter({
                    children: [
                      presentation.leafNodeViewNodeText({
                        value: "ABCD-1234",
                        link: "abcd",
                      }).root,
                    ],
                  }).root,
                  presentation.contPageNodeSectionRel({
                    children: [
                      presentation.contNodeRowOutgoing({
                        children: [
                          presentation.leafNodeViewPredicate({
                            value: "sunwet/1/has",
                          }).root,
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                            link: "abcd",
                          }).root,
                        ],
                        new: false,
                      }).root,
                      presentation.contNodeRowOutgoing({
                        children: [
                          presentation.leafNodeViewPredicate({
                            value: "sunwet/1/has",
                          }).root,
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                            link: "abcd",
                          }).root,
                        ],
                        new: false,
                      }).root,
                    ],
                  }).root,
                ],
              }).root,
              menuBody: stagingMenu,
            }).root
          );
        }
        break;
      case "#node_edit":
        {
          /** @type { (args: {hint: string, value: string})=> Element} */
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
              mainBody: presentation.contPageNodeEdit({
                pageButtonChildren: [
                  presentation.leafButtonSmallView({ link: "abcd" }).root,
                ],
                children: [
                  presentation.contNodeRowIncoming({
                    children: [
                      presentation.leafButtonNodeEditAdd({
                        hint: "Add incoming triple",
                      }).root,
                    ],
                    new: true,
                  }).root,
                  presentation.contPageNodeSectionRel({
                    children: [
                      presentation.contNodeRowIncoming({
                        children: [
                          presentation.leafNodeEditButtons({}).root,
                          presentation.leafNodeEditNode({
                            inputType: nodeTypeSel({
                              hint: "Subject",
                              value: "file",
                            }),
                            inputValue: presentation.leafInputText({
                              title: "Subject",
                              value: "ABCD-1234",
                            }).root,
                          }).root,
                          presentation.leafNodeEditPredicate({
                            value: "sunwet/1/is",
                          }).root,
                        ],
                        new: true,
                      }).root,
                      presentation.contNodeRowIncoming({
                        children: [
                          presentation.leafNodeEditButtons({}).root,
                          presentation.leafNodeEditNode({
                            inputType: nodeTypeSel({
                              hint: "Subject",
                              value: "file",
                            }),
                            inputValue: presentation.leafInputText({
                              title: "Subject",
                              value: "ABCD-1234",
                            }).root,
                          }).root,
                          presentation.leafNodeEditPredicate({
                            value: "sunwet/1/has",
                          }).root,
                        ],
                        new: false,
                      }).root,
                    ],
                  }).root,
                  presentation.contNodeSectionCenter({
                    children: [
                      presentation.leafNodeEditButtons({}).root,
                      presentation.leafNodeEditNode({
                        inputType: nodeTypeSel({
                          hint: "Subject",
                          value: "file",
                        }),
                        inputValue: presentation.leafInputText({
                          title: "Subject",
                          value: "ABCD-1234",
                        }).root,
                      }).root,
                    ],
                  }).root,
                  presentation.contPageNodeSectionRel({
                    children: [
                      presentation.contNodeRowOutgoing({
                        children: [
                          presentation.leafNodeEditButtons({}).root,
                          presentation.leafNodeEditPredicate({
                            value: "sunwet/1/is",
                          }).root,
                          presentation.leafNodeEditNode({
                            inputType: nodeTypeSel({
                              hint: "Subject",
                              value: "file",
                            }),
                            inputValue: presentation.leafInputText({
                              title: "Subject",
                              value: "ABCD-1234",
                            }).root,
                          }).root,
                        ],
                        new: false,
                      }).root,
                      presentation.contNodeRowOutgoing({
                        children: [
                          presentation.leafNodeEditButtons({}).root,
                          presentation.leafNodeEditPredicate({
                            value: "sunwet/1/has",
                          }).root,
                          presentation.leafNodeEditNode({
                            inputType: nodeTypeSel({
                              hint: "Subject",
                              value: "file",
                            }),
                            inputValue: presentation.leafInputText({
                              title: "Subject",
                              value: "ABCD-1234",
                            }).root,
                          }).root,
                        ],
                        new: true,
                      }).root,
                    ],
                  }).root,
                  presentation.contNodeRowOutgoing({
                    children: [
                      presentation.leafButtonNodeEditAdd({
                        hint: "Add outgoing triple",
                      }).root,
                    ],
                    new: true,
                  }).root,
                ],
                barChildren: [presentation.leafButtonBigSave({}).root],
              }).root,
              menuBody: stagingMenu,
            }).root
          );
        }
        break;
      case "#history":
        {
          document.body.appendChild(
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "History" }).root,
              mainBody: presentation.contPageNodeViewAndHistory({
                pageButtonChildren: [],
                children: [
                  presentation.contHistoryCommit({
                    stamp: new Date().toISOString(),
                    desc: "",
                    children: [
                      presentation.contHistorySubject({
                        center: [
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                            link: "abcd",
                          }).root,
                        ],
                        rows: [
                          presentation.contHistoryPredicateObjectRemove({
                            children: [
                              presentation.leafNodeViewPredicate({
                                value: "sunwet/1/has",
                              }).root,
                              presentation.leafNodeViewNodeText({
                                value: "ABCD-1234",
                                link: "abcd",
                              }).root,
                            ],
                          }).root,
                          presentation.contHistoryPredicateObjectAdd({
                            children: [
                              presentation.leafNodeViewPredicate({
                                value: "sunwet/1/has",
                              }).root,
                              presentation.leafNodeViewNodeText({
                                value: "ABCD-1234",
                                link: "abcd",
                              }).root,
                            ],
                          }).root,
                        ],
                      }).root,
                      presentation.contHistorySubject({
                        center: [
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                            link: "abcd",
                          }).root,
                        ],
                        rows: [
                          presentation.contHistoryPredicateObjectRemove({
                            children: [
                              presentation.leafNodeViewPredicate({
                                value: "sunwet/1/has",
                              }).root,
                              presentation.leafNodeViewNodeText({
                                value: "ABCD-1234",
                                link: "abcd",
                              }).root,
                            ],
                          }).root,
                          presentation.contHistoryPredicateObjectAdd({
                            children: [
                              presentation.leafNodeViewPredicate({
                                value: "sunwet/1/has",
                              }).root,
                              presentation.leafNodeViewNodeText({
                                value: "ABCD-1234",
                                link: "abcd",
                              }).root,
                            ],
                          }).root,
                        ],
                      }).root,
                    ],
                  }).root,
                  presentation.contHistoryCommit({
                    stamp: new Date().toISOString(),
                    desc: "Something",
                    children: [
                      presentation.contHistorySubject({
                        center: [
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                            link: "abcd",
                          }).root,
                        ],
                        rows: [
                          presentation.contHistoryPredicateObjectAdd({
                            children: [
                              presentation.leafNodeViewPredicate({
                                value: "sunwet/1/has",
                              }).root,
                              presentation.leafNodeViewNodeText({
                                value: "ABCD-1234",
                                link: "abcd",
                              }).root,
                            ],
                          }).root,
                          presentation.contHistoryPredicateObjectAdd({
                            children: [
                              presentation.leafNodeViewPredicate({
                                value: "sunwet/1/has",
                              }).root,
                              presentation.leafNodeViewNodeText({
                                value: "ABCD-1234",
                                link: "abcd",
                              }).root,
                            ],
                          }).root,
                        ],
                      }).root,
                      presentation.contHistorySubject({
                        center: [
                          presentation.leafNodeViewNodeText({
                            value: "ABCD-1234",
                            link: "abcd",
                          }).root,
                        ],
                        rows: [
                          presentation.contHistoryPredicateObjectRemove({
                            children: [
                              presentation.leafNodeViewPredicate({
                                value: "sunwet/1/has",
                              }).root,
                              presentation.leafNodeViewNodeText({
                                value: "ABCD-1234",
                                link: "abcd",
                              }).root,
                            ],
                          }).root,
                          presentation.contHistoryPredicateObjectRemove({
                            children: [
                              presentation.leafNodeViewPredicate({
                                value: "sunwet/1/has",
                              }).root,
                              presentation.leafNodeViewNodeText({
                                value: "ABCD-1234",
                                link: "abcd",
                              }).root,
                            ],
                          }).root,
                        ],
                      }).root,
                    ],
                  }).root,
                ],
              }).root,
              menuBody: stagingMenu,
            }).root
          );
        }
        break;
      case "#link_perms":
        {
          document.body.appendChild(presentation.appLinkPerms({}).root);
        }
        break;
      case "#link_waiting":
        {
          document.body.appendChild(presentation.appLink({}).root);
        }
        break;
      case "#link":
        {
          const a = presentation.appLink({});
          const cover = document.createElement("img");
          cover.src = "testcover.jpg";
          a.displayOver.innerHTML = "";
          a.display.appendChild(cover);
          document.body.appendChild(a.root);
        }
        break;
      default:
        throw new Error();
    }
  });
}
