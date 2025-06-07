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
        params: [
          presentation.leafInputPairText({
            id: "",
            title: "Artist",
            value: "",
          }).root,
        ],
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
                      wrap: false,
                      transAlign: "start",
                      xScroll: false,
                      children: [
                        presentation.leafViewText({
                          transAlign: "start",
                          orientation: "right_down",
                          text: "Harmônicos",
                          fontSize: "20pt",
                        }).root,
                        presentation.leafViewDatetime({
                          transAlign: "start",
                          orientation: "right_down",
                          value: new Date().toISOString(),
                          fontSize: "14pt",
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
                                image: "testcover.jpg",
                                width: "1.5cm",
                                height: "1.5cm",
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
                      transAlign: "start",
                      wrap: false,
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

    const buildRoot = /** @type {(e: Element)=>void} */ (e) => {
      document.body.appendChild(
        presentation.contRootStack({ children: [e] }).root
      );
    };

    const hash = location.hash;
    switch (hash) {
      case "#main_async":
        {
          buildRoot(
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
          buildRoot(
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
          buildRoot(
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
          buildRoot(stagingPageView);
        }
        break;
      case "#menu":
        {
          buildRoot(stagingPageView);
          for (const e of document.getElementsByClassName(
            presentation.classMenuWantStateOpen({}).value
          )) {
            e.classList.add(presentation.classMenuStateOpen({}).value);
          }
        }
        break;
      case "#view_modal_share":
        {
          buildRoot(stagingPageView);
          buildRoot(
            presentation.contModalViewShare({
              qr: /** @type {HTMLElement} */ (
                new DOMParser().parseFromString(
                  `
            <svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 580 580">
    <path fill="currentColor" d="M240 80h20v20h-20zm20 0h20v20h-20zm0 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm0 20h20v20h-20zm-80 20h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm-80 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm-80 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm-60 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm-80 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zM80 240h20v20H80zm80 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm-380 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm100 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm-360 20h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zM80 300h20v20H80zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zM80 320h20v20H80zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-240 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-240 20h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-200 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm100 0h20v20h-20zm-240 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-200 20h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm100 0h20v20h-20zm20 0h20v20h-20zm40 0h20v20h-20zm-220 20h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm-180 20h20v20h-20zm80 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm-240 20h20v20h-20zm40 0h20v20h-20zm40 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm20 0h20v20h-20zm60 0h20v20h-20zm40 0h20v20h-20z" shape-rendering="crispEdges"/>
    <path fill="currentColor" fill-rule="evenodd" d="M80 80h140v140H80Zm20 20h100v100H100Zm260-20h140v140H360Zm20 20h100v100H380Z" shape-rendering="crispEdges"/>
    <path fill="currentColor" d="M120 120h60v60h-60zm280 0h60v60h-60z" shape-rendering="crispEdges"/>
    <path fill="currentColor" fill-rule="evenodd" d="M80 360h140v140H80Zm20 20h100v100H100Z" shape-rendering="crispEdges"/>
    <path fill="currentColor" d="M120 400h60v60h-60z" shape-rendering="crispEdges"/>
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
          buildRoot(
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
          buildRoot(
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Music" }).root,
              mainBody: presentation.contPageForm({
                entries: [
                  presentation.leafErrBlock({
                    data: "This is an error of greatest magnitude",
                    inRoot: false,
                  }).root,
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
                    value: "2024-08-23T22:10:10",
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
          buildRoot(
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Music" }).root,
              mainBody: presentation.contPageNode({
                pageButtonChildren: [
                  presentation.leafButtonSmallEdit({ link: "abcd" }).root,
                ],
                barChildren: [],
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
          buildRoot(
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "Music" }).root,
              mainBody: presentation.contPageNode({
                pageButtonChildren: [
                  presentation.leafButtonSmallView({ link: "abcd" }).root,
                ],
                children: [
                  presentation.contNodeRowIncomingAdd({
                    hint: "Add incoming triple",
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
                  presentation.contNodeRowOutgoingAdd({
                    hint: "Add outgoing triple",
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
          buildRoot(
            presentation.appMain({
              mainTitle: presentation.leafTitle({ text: "History" }).root,
              mainBody: presentation.contPageNode({
                pageButtonChildren: [],
                barChildren: [presentation.leafButtonBigSave({}).root],
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
                                value:
                                  "ABCD-1234 this is a ton of data PLUS-A_VERY_LONG_IDENTIFIERleafNodeViewNodeText and some unbreakable",
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
          buildRoot(presentation.appLinkPerms({}).root);
        }
        break;
      case "#link_waiting":
        {
          buildRoot(presentation.appLink({}).root);
        }
        break;
      case "#link":
        {
          const a = presentation.appLink({});
          const cover = document.createElement("img");
          cover.src = "testcover.jpg";
          a.displayOver.innerHTML = "";
          a.display.appendChild(cover);
          buildRoot(a.root);
        }
        break;
      case "#media_comic":
        {
          const baseUrl =
            "/mnt/home-dev/r/server3/servers/main/stage/testmedia/xmen/";
          /** @type { {rtl: boolean, pages: {width: number, height: number, path: string }[]} } */
          const manifest = await (await fetch(`${baseUrl}sunwet.json`)).json();

          const children = [];
          var minAspect = 1;
          for (let i = 0; i < manifest.pages.length; i += 1) {
            const page = manifest.pages[i];

            const img = presentation.leafMediaComicPage({
              src: `${baseUrl}${page.path}`,
              aspectX: page.width.toString(),
              aspectY: page.height.toString(),
            }).root;
            const vertAspect = page.width / page.height;
            if (vertAspect < minAspect) {
              minAspect = vertAspect;
            }

            if (i == 0) {
              children.push(presentation.leafMediaComicEndPad({}).root);
            } else if (i % 2 == 1) {
              children.push(presentation.leafMediaComicMidPad({}).root);
            }
            children.push(img);
            if (i == manifest.pages.length - 1) {
              children.push(presentation.leafMediaComicEndPad({}).root);
            }
          }
          buildRoot(
            presentation.contMediaFullscreen({
              media: presentation.contMediaComicOuter({
                children: [
                  presentation.contMediaComicInner({
                    minAspectX: minAspect.toString(),
                    minAspectY: "1",
                    children: children,
                    rtl: true,
                  }).root,
                ],
              }).root,
            }).root
          );
        }
        break;
      default:
        throw new Error();
    }
  });
}
