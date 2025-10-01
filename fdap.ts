import * as sunwet from "./sunwet/source/generated/ts/index.ts";
import * as sortquery from "./sunwet/source/generated/ts/sub/SortQuery.ts";
import * as child_process from "child_process";
import * as process from "process";

(async () => {
  const run_output = async (cmd: string, args: string[]): Promise<string> => {
    return new Promise((yes, no) => {
      var p = child_process.spawn(cmd, args);
      var result = "";
      p.stdout.on("data", (data) => {
        result += data.toString();
      });
      p.stderr.on("data", (data) => {
        console.log("" + data);
      });
      p.on("close", (code) => {
        if (code === 0) {
          return yes(result);
        } else {
          no(
            new Error(
              `[${cmd}] [${args.join(", ")}] exited with non-zero code: ${code}`
            )
          );
        }
      });
    });
  };
  const compile_query_head_tail = async (
    head_path: string,
    tail_path: string,
    sort?: sortquery.SortQuery
  ): Promise<sunwet.Query> => {
    const head: sunwet.Query = JSON.parse(
      await run_output("sunwet", ["compile-query", "--file", head_path])
    );
    if (head.suffix != null) {
      throw new Error();
    }
    const tail: sunwet.Query = JSON.parse(
      await run_output("sunwet", ["compile-query", "--file", tail_path])
    );
    if (
      tail.suffix == null ||
      tail.chain_head.root != null ||
      tail.chain_head.steps.length > 0
    ) {
      throw new Error();
    }
    return {
      chain_head: head.chain_head,
      suffix: {
        chain_tail: tail.suffix.chain_tail,
        sort: sort,
      },
    };
  };
  const compile_query = async (
    path: string,
    sort?: sortquery.SortQuery
  ): Promise<sunwet.Query> => {
    return JSON.parse(
      await run_output("sunwet", ["compile-query", "--file", path])
    );
  };

  const widget_node_link = (link: sunwet.Link): sunwet.Widget => {
    return {
      icon: {
        color: "rgba(from var(--c-foreground) r g b / .4)",
        data: "\uf81b",
        link: link,
        width: "0.5cm",
        height: "0.5cm",
        orientation: "right_down",
        trans_align: "middle",
      },
    };
  };

  // import * as fdap_login from "./fdap-login/source/generated/ts/index";
  const album_title_block_width = "6cm";
  const album_tracks_height = "9cm";
  const display_audio_albums: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    row_blocks: [
      {
        width: album_title_block_width,
        widget: {
          layout: {
            trans_align: "end",
            direction: "down",
            elements: [
              {
                media: {
                  trans_align: "start",
                  width: "100%",
                  data: { field: "cover" },
                },
              },
              {
                text: {
                  trans_align: "start",
                  font_size: "18pt",
                  conv_size_mode: "ellipsize",
                  orientation: "right_down",
                  data: { field: "album_name" },
                  link: {
                    title: {
                      field: "album_name",
                    },
                    dest: {
                      view: {
                        id: "audio_albums_eq_album",
                        parameters: {
                          album_id: {
                            field: "album_id",
                          },
                        },
                      },
                    },
                  },
                },
              },
              {
                text: {
                  trans_align: "start",
                  font_size: "12pt",
                  conv_size_mode: "ellipsize",
                  orientation: "right_down",
                  data: { field: "album_artist_name" },
                  link: {
                    title: {
                      field: "album_artist_name",
                    },
                    dest: {
                      view: {
                        id: "audio_albums_eq_artist_by_name",
                        parameters: {
                          artist_id: {
                            field: "album_artist_id",
                          },
                        },
                      },
                    },
                  },
                },
              },
            ],
          },
        },
      },
      {
        widget: {
          data_rows: {
            data: { query: "tracks" },
            row_widget: {
              table: {
                orientation: "right_down",
                conv_scroll: true,
                gap: "0.2cm",
                trans_size_max: album_tracks_height,
                elements: [
                  {
                    play_button: {
                      trans_align: "middle",
                      orientation: "down_left",
                      media_file_field: "file",
                      name_field: "track_name",
                      album_field: "album_name",
                      artist_field: "artist_name",
                      cover_field: "cover",
                    },
                  },
                  {
                    text: {
                      trans_align: "middle",
                      data: {
                        field: "track_superindex",
                      },
                      suffix: ". ",
                      font_size: "12pt",
                      conv_size_mode: "wrap",
                      orientation: "down_left",
                    },
                  },
                  {
                    text: {
                      trans_align: "middle",
                      data: {
                        field: "track_index",
                      },
                      suffix: ". ",
                      font_size: "12pt",
                      conv_size_mode: "wrap",
                      orientation: "down_left",
                    },
                  },
                  {
                    text: {
                      trans_align: "middle",
                      data: {
                        field: "track_name",
                      },
                      link: {
                        title: {
                          field: "track_name",
                        },
                        dest: {
                          node: {
                            field: "track_id",
                          },
                        },
                      },
                      font_size: "12pt",
                      conv_size_mode: "wrap",
                      orientation: "down_left",
                    },
                  },
                ],
              },
            },
          },
        },
      },
    ],
  };

  const display_audio_albums_few: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    row_blocks: [
      {
        width: "6cm",
        widget: {
          media: {
            trans_align: "start",
            width: "100%",
            data: { field: "cover" },
          },
        },
      },
      {
        widget: {
          layout: {
            trans_align: "start",
            direction: "down",
            elements: [
              {
                layout: {
                  trans_align: "start",
                  direction: "right",
                  elements: [
                    {
                      text: {
                        trans_align: "start",
                        font_size: "18pt",
                        conv_size_mode: "ellipsize",
                        orientation: "right_down",
                        data: { field: "album_name" },
                        link: {
                          title: {
                            field: "album_name",
                          },
                          dest: {
                            view: {
                              id: "audio_albums_eq_album",
                              parameters: {
                                album_id: {
                                  field: "album_id",
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                    widget_node_link({
                      title: {
                        field: "album_name",
                      },
                      dest: {
                        node: {
                          field: "album_id",
                        },
                      },
                    }),
                  ],
                },
              },
              {
                text: {
                  trans_align: "start",
                  font_size: "12pt",
                  conv_size_mode: "ellipsize",
                  orientation: "right_down",
                  data: { field: "album_artist_name" },
                  link: {
                    title: {
                      field: "album_artist_name",
                    },
                    dest: {
                      view: {
                        id: "audio_albums_eq_artist_by_name",
                        parameters: {
                          artist_id: {
                            field: "album_artist_id",
                          },
                        },
                      },
                    },
                  },
                },
              },
              {
                data_rows: {
                  data: { query: "tracks" },
                  row_widget: {
                    table: {
                      orientation: "down_right",
                      gap: "0.2cm",
                      elements: [
                        {
                          play_button: {
                            trans_align: "middle",
                            orientation: "right_down",
                            media_file_field: "file",
                            name_field: "track_name",
                            album_field: "album_name",
                            artist_field: "artist_name",
                            cover_field: "cover",
                          },
                        },
                        {
                          text: {
                            trans_align: "middle",
                            data: {
                              field: "track_superindex",
                            },
                            suffix: ". ",
                            font_size: "12pt",
                            conv_size_mode: "wrap",
                            orientation: "right_down",
                          },
                        },
                        {
                          text: {
                            trans_align: "middle",
                            data: {
                              field: "track_index",
                            },
                            suffix: ". ",
                            font_size: "12pt",
                            conv_size_mode: "wrap",
                            orientation: "right_down",
                          },
                        },
                        {
                          text: {
                            trans_align: "middle",
                            data: {
                              field: "track_name",
                            },
                            link: {
                              title: {
                                field: "track_name",
                              },
                              dest: {
                                node: {
                                  field: "track_id",
                                },
                              },
                            },
                            font_size: "12pt",
                            conv_size_mode: "wrap",
                            conv_size_max: "8cm",
                            orientation: "right_down",
                          },
                        },
                      ],
                    },
                  },
                },
              },
            ],
          },
        },
      },
    ],
  };

  const display_audio_tracks: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    row_blocks: [
      {
        widget: {
          layout: {
            direction: "down",
            elements: [
              {
                layout: {
                  direction: "right",
                  elements: [
                    {
                      play_button: {
                        media_file_field: "file",
                        show_image: true,
                        width: "min(15dvw, 1.6cm)",
                        height: "min(15dvw, 1.6cm)",
                        name_field: "track_name",
                        album_field: "album_name",
                        artist_field: "artist_name",
                        cover_field: "cover",
                        trans_align: "start",
                      },
                    },
                    {
                      layout: {
                        direction: "down",
                        gap: "0.2cm",
                        trans_align: "middle",
                        elements: [
                          {
                            text: {
                              data: {
                                field: "track_name",
                              },
                              orientation: "right_down",
                              font_size: "14pt",
                            },
                          },
                          {
                            layout: {
                              direction: "right",
                              gap: "0",
                              wrap: true,
                              elements: [
                                {
                                  text: {
                                    data: {
                                      field: "artist_name",
                                    },
                                    suffix: " - ",
                                    link: {
                                      title: {
                                        field: "artist_name",
                                      },
                                      dest: {
                                        view: {
                                          id: "audio_albums_eq_artist_by_name",
                                          parameters: {
                                            artist_id: {
                                              field: "artist_id",
                                            },
                                          },
                                        },
                                      },
                                    },
                                    orientation: "right_down",
                                  },
                                },
                                {
                                  text: {
                                    data: {
                                      field: "album_name",
                                    },
                                    link: {
                                      title: {
                                        field: "album_name",
                                      },
                                      dest: {
                                        view: {
                                          id: "audio_albums_eq_album",
                                          parameters: {
                                            album_id: {
                                              field: "album_id",
                                            },
                                          },
                                        },
                                      },
                                    },
                                    orientation: "right_down",
                                  },
                                },
                              ],
                            },
                          },
                          "space",
                        ],
                      },
                    },
                    "space",
                  ],
                },
              },
              "space",
            ],
          },
        },
      },
    ],
  };
  const display_video_albums: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    row_blocks: [
      {
        width: "6cm",
        widget: {
          media: {
            trans_align: "start",
            width: "100%",
            data: { field: "cover" },
          },
        },
      },
      {
        widget: {
          layout: {
            trans_align: "start",
            direction: "down",
            elements: [
              {
                layout: {
                  trans_align: "start",
                  direction: "right",
                  elements: [
                    {
                      text: {
                        trans_align: "start",
                        font_size: "18pt",
                        conv_size_mode: "ellipsize",
                        orientation: "right_down",
                        data: { field: "album_name" },
                        link: {
                          title: {
                            field: "album_name",
                          },
                          dest: {
                            view: {
                              id: "video_albums_eq_album",
                              parameters: {
                                album_id: {
                                  field: "album_id",
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                    widget_node_link({
                      title: {
                        field: "album_name",
                      },
                      dest: {
                        node: {
                          field: "album_id",
                        },
                      },
                    }),
                  ],
                },
              },
              {
                data_rows: {
                  data: { query: "tracks" },
                  row_widget: {
                    table: {
                      orientation: "right_down",
                      conv_scroll: true,
                      gap: "0.2cm",
                      elements: [
                        {
                          play_button: {
                            trans_align: "middle",
                            orientation: "down_left",
                            media_file_field: "file",
                            name_field: "track_name",
                            album_field: "album_name",
                            artist_field: "artist_name",
                            cover_field: "cover",
                          },
                        },
                        {
                          text: {
                            trans_align: "middle",
                            data: {
                              field: "track_superindex",
                            },
                            suffix: ". ",
                            font_size: "12pt",
                            conv_size_mode: "wrap",
                            orientation: "down_left",
                          },
                        },
                        {
                          text: {
                            trans_align: "middle",
                            data: {
                              field: "track_index",
                            },
                            suffix: ". ",
                            font_size: "12pt",
                            conv_size_mode: "wrap",
                            orientation: "down_left",
                          },
                        },
                        {
                          text: {
                            trans_align: "middle",
                            data: {
                              field: "track_name",
                            },
                            link: {
                              title: {
                                field: "track_name",
                              },
                              dest: {
                                node: {
                                  field: "track_id",
                                },
                              },
                            },
                            font_size: "12pt",
                            conv_size_mode: "wrap",
                            conv_size_max: "8cm",
                            orientation: "down_left",
                          },
                        },
                      ],
                    },
                  },
                },
              },
            ],
          },
        },
      },
    ],
  };
  const display_comic_albums: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    row_blocks: [
      {
        widget: {
          layout: {
            trans_align: "start",
            direction: "down",
            elements: [
              {
                layout: {
                  trans_align: "start",
                  direction: "right",
                  elements: [
                    {
                      text: {
                        trans_align: "start",
                        font_size: "18pt",
                        conv_size_mode: "ellipsize",
                        orientation: "right_down",
                        data: { field: "album_name" },
                        link: {
                          title: {
                            field: "album_name",
                          },
                          dest: {
                            view: {
                              id: "comic_albums_eq_album",
                              parameters: {
                                album_id: {
                                  field: "album_id",
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                    widget_node_link({
                      title: {
                        field: "album_name",
                      },
                      dest: {
                        node: {
                          field: "album_id",
                        },
                      },
                    }),
                  ],
                },
              },
              {
                text: {
                  trans_align: "start",
                  font_size: "12pt",
                  conv_size_mode: "ellipsize",
                  orientation: "right_down",
                  data: { field: "album_artist_name" },
                  link: {
                    title: {
                      field: "album_artist_name",
                    },
                    dest: {
                      view: {
                        id: "comic_albums_eq_artist_by_name",
                        parameters: {
                          lang: { field: "lang" },
                          artist_id: {
                            field: "album_artist_id",
                          },
                        },
                      },
                    },
                  },
                },
              },
              {
                data_rows: {
                  data: { query: "tracks" },
                  row_widget: {
                    unaligned: {
                      conv_scroll: true,
                      direction: "right",
                      widget: {
                        layout: {
                          direction: "down",
                          elements: [
                            {
                              media: {
                                data: {
                                  field: "track_cover",
                                },
                                height: "5cm",
                              },
                            },
                            {
                              layout: {
                                direction: "right",
                                elements: [
                                  "space",
                                  {
                                    text: {
                                      trans_align: "middle",
                                      data: {
                                        field: "track_superindex",
                                      },
                                      suffix: ". ",
                                      orientation: "right_down",
                                      font_size: "12pt",
                                    },
                                  },
                                  {
                                    text: {
                                      trans_align: "middle",
                                      data: {
                                        field: "track_index",
                                      },
                                      orientation: "right_down",
                                      font_size: "12pt",
                                    },
                                  },
                                  {
                                    play_button: {
                                      trans_align: "middle",
                                      media_file_field: "track_file",
                                      orientation: "right_down",
                                      name_field: "track_name",
                                      album_field: "album_name",
                                      artist_field: "track_artist_name",
                                      cover_field: "track_cover",
                                    },
                                  },
                                ],
                              },
                            },
                          ],
                        },
                      },
                    },
                  },
                },
              },
            ],
          },
        },
      },
    ],
  };
  const display_book_albums: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    row_blocks: [
      {
        widget: {
          layout: {
            trans_align: "start",
            direction: "down",
            elements: [
              {
                layout: {
                  trans_align: "start",
                  direction: "right",
                  elements: [
                    {
                      text: {
                        trans_align: "start",
                        font_size: "18pt",
                        conv_size_mode: "ellipsize",
                        orientation: "right_down",
                        data: { field: "album_name" },
                        link: {
                          title: {
                            field: "album_name",
                          },
                          dest: {
                            view: {
                              id: "book_albums_eq_album",
                              parameters: {
                                album_id: {
                                  field: "album_id",
                                },
                              },
                            },
                          },
                        },
                      },
                    },
                    widget_node_link({
                      title: {
                        field: "album_name",
                      },
                      dest: {
                        node: {
                          field: "album_id",
                        },
                      },
                    }),
                  ],
                },
              },
              {
                text: {
                  trans_align: "start",
                  font_size: "12pt",
                  conv_size_mode: "ellipsize",
                  orientation: "right_down",
                  data: { field: "album_artist_name" },
                  link: {
                    title: {
                      field: "album_artist_name",
                    },
                    dest: {
                      view: {
                        id: "book_albums_eq_artist_by_name",
                        parameters: {
                          artist_id: {
                            field: "album_artist_id",
                          },
                        },
                      },
                    },
                  },
                },
              },
              {
                data_rows: {
                  data: { query: "tracks" },
                  row_widget: {
                    unaligned: {
                      conv_scroll: true,
                      direction: "right",
                      widget: {
                        layout: {
                          direction: "down",
                          elements: [
                            {
                              media: {
                                data: {
                                  field: "track_cover",
                                },
                                height: "5cm",
                              },
                            },
                            {
                              layout: {
                                direction: "right",
                                elements: [
                                  "space",
                                  {
                                    text: {
                                      data: {
                                        field: "track_superindex",
                                      },
                                      suffix: ". ",
                                      orientation: "right_down",
                                      font_size: "12pt",
                                      trans_align: "middle",
                                    },
                                  },
                                  {
                                    text: {
                                      data: {
                                        field: "track_index",
                                      },
                                      orientation: "right_down",
                                      font_size: "12pt",
                                      trans_align: "middle",
                                    },
                                  },
                                  {
                                    play_button: {
                                      media_file_field: "track_file",
                                      trans_align: "middle",
                                      orientation: "right_down",
                                      name_field: "track_name",
                                      album_field: "album_name",
                                      artist_field: "track_artist_name",
                                      cover_field: "track_cover",
                                    },
                                  },
                                ],
                              },
                            },
                          ],
                        },
                      },
                    },
                  },
                },
              },
            ],
          },
        },
      },
    ],
  };
  const display_notes: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    row_blocks: [
      {
        widget: {
          layout: {
            direction: "down",
            gap: "0.1cm",
            elements: [
              {
                layout: {
                  direction: "right",
                  elements: [
                    {
                      text: {
                        trans_align: "start",
                        font_size: "12pt",
                        color: "rgba(78, 94, 119, 0.8)",
                        conv_size_mode: "wrap",
                        prefix: "Topic: ",
                        data: { field: "topic" },
                        orientation: "right_down",
                      },
                    },
                    "space",
                    {
                      datetime: {
                        orientation: "right_down",
                        data: {
                          field: "add_timestamp",
                        },
                        font_size: "12pt",
                        color: "rgba(0,0,0,0.3)",
                      },
                    },
                    widget_node_link({
                      title: {
                        literal: {
                          t: "v",
                          v: "Node",
                        },
                      },
                      dest: {
                        node: {
                          field: "note_id",
                        },
                      },
                    }),
                  ],
                },
              },
              {
                text: {
                  trans_align: "start",
                  font_size: "12pt",
                  conv_size_mode: "wrap",
                  data: { field: "text" },
                  orientation: "right_down",
                },
              },
              {
                media: { data: { field: "file" } },
              },
            ],
          },
        },
      },
    ],
  };
  const userConfig: {
    [_: string]: {
      // "fdap-login": fdap_login.UserConfig;
      "fdap-login": any;
      sunwet: sunwet.UserConfig;
    };
  } = {
    andrew: {
      "fdap-login": { password: process.env.ANDREW_PASSWORD },
      sunwet: { iam_grants: "admin" },
    },
    andrewmobile: {
      "fdap-login": { password: process.env.ANDREWMOBILE_PASSWORD },
      sunwet: {
        iam_grants: {
          limited: {
            menu_items: [
              "audio_group",
              "comics_group",
              "books_group",
              "notes_group",
              "logs",
            ],
            views: [
              "audio_albums_eq_artist_by_name",
              "audio_albums_eq_album",
              "comic_albums_eq_artist_by_name",
              "comic_albums_eq_album",
              "book_albums_eq_artist_by_name",
              "book_albums_eq_album",
            ],
          },
        },
      },
    },
  };
  const query_audio_albums_tracks = await compile_query(
    "./sunwet/source/queries/query_audio_albums_tracks.txt"
  );
  const query_video_albums_tracks = await compile_query(
    "./sunwet/source/queries/query_video_albums_tracks.txt"
  );
  const query_comic_albums_tracks = await compile_query(
    "./sunwet/source/queries/query_comic_albums_tracks.txt"
  );
  const query_book_albums_tracks = await compile_query(
    "./sunwet/source/queries/query_book_albums_tracks.txt"
  );
  const sunwet_config: sunwet.GlobalConfig = {
    api_tokens: { [process.env.ADMIN_TOKEN]: "admin" },
    menu: [
      {
        id: "audio_group",
        name: "Music",
        detail: {
          section: {
            children: [
              {
                name: "albums by add date",
                id: "audio_albums_by_add_date",
                detail: {
                  page: {
                    view: { view_id: "audio_albums_by_add_date" },
                  },
                },
              },
              {
                id: "audio_albums_by_random",
                name: "albums by random",
                detail: {
                  page: {
                    view: { view_id: "audio_albums_by_random" },
                  },
                },
              },
              {
                id: "audio_albums_search_artists",
                name: "albums, search by artist",
                detail: {
                  page: {
                    view: {
                      view_id: "audio_albums_search_artists",
                    },
                  },
                },
              },
              {
                id: "audio_albums_search_name",
                name: "albums, search by name",
                detail: {
                  page: {
                    view: { view_id: "audio_albums_search_name" },
                  },
                },
              },
              {
                id: "audio_tracks_random",
                name: "tracks by random",
                detail: {
                  page: {
                    view: { view_id: "audio_tracks_random" },
                  },
                },
              },
              {
                id: "audio_tracks_search_artists",
                name: "tracks, search by artist",
                detail: {
                  page: {
                    view: {
                      view_id: "audio_tracks_search_artists",
                    },
                  },
                },
              },
              {
                id: "audio_tracks_search_name",
                name: "tracks, search by name",
                detail: {
                  page: {
                    view: { view_id: "audio_tracks_search_name" },
                  },
                },
              },
            ],
          },
        },
      },
      {
        id: "comics_group",
        name: "Comics",
        detail: {
          section: {
            children: [
              {
                id: "comic_albums_by_name",
                name: "by title",
                detail: {
                  page: {
                    view: {
                      view_id: "comic_albums_by_name",
                      parameters: { lang: { t: "v", v: "en" } },
                    },
                  },
                },
              },
              {
                id: "comic_albums_by_artist",
                name: "by author",
                detail: {
                  page: {
                    view: {
                      view_id: "comic_albums_by_artist",
                      parameters: { lang: { t: "v", v: "en" } },
                    },
                  },
                },
              },
              {
                id: "comic_albums_search_name",
                name: "search by name",
                detail: {
                  page: {
                    view: {
                      view_id: "comic_albums_search_name",
                      parameters: { lang: { t: "v", v: "en" } },
                    },
                  },
                },
              },
              {
                id: "comic_albums_search_artists",
                name: "search by author",
                detail: {
                  page: {
                    view: {
                      view_id: "comic_albums_search_artists",
                      parameters: { lang: { t: "v", v: "en" } },
                    },
                  },
                },
              },
            ],
          },
        },
      },
      {
        id: "books_group",
        name: "Books",
        detail: {
          section: {
            children: [
              {
                id: "book_albums_by_name",
                name: "by name",
                detail: {
                  page: {
                    view: { view_id: "book_albums_by_name" },
                  },
                },
              },
              {
                id: "book_albums_by_artist",
                name: "by artist",
                detail: {
                  page: {
                    view: { view_id: "book_albums_by_artist" },
                  },
                },
              },
              {
                id: "book_albums_search_name",
                name: "search by name",
                detail: {
                  page: {
                    view: { view_id: "book_albums_search_name" },
                  },
                },
              },
              {
                id: "book_albums_search_artist",
                name: "search by artist",
                detail: {
                  page: {
                    view: { view_id: "book_albums_search_artist" },
                  },
                },
              },
            ],
          },
        },
      },
      {
        id: "video_group",
        name: "Video",
        detail: {
          section: {
            children: [
              {
                name: "albums by add date",
                id: "video_albums_by_add_date",
                detail: {
                  page: {
                    view: { view_id: "video_albums_by_add_date" },
                  },
                },
              },
              {
                id: "video_albums_by_name",
                name: "albums by name",
                detail: {
                  page: {
                    view: { view_id: "video_albums_by_name" },
                  },
                },
              },
              {
                id: "video_albums_search_name",
                name: "albums, search by name",
                detail: {
                  page: {
                    view: { view_id: "video_albums_search_name" },
                  },
                },
              },
            ],
          },
        },
      },
      {
        id: "notes_group",
        name: "Notes",
        detail: {
          section: {
            children: [
              {
                id: "notes_by_add_date",
                name: "by add date",
                detail: {
                  page: { view: { view_id: "notes_by_add_date" } },
                },
              },
              {
                id: "notes_by_random",
                name: "by random",
                detail: {
                  page: { view: { view_id: "notes_by_random" } },
                },
              },
              {
                id: "notes_search_text",
                name: "search text",
                detail: {
                  page: { view: { view_id: "notes_search_text" } },
                },
              },
              {
                id: "notes_search_topic",
                name: "search topic",
                detail: {
                  page: { view: { view_id: "notes_search_topic" } },
                },
              },
              {
                id: "notes_new",
                name: "new note",
                detail: { page: { form: { form_id: "notes_new" } } },
              },
              {
                id: "notes_new_file",
                name: "new file note",
                detail: {
                  page: { form: { form_id: "notes_new_file" } },
                },
              },
            ],
          },
        },
      },
      {
        id: "history",
        name: "History",
        detail: { page: "history" },
      },
      { id: "query", name: "Query", detail: { page: "query" } },
      { id: "logs", name: "Logs", detail: { page: "logs" } },
    ],
    views: {
      audio_albums_by_add_date: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_audio_albums.txt",
            "./sunwet/source/queries/query_audio_albums_select.txt",
            {
              fields: [
                ["desc", "album_add_timestamp"],
                ["asc", "album_name"],
              ],
            }
          ),
          tracks: query_audio_albums_tracks,
        },
        display: display_audio_albums,
      },
      audio_albums_eq_album: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_audio_albums_eq_album.txt",
            "./sunwet/source/queries/query_audio_albums_select.txt"
          ),
          tracks: query_audio_albums_tracks,
        },
        parameters: { album_id: "text" },
        display: display_audio_albums_few,
      },
      audio_albums_eq_artist_by_name: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_audio_albums_eq_artist.txt",
            "./sunwet/source/queries/query_audio_albums_select.txt",
            {
              fields: [["asc", "album_name"]],
            }
          ),
          tracks: query_audio_albums_tracks,
        },
        parameters: { artist_id: "text" },
        display: display_audio_albums,
      },
      audio_albums_by_random: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_audio_albums.txt",
            "./sunwet/source/queries/query_audio_albums_select.txt",
            "shuffle"
          ),
          tracks: query_audio_albums_tracks,
        },
        display: display_audio_albums,
      },
      audio_albums_search_artists: {
        parameters: { artist: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_audio_albums_search_artist.txt",
            "./sunwet/source/queries/query_audio_albums_select.txt"
          ),
          tracks: query_audio_albums_tracks,
        },
        display: display_audio_albums,
      },
      audio_albums_search_name: {
        parameters: { name: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_audio_albums_search_name.txt",
            "./sunwet/source/queries/query_audio_albums_select.txt"
          ),
          tracks: query_audio_albums_tracks,
        },
        display: display_audio_albums,
      },
      audio_tracks_random: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_audio_tracks.txt",
            "./sunwet/source/queries/query_audio_tracks_select.txt",
            "shuffle"
          ),
        },
        display: display_audio_tracks,
      },
      audio_tracks_search_artists: {
        parameters: { artist: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_audio_tracks_search_artist.txt",
            "./sunwet/source/queries/query_audio_tracks_select.txt"
          ),
        },
        display: display_audio_tracks,
      },
      audio_tracks_search_name: {
        parameters: { name: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_audio_tracks_search_name.txt",
            "./sunwet/source/queries/query_audio_tracks_select.txt"
          ),
        },
        display: display_audio_tracks,
      },
      video_albums_by_add_date: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_video_albums.txt",
            "./sunwet/source/queries/query_video_albums_select.txt",
            {
              fields: [
                ["desc", "album_add_timestamp"],
                ["asc", "album_name"],
              ],
            }
          ),
          tracks: query_video_albums_tracks,
        },
        display: display_video_albums,
      },
      video_albums_eq_album: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_video_albums_eq_album.txt",
            "./sunwet/source/queries/query_video_albums_select.txt"
          ),
          tracks: query_video_albums_tracks,
        },
        display: display_video_albums,
      },
      video_albums_by_name: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_video_albums.txt",
            "./sunwet/source/queries/query_video_albums_select.txt",
            {
              fields: [["asc", "album_name"]],
            }
          ),
          tracks: query_video_albums_tracks,
        },
        display: display_video_albums,
      },
      video_albums_search_name: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_video_albums_search_name.txt",
            "./sunwet/source/queries/query_video_albums_select.txt"
          ),
          tracks: query_video_albums_tracks,
        },
        display: display_video_albums,
      },
      comic_albums_by_name: {
        parameters: { lang: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_comic_albums.txt",
            "./sunwet/source/queries/query_comic_albums_select.txt",
            {
              fields: [["asc", "album_name"]],
            }
          ),
          tracks: query_comic_albums_tracks,
        },
        display: display_comic_albums,
      },
      comic_albums_eq_album: {
        parameters: { album_id: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_comic_albums_eq_album.txt",
            "./sunwet/source/queries/query_comic_albums_select.txt"
          ),
          tracks: query_comic_albums_tracks,
        },
        display: display_comic_albums,
      },
      comic_albums_eq_artist_by_name: {
        parameters: { artist_id: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_comic_albums_eq_artist.txt",
            "./sunwet/source/queries/query_comic_albums_select.txt",
            {
              fields: [["asc", "album_name"]],
            }
          ),
          tracks: query_comic_albums_tracks,
        },
        display: display_comic_albums,
      },
      comic_albums_search_name: {
        parameters: { lang: "text", name: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_comic_albums_search_name.txt",
            "./sunwet/source/queries/query_comic_albums_select.txt"
          ),
          tracks: query_comic_albums_tracks,
        },
        display: display_comic_albums,
      },
      comic_albums_search_artists: {
        parameters: { lang: "text", artist: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_comic_albums_search_artist.txt",
            "./sunwet/source/queries/query_comic_albums_select.txt"
          ),
          tracks: query_comic_albums_tracks,
        },
        display: display_comic_albums,
      },
      book_albums_by_name: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_book_albums.txt",
            "./sunwet/source/queries/query_book_albums_select.txt",
            {
              fields: [["asc", "album_name"]],
            }
          ),
          tracks: query_book_albums_tracks,
        },
        display: display_book_albums,
      },
      book_albums_eq_album: {
        parameters: { album_id: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_book_albums_eq_album.txt",
            "./sunwet/source/queries/query_book_albums_select.txt"
          ),
          tracks: query_book_albums_tracks,
        },
        display: display_book_albums,
      },
      book_albums_eq_artist_by_name: {
        parameters: { artist_id: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_book_albums_eq_artist.txt",
            "./sunwet/source/queries/query_book_albums_select.txt",
            {
              fields: [["asc", "album_name"]],
            }
          ),
          tracks: query_book_albums_tracks,
        },
        display: display_book_albums,
      },
      book_albums_search_name: {
        parameters: { name: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_book_albums_search_name.txt",
            "./sunwet/source/queries/query_book_albums_select.txt"
          ),
          tracks: query_book_albums_tracks,
        },
        display: display_book_albums,
      },
      book_albums_search_artist: {
        parameters: { artist: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_book_albums_search_artist.txt",
            "./sunwet/source/queries/query_book_albums_select.txt"
          ),
          tracks: query_book_albums_tracks,
        },
        display: display_book_albums,
      },
      notes_by_add_date: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_notes.txt",
            "./sunwet/source/queries/query_notes_select.txt",
            {
              fields: [
                ["desc", "album_add_timestamp"],
                ["asc", "album_name"],
              ],
            }
          ),
        },
        display: display_notes,
      },
      notes_by_random: {
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_notes.txt",
            "./sunwet/source/queries/query_notes_select.txt",
            "shuffle"
          ),
        },
        display: display_notes,
      },
      notes_search_text: {
        parameters: { text: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_notes_search_text.txt",
            "./sunwet/source/queries/query_notes_select.txt"
          ),
        },
        display: display_notes,
      },
      notes_search_topic: {
        parameters: { text: "text" },
        queries: {
          root: await compile_query_head_tail(
            "./sunwet/source/queries/query_notes_search_topic.txt",
            "./sunwet/source/queries/query_notes_select.txt"
          ),
        },
        display: display_notes,
      },
    },
    forms: {
      notes_new: {
        fields: [
          { id: "id", label: "", type: "id" },
          { id: "stamp", label: "Date", type: "datetime" },
          { id: "topic", label: "Topic", type: { text: {} } },
          { id: "text", label: "Note", type: { text: {} } },
        ],
        outputs: [
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/is" },
            object: { inline: { t: "v", v: "sunwet/1/note" } },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/add_timestamp" },
            object: { input: "stamp" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/text" },
            object: { input: "text" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/topic" },
            object: { input: "topic" },
          },
        ],
      },
      notes_new_file: {
        fields: [
          { id: "id", label: "", type: "id" },
          { id: "stamp", label: "Date", type: "datetime" },
          { id: "topic", label: "Topic", type: { text: {} } },
          { id: "text", label: "Note", type: { text: {} } },
          { id: "file", label: "File", type: "file" },
        ],
        outputs: [
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/is" },
            object: { inline: { t: "v", v: "sunwet/1/note" } },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/add_timestamp" },
            object: { input: "stamp" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/text" },
            object: { input: "text" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/topic" },
            object: { input: "topic" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/file" },
            object: { input: "file" },
          },
        ],
      },
    },
  };
  const config = {
    user: userConfig,
    sunwet: sunwet_config,
  };

  let res = await fetch(process.env.SUNWET_URL, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${process.env.SUNWET_TOKEN}`,
    },
    body: JSON.stringify(config),
  });
  if (res.status >= 300) {
    throw new Error(`Failed [${res.status}]:\n${await res.text()}`);
  }
})();
