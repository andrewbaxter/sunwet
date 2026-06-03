import * as sunwet from "./generated/ts/index.ts";
import * as sortquery from "./generated/ts/sub/SortQuery.ts";
import * as child_process from "child_process";
import * as process from "process";
const dirname = import.meta.dirname;

export const BROWSEREXT_VIEW_MICROBLOG_EXISTS = "microblog-exists";
export const BROWSEREXT_VIEW_PROFILE_EXISTS = "profile-exists";
export const BROWSEREXT_VIEW_IMAGE_EXISTS = "image-exists";
export const BROWSEREXT_FORM_MICROBLOG = "capture-microblog";
export const BROWSEREXT_FORM_PROFILE = "capture-profile";
export const BROWSEREXT_FORM_IMAGE = "capture-image";

export const buildGlobal = async (apiTokens: {
  [x: string]: sunwet.ConfigIamGrants;
}): Promise<sunwet.GlobalConfig> => {
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
              `[${cmd}] [${args.join(", ")}] exited with non-zero code: ${code}`,
            ),
          );
        }
      });
    });
  };
  const compileQueryHeadTail = async (
    headPath: string,
    tailPath: string,
    sort?: sortquery.SortQuery,
  ): Promise<sunwet.Query> => {
    const head: sunwet.Query = JSON.parse(
      await run_output("sunwet", ["compile-query", "--file", headPath]),
    );
    if (head.suffix != null) {
      throw new Error();
    }
    const tail: sunwet.Query = JSON.parse(
      await run_output("sunwet", ["compile-query", "--file", tailPath]),
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
  const compileQuery = async (path: string): Promise<sunwet.Query> => {
    return JSON.parse(
      await run_output("sunwet", ["compile-query", "--file", path]),
    );
  };

  const widgetNodeLink = (
    nameField: string,
    nodeField: string,
  ): sunwet.Widget => {
    return {
      node: {
        name: { field: nameField },
        node: { field: nodeField },
        orientation: "right_down",
      },
    };
  };

  // import * as fdap_login from "./fdap-login/source/generated/ts/index";
  const audioAlbumTitleBlockSize = "6cm";
  const albumTitleBlockHeight = "8.4cm";
  const albumTracksHeight = "min(max-content, 100dvh)";
  const displayAudioAlbums: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_width: audioAlbumTitleBlockSize,
    element_height: albumTitleBlockHeight,
    element_body: {
      layout: {
        trans_align: "end",
        orientation: "down_right",
        elements: [
          {
            media: {
              orientation: "down_right",
              trans_align: "middle",
              width: audioAlbumTitleBlockSize,
              aspect: "1",
              data: { field: "cover" },
            },
          },
          {
            text: {
              trans_align: "start",
              font_size: "18pt",
              con_size_mode: "ellipsize",
              con_size_max: "100%",
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
              con_size_mode: "ellipsize",
              con_size_max: "100%",
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
    element_expansion: {
      layout: {
        orientation: "right_down",
        elements: [
          "space",
          {
            data_rows: {
              data: { query: "tracks" },
              row_widget: {
                table: {
                  orientation: "right_down",
                  row_trans_direction_downright: false,
                  con_scroll: true,
                  row_gap: "0.2cm",
                  trans_size_max: albumTracksHeight,
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
                        con_size_mode: "wrap",
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
                        con_size_mode: "wrap",
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
                        con_size_mode: "wrap",
                        orientation: "down_left",
                      },
                    },
                  ],
                },
              },
            },
          },
          "space",
        ],
      },
    },
  };

  const displayAudioAlbumsFew: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_body: {
      layout: {
        trans_align: "start",
        orientation: "right_down",
        elements: [
          {
            media: {
              orientation: "down_right",
              trans_align: "start",
              width: "6cm",
              data: { field: "cover" },
            },
          },
          {
            layout: {
              trans_align: "start",
              orientation: "down_right",
              elements: [
                {
                  layout: {
                    trans_align: "start",
                    orientation: "right_down",
                    elements: [
                      {
                        text: {
                          trans_align: "start",
                          font_size: "18pt",
                          con_size_mode: "ellipsize",
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
                      widgetNodeLink("album_name", "album_id"),
                    ],
                  },
                },
                {
                  text: {
                    trans_align: "start",
                    font_size: "12pt",
                    con_size_mode: "ellipsize",
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
                        orientation: "right_down",
                        row_trans_direction_downright: false,
                        con_scroll: true,
                        row_gap: "0.2cm",
                        elements: [
                          {
                            play_button: {
                              trans_align: "middle",
                              orientation: "down_right",
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
                              con_size_mode: "wrap",
                              orientation: "down_right",
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
                              con_size_mode: "wrap",
                              orientation: "down_right",
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
                              con_size_mode: "wrap",
                              con_size_max: "8cm",
                              orientation: "down_right",
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
        ],
      },
    },
  };

  const audioTracksBlockSize = "min(15dvw, 1.6cm)";
  const displayAudioTracks: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_body: {
      layout: {
        orientation: "down_right",
        elements: [
          {
            layout: {
              orientation: "right_down",
              elements: [
                {
                  play_button: {
                    media_file_field: "file",
                    show_image: true,
                    width: audioTracksBlockSize,
                    height: audioTracksBlockSize,
                    name_field: "track_name",
                    album_field: "album_name",
                    artist_field: "artist_name",
                    cover_field: "cover",
                    trans_align: "middle",
                  },
                },
                {
                  media: {
                    orientation: "right_down",
                    trans_align: "middle",
                    width: audioTracksBlockSize,
                    height: audioTracksBlockSize,
                    data: { field: "cover" },
                  },
                },
                {
                  layout: {
                    orientation: "down_right",
                    gap: "0.2cm",
                    trans_align: "middle",
                    elements: [
                      "space",
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
                          orientation: "right_down",
                          gap: "0",
                          con_wrap: true,
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
  };
  const displayVideoAlbums: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_body: {
      layout: {
        orientation: "right_down",
        trans_align: "start",
        elements: [
          {
            media: {
              orientation: "down_right",
              trans_align: "start",
              width: "6cm",
              data: { field: "cover" },
            },
          },
          {
            layout: {
              trans_align: "start",
              orientation: "down_right",
              elements: [
                {
                  layout: {
                    trans_align: "start",
                    orientation: "right_down",
                    elements: [
                      {
                        text: {
                          trans_align: "start",
                          font_size: "18pt",
                          con_size_mode: "ellipsize",
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
                      widgetNodeLink("album_name", "album_id"),
                    ],
                  },
                },
                {
                  data_rows: {
                    data: { query: "tracks" },
                    row_widget: {
                      table: {
                        orientation: "right_down",
                        row_trans_direction_downright: false,
                        con_scroll: true,
                        row_gap: "0.2cm",
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
                              con_size_mode: "wrap",
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
                              con_size_mode: "wrap",
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
                              con_size_mode: "wrap",
                              con_size_max: "8cm",
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
        ],
      },
    },
  };
  const displayComicAlbums: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_body: {
      layout: {
        trans_align: "start",
        orientation: "down_right",
        elements: [
          {
            layout: {
              trans_align: "start",
              orientation: "right_down",
              elements: [
                {
                  text: {
                    trans_align: "start",
                    font_size: "18pt",
                    con_size_mode: "ellipsize",
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
                widgetNodeLink("album_name", "album_id"),
              ],
            },
          },
          {
            text: {
              trans_align: "start",
              font_size: "12pt",
              con_size_mode: "ellipsize",
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
                  con_scroll: true,
                  orientation: "right_down",
                  widget: {
                    layout: {
                      orientation: "down_right",
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
                            orientation: "right_down",
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
                                  link: {
                                    title: {
                                      field: "track_index",
                                    },
                                    dest: {
                                      node: {
                                        field: "track_id",
                                      },
                                    },
                                  },
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
  };
  const displayBookAlbums: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_body: {
      layout: {
        trans_align: "start",
        orientation: "down_right",
        elements: [
          {
            layout: {
              trans_align: "start",
              orientation: "right_down",
              elements: [
                {
                  text: {
                    trans_align: "start",
                    font_size: "18pt",
                    con_size_mode: "ellipsize",
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
                widgetNodeLink("album_name", "album_id"),
              ],
            },
          },
          {
            text: {
              trans_align: "start",
              font_size: "12pt",
              con_size_mode: "ellipsize",
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
                  con_scroll: true,
                  orientation: "right_down",
                  widget: {
                    layout: {
                      orientation: "down_right",
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
                            orientation: "right_down",
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
                                  link: {
                                    title: {
                                      field: "track_index",
                                    },
                                    dest: {
                                      node: {
                                        field: "track_id",
                                      },
                                    },
                                  },
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
                                  link: {
                                    title: {
                                      field: "track_index",
                                    },
                                    dest: {
                                      node: {
                                        field: "track_id",
                                      },
                                    },
                                  },
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
  };
  const displayNotes: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_body: {
      layout: {
        orientation: "down_right",
        gap: "0.1cm",
        elements: [
          {
            layout: {
              orientation: "right_down",
              elements: [
                {
                  text: {
                    trans_align: "start",
                    font_size: "12pt",
                    color: "rgba(78, 94, 119, 0.8)",
                    con_size_mode: "wrap",
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
                widgetNodeLink("note_id", "note_id"),
              ],
            },
          },
          {
            text: {
              trans_align: "start",
              font_size: "12pt",
              con_size_mode: "wrap",
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
  };
  const mediaBlockSize = "8cm";
  const displayMicroblogMedia: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_width: mediaBlockSize,
    element_height: mediaBlockSize,
    element_body: {
      media: {
        data: { field: "file" },
        trans_align: "middle",
      },
    },
    element_expansion: {
      layout: {
        orientation: "right_down",
        elements: [
          "space",
          {
            layout: {
              orientation: "down_right",
              trans_size_max: "20cm",
              trans_align: "middle",
              gap: "0.3cm",
              elements: [
                {
                  table: {
                    columns: 2,
                    orientation: "down_right",
                    column_gap: "0.5cm",
                    row_gap: "0.2cm",
                    row_trans_direction_downright: true,
                    elements: [
                      // 1st
                      "space",
                      {
                        layout: {
                          orientation: "right_down",
                          trans_align: "end",
                          elements: [
                            "space",
                            widgetNodeLink("microblog_id", "microblog_id"),
                            {
                              play_button: {
                                media_file_field: "file",
                                orientation: "right_down",
                                name_field: "author",
                                album_field: "url",
                                artist_field: "author",
                                cover_field: "file",
                              },
                            },
                          ],
                        },
                      },
                      // 2nd row
                      {
                        text: {
                          data: { literal: "Author" },
                          orientation: "right_down",
                        },
                      },
                      {
                        text: {
                          data: { field: "author" },
                          orientation: "right_down",
                        },
                      },
                      // 3rd
                      {
                        text: {
                          data: { literal: "Date" },
                          orientation: "right_down",
                        },
                      },
                      {
                        date: {
                          data: { field: "create_timestamp" },
                          orientation: "right_down",
                        },
                      },
                      // 4th
                      {
                        text: {
                          data: { literal: "URL" },
                          orientation: "right_down",
                        },
                      },
                      {
                        text: {
                          data: { field: "url" },
                          con_size_mode: "wrap",
                          orientation: "right_down",
                          link: {
                            title: { field: "url" },
                            dest: { plain: { field: "url" } },
                          },
                        },
                      },
                    ],
                  },
                },
                {
                  text: {
                    data: { field: "text" },
                    con_size_mode: "wrap",
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
  };
  const displayImages: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_width: mediaBlockSize,
    element_height: mediaBlockSize,
    element_body: {
      media: {
        data: { field: "file" },
        trans_align: "middle",
      },
    },
    element_expansion: {
      layout: {
        orientation: "right_down",
        elements: [
          "space",
          {
            layout: {
              orientation: "down_right",
              trans_size_max: "20cm",
              trans_align: "middle",
              gap: "0.3cm",
              elements: [
                {
                  table: {
                    columns: 2,
                    orientation: "down_right",
                    column_gap: "0.5cm",
                    row_gap: "0.2cm",
                    row_trans_direction_downright: true,
                    elements: [
                      // 1st
                      "space",
                      {
                        layout: {
                          orientation: "right_down",
                          trans_align: "end",
                          elements: [
                            "space",
                            widgetNodeLink("image_id", "image_id"),
                            {
                              play_button: {
                                media_file_field: "file",
                                orientation: "right_down",
                                name_field: "artist_name",
                                album_field: "url",
                                artist_field: "artist_name",
                                cover_field: "file",
                              },
                            },
                          ],
                        },
                      },
                      // 2nd row
                      {
                        text: {
                          data: { literal: "Artist" },
                          orientation: "right_down",
                        },
                      },
                      {
                        text: {
                          data: { field: "artist_name" },
                          orientation: "right_down",
                        },
                      },
                      // 3rd
                      {
                        text: {
                          data: { literal: "URL" },
                          orientation: "right_down",
                        },
                      },
                      {
                        text: {
                          data: { field: "url" },
                          con_size_mode: "wrap",
                          orientation: "right_down",
                          link: {
                            title: { field: "url" },
                            dest: { plain: { field: "url" } },
                          },
                        },
                      },
                      // 4th
                      {
                        text: {
                          data: { literal: "Source URL" },
                          orientation: "right_down",
                        },
                      },
                      {
                        text: {
                          data: { field: "source_url" },
                          con_size_mode: "wrap",
                          orientation: "right_down",
                          link: {
                            title: { field: "source_url" },
                            dest: { plain: { field: "source_url" } },
                          },
                        },
                      },
                      // 5th
                      {
                        text: {
                          data: { literal: "Artist URL" },
                          orientation: "right_down",
                        },
                      },
                      {
                        text: {
                          data: { field: "artist_url" },
                          con_size_mode: "wrap",
                          orientation: "right_down",
                          link: {
                            title: { field: "artist_url" },
                            dest: { plain: { field: "artist_url" } },
                          },
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
          "space",
        ],
      },
    },
  };
  const displayPlaylists: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_width: audioAlbumTitleBlockSize,
    element_height: albumTitleBlockHeight,
    element_body: {
      layout: {
        trans_align: "end",
        orientation: "down_right",
        elements: [
          {
            media: {
              orientation: "down_right",
              trans_align: "middle",
              width: audioAlbumTitleBlockSize,
              aspect: "1",
              data: { field: "cover" },
            },
          },
          {
            text: {
              trans_align: "start",
              font_size: "18pt",
              con_size_mode: "ellipsize",
              orientation: "right_down",
              data: { field: "playlist_name" },
              link: {
                title: {
                  field: "playlist_name",
                },
                dest: {
                  view: {
                    id: "audio_playlists_eq_playlist",
                    parameters: {
                      playlist_id: {
                        field: "playlist_id",
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
              con_size_mode: "ellipsize",
              orientation: "right_down",
              data: { field: "playlist_artist_name" },
              link: {
                title: {
                  field: "playlist_artist_name",
                },
                dest: {
                  view: {
                    id: "audio_playlists_eq_artist_by_name",
                    parameters: {
                      artist_id: {
                        field: "playlist_artist_id",
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
    element_expansion: {
      data_rows: {
        data: { query: "tracks" },
        row_widget: {
          table: {
            orientation: "right_down",
            row_trans_direction_downright: false,
            con_scroll: true,
            row_gap: "0.2cm",
            trans_size_max: albumTracksHeight,
            elements: [
              {
                play_button: {
                  trans_align: "middle",
                  orientation: "down_left",
                  media_file_field: "file",
                  name_field: "track_name",
                  album_field: "playlist_name",
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
                  con_size_mode: "wrap",
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
                  con_size_mode: "wrap",
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
                  con_size_mode: "wrap",
                  orientation: "down_left",
                },
              },
            ],
          },
        },
      },
    },
  };
  const queryAudioAlbumsTracks = await compileQuery(
    `${dirname}/queries/query_audio_albums_tracks.txt`,
  );
  const queryVideoAlbumsTracks = await compileQuery(
    `${dirname}/queries/query_video_albums_tracks.txt`,
  );
  const queryComicAlbumsTracks = await compileQuery(
    `${dirname}/queries/query_comic_albums_tracks.txt`,
  );
  const queryBookAlbumsTracks = await compileQuery(
    `${dirname}/queries/query_book_albums_tracks.txt`,
  );
  const queryPlaylistsTracks = await compileQuery(
    `${dirname}/queries/query_playlists_tracks.txt`,
  );

  // Microblog and image view queries
  const queryMicroblogHead = `${dirname}/queries/query_microblog.txt`;
  const queryMicroblogSelect = `${dirname}/queries/query_microblog_select.txt`;
  const queryImageHead = `${dirname}/queries/query_image.txt`;
  const queryImageSelect = `${dirname}/queries/query_image_select.txt`;

  // Browser extension: existence check queries
  const queryMicroblogExists = await compileQuery(
    `${dirname}/queries/query_microblog_exists.txt`,
  );
  const queryProfileExists = await compileQuery(
    `${dirname}/queries/query_profile_exists.txt`,
  );
  const queryImageExists = await compileQuery(
    `${dirname}/queries/query_image_exists.txt`,
  );

  // Browser extension: minimal display for existence-check-only views
  const existsDisplay: sunwet.WidgetRootDataRows = {
    data: { query: "root" },
    element_body: {
      text: {
        data: { field: "id" },
        orientation: "right_down",
      },
    },
  };

  return {
    api_tokens: { [process.env.SUNWET_TOKEN]: "admin", ...apiTokens },
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
        id: "playlists_group",
        name: "Playlists",
        detail: {
          section: {
            children: [
              {
                name: "playlists by add date",
                id: "playlists_by_add_date",
                detail: {
                  page: {
                    view: { view_id: "playlists_by_add_date" },
                  },
                },
              },
              {
                id: "playlists_by_random",
                name: "playlists by random",
                detail: {
                  page: {
                    view: { view_id: "playlists_by_random" },
                  },
                },
              },
              {
                id: "playlists_search_name",
                name: "playlists, search by name",
                detail: {
                  page: {
                    view: { view_id: "playlists_search_name" },
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
        id: "microblog_group",
        name: "Microblog",
        detail: {
          section: {
            children: [
              {
                id: "microblog_media_by_add_date",
                name: "media by add date",
                detail: {
                  page: {
                    view: { view_id: "microblog_media_by_add_date" },
                  },
                },
              },
              {
                id: "microblog_media_by_create_date",
                name: "media by created date",
                detail: {
                  page: {
                    view: { view_id: "microblog_media_by_create_date" },
                  },
                },
              },
              {
                id: "microblog_media_by_random",
                name: "media by random",
                detail: {
                  page: {
                    view: { view_id: "microblog_media_by_random" },
                  },
                },
              },
            ],
          },
        },
      },
      {
        id: "images_group",
        name: "Images",
        detail: {
          section: {
            children: [
              {
                id: "images_by_add_date",
                name: "by add date",
                detail: {
                  page: {
                    view: { view_id: "images_by_add_date" },
                  },
                },
              },
              {
                id: "images_by_random",
                name: "by random",
                detail: {
                  page: {
                    view: { view_id: "images_by_random" },
                  },
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
    ],
    views: {
      audio_albums_by_add_date: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_audio_albums.txt`,
            `${dirname}/queries/query_audio_albums_select.txt`,
            {
              fields: [
                ["desc", "album_add_timestamp"],
                ["asc", "album_name"],
              ],
            },
          ),
          tracks: queryAudioAlbumsTracks,
        },
        display: displayAudioAlbums,
      },
      audio_albums_eq_album: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_audio_albums_eq_album.txt`,
            `${dirname}/queries/query_audio_albums_select.txt`,
          ),
          tracks: queryAudioAlbumsTracks,
        },
        parameters: { album_id: "text" },
        display: displayAudioAlbumsFew,
      },
      audio_albums_eq_artist_by_name: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_audio_albums_eq_artist.txt`,
            `${dirname}/queries/query_audio_albums_select.txt`,
            {
              fields: [["asc", "album_name"]],
            },
          ),
          tracks: queryAudioAlbumsTracks,
        },
        parameters: { artist_id: "text" },
        display: displayAudioAlbums,
      },
      audio_albums_by_random: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_audio_albums.txt`,
            `${dirname}/queries/query_audio_albums_select.txt`,
            "shuffle",
          ),
          tracks: queryAudioAlbumsTracks,
        },
        display: displayAudioAlbums,
      },
      audio_albums_search_artists: {
        parameters: { artist: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_audio_albums_search_artist.txt`,
            `${dirname}/queries/query_audio_albums_select.txt`,
          ),
          tracks: queryAudioAlbumsTracks,
        },
        display: displayAudioAlbums,
      },
      audio_albums_search_name: {
        parameters: { name: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_audio_albums_search_name.txt`,
            `${dirname}/queries/query_audio_albums_select.txt`,
          ),
          tracks: queryAudioAlbumsTracks,
        },
        display: displayAudioAlbums,
      },
      audio_tracks_random: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_audio_tracks.txt`,
            `${dirname}/queries/query_audio_tracks_select.txt`,
            "shuffle",
          ),
        },
        display: displayAudioTracks,
      },
      audio_tracks_search_artists: {
        parameters: { artist: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_audio_tracks_search_artist.txt`,
            `${dirname}/queries/query_audio_tracks_select.txt`,
          ),
        },
        display: displayAudioTracks,
      },
      audio_tracks_search_name: {
        parameters: { name: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_audio_tracks_search_name.txt`,
            `${dirname}/queries/query_audio_tracks_select.txt`,
          ),
        },
        display: displayAudioTracks,
      },
      video_albums_by_add_date: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_video_albums.txt`,
            `${dirname}/queries/query_video_albums_select.txt`,
            {
              fields: [
                ["desc", "album_add_timestamp"],
                ["asc", "album_name"],
              ],
            },
          ),
          tracks: queryVideoAlbumsTracks,
        },
        display: displayVideoAlbums,
      },
      video_albums_eq_album: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_video_albums_eq_album.txt`,
            `${dirname}/queries/query_video_albums_select.txt`,
          ),
          tracks: queryVideoAlbumsTracks,
        },
        display: displayVideoAlbums,
      },
      video_albums_by_name: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_video_albums.txt`,
            `${dirname}/queries/query_video_albums_select.txt`,
            {
              fields: [["asc", "album_name"]],
            },
          ),
          tracks: queryVideoAlbumsTracks,
        },
        display: displayVideoAlbums,
      },
      video_albums_search_name: {
        parameters: { name: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_video_albums_search_name.txt`,
            `${dirname}/queries/query_video_albums_select.txt`,
          ),
          tracks: queryVideoAlbumsTracks,
        },
        display: displayVideoAlbums,
      },
      comic_albums_by_name: {
        parameters: { lang: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_comic_albums.txt`,
            `${dirname}/queries/query_comic_albums_select.txt`,
            {
              fields: [["asc", "album_name"]],
            },
          ),
          tracks: queryComicAlbumsTracks,
        },
        display: displayComicAlbums,
      },
      comic_albums_eq_album: {
        parameters: { album_id: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_comic_albums_eq_album.txt`,
            `${dirname}/queries/query_comic_albums_select.txt`,
          ),
          tracks: queryComicAlbumsTracks,
        },
        display: displayComicAlbums,
      },
      comic_albums_eq_artist_by_name: {
        parameters: { artist_id: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_comic_albums_eq_artist.txt`,
            `${dirname}/queries/query_comic_albums_select.txt`,
            {
              fields: [["asc", "album_name"]],
            },
          ),
          tracks: queryComicAlbumsTracks,
        },
        display: displayComicAlbums,
      },
      comic_albums_search_name: {
        parameters: { lang: "text", name: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_comic_albums_search_name.txt`,
            `${dirname}/queries/query_comic_albums_select.txt`,
          ),
          tracks: queryComicAlbumsTracks,
        },
        display: displayComicAlbums,
      },
      comic_albums_search_artists: {
        parameters: { lang: "text", artist: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_comic_albums_search_artist.txt`,
            `${dirname}/queries/query_comic_albums_select.txt`,
          ),
          tracks: queryComicAlbumsTracks,
        },
        display: displayComicAlbums,
      },
      book_albums_by_name: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_book_albums.txt`,
            `${dirname}/queries/query_book_albums_select.txt`,
            {
              fields: [["asc", "album_name"]],
            },
          ),
          tracks: queryBookAlbumsTracks,
        },
        display: displayBookAlbums,
      },
      book_albums_eq_album: {
        parameters: { album_id: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_book_albums_eq_album.txt`,
            `${dirname}/queries/query_book_albums_select.txt`,
          ),
          tracks: queryBookAlbumsTracks,
        },
        display: displayBookAlbums,
      },
      book_albums_eq_artist_by_name: {
        parameters: { artist_id: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_book_albums_eq_artist.txt`,
            `${dirname}/queries/query_book_albums_select.txt`,
            {
              fields: [["asc", "album_name"]],
            },
          ),
          tracks: queryBookAlbumsTracks,
        },
        display: displayBookAlbums,
      },
      book_albums_search_name: {
        parameters: { name: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_book_albums_search_name.txt`,
            `${dirname}/queries/query_book_albums_select.txt`,
          ),
          tracks: queryBookAlbumsTracks,
        },
        display: displayBookAlbums,
      },
      book_albums_search_artist: {
        parameters: { artist: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_book_albums_search_artist.txt`,
            `${dirname}/queries/query_book_albums_select.txt`,
          ),
          tracks: queryBookAlbumsTracks,
        },
        display: displayBookAlbums,
      },
      notes_by_add_date: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_notes.txt`,
            `${dirname}/queries/query_notes_select.txt`,
            {
              fields: [["desc", "add_timestamp"]],
            },
          ),
        },
        display: displayNotes,
      },
      notes_by_random: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_notes.txt`,
            `${dirname}/queries/query_notes_select.txt`,
            "shuffle",
          ),
        },
        display: displayNotes,
      },
      notes_search_text: {
        parameters: { text: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_notes_search_text.txt`,
            `${dirname}/queries/query_notes_select.txt`,
          ),
        },
        display: displayNotes,
      },
      notes_search_topic: {
        parameters: { text: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_notes_search_topic.txt`,
            `${dirname}/queries/query_notes_select.txt`,
          ),
        },
        display: displayNotes,
      },
      playlists_by_add_date: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_playlists.txt`,
            `${dirname}/queries/query_playlists_select.txt`,
            {
              fields: [
                ["desc", "playlist_add_timestamp"],
                ["asc", "playlist_name"],
              ],
            },
          ),
          tracks: queryPlaylistsTracks,
        },
        display: displayPlaylists,
      },
      playlists_by_random: {
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_playlists.txt`,
            `${dirname}/queries/query_playlists_select.txt`,
            "shuffle",
          ),
          tracks: queryPlaylistsTracks,
        },
        display: displayPlaylists,
      },
      playlists_search_name: {
        parameters: { name: "text" },
        queries: {
          root: await compileQueryHeadTail(
            `${dirname}/queries/query_playlists_search_name.txt`,
            `${dirname}/queries/query_playlists_select.txt`,
          ),
          tracks: queryPlaylistsTracks,
        },
        display: displayPlaylists,
      },

      // Microblog media views
      microblog_media_by_add_date: {
        queries: {
          root: await compileQueryHeadTail(
            queryMicroblogHead,
            queryMicroblogSelect,
            {
              fields: [["desc", "add_timestamp"]],
            },
          ),
        },
        track_end_mode: "loop",
        display: displayMicroblogMedia,
      },
      microblog_media_by_create_date: {
        queries: {
          root: await compileQueryHeadTail(
            queryMicroblogHead,
            queryMicroblogSelect,
            {
              fields: [["desc", "create_timestamp"]],
            },
          ),
        },
        track_end_mode: "loop",
        display: displayMicroblogMedia,
      },
      microblog_media_by_random: {
        queries: {
          root: await compileQueryHeadTail(
            queryMicroblogHead,
            queryMicroblogSelect,
            "shuffle",
          ),
        },
        track_end_mode: "loop",
        display: displayMicroblogMedia,
      },

      // Image views
      images_by_add_date: {
        queries: {
          root: await compileQueryHeadTail(queryImageHead, queryImageSelect, {
            fields: [["desc", "add_timestamp"]],
          }),
        },
        track_end_mode: "loop",
        display: displayImages,
      },
      images_by_random: {
        queries: {
          root: await compileQueryHeadTail(
            queryImageHead,
            queryImageSelect,
            "shuffle",
          ),
        },
        track_end_mode: "loop",
        display: displayImages,
      },

      // Browser extension: existence check views
      [BROWSEREXT_VIEW_MICROBLOG_EXISTS]: {
        parameters: { id: "text" },
        queries: { root: queryMicroblogExists },
        display: existsDisplay,
      },
      [BROWSEREXT_VIEW_PROFILE_EXISTS]: {
        parameters: { id: "text" },
        queries: { root: queryProfileExists },
        display: existsDisplay,
      },
      [BROWSEREXT_VIEW_IMAGE_EXISTS]: {
        parameters: { id: "text" },
        queries: { root: queryImageExists },
        display: existsDisplay,
      },
    },
    forms: {
      notes_new: {
        fields: [
          { id: "id", label: "", type: "id" },
          { id: "stamp", label: "", type: "datetime_now" },
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
      playlists_new: {
        fields: [
          { id: "id", label: "", type: "id" },
          { id: "name", label: "Name", type: { text: {} } },
        ],
        outputs: [
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/is" },
            object: { inline: { t: "v", v: "sunwet/1/playlist" } },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/add_timestamp" },
            object: { input: "stamp" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/name" },
            object: { input: "name" },
          },
        ],
      },

      // Browser extension: capture forms
      [BROWSEREXT_FORM_MICROBLOG]: {
        fields: [
          { id: "id", label: "", type: "id" },
          { id: "stamp", label: "", type: "datetime_now" },
          { id: "url", label: "URL", type: { text: {} } },
          { id: "author", label: "Author", type: { text: {} } },
          { id: "text", label: "Text", type: { text: {} } },
          {
            id: "create_timestamp",
            label: "Create timestamp",
            type: { text: {} },
          },
          { id: "file", label: "File", type: { text: {} } },
        ],
        outputs: [
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/is" },
            object: { inline: { t: "v", v: "sunwet/1/microblog" } },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/add_timestamp" },
            object: { input: "stamp" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/url" },
            object: { input: "url" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/author" },
            object: { input: "author" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/text" },
            object: { input: "text" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/create_timestamp" },
            object: { input: "create_timestamp" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/file" },
            object: { input: "file" },
          },
        ],
      },
      [BROWSEREXT_FORM_PROFILE]: {
        fields: [
          { id: "id", label: "", type: "id" },
          { id: "stamp", label: "", type: "datetime_now" },
          { id: "url", label: "URL", type: { text: {} } },
          { id: "name", label: "Name", type: { text: {} } },
          { id: "handle", label: "Handle", type: { text: {} } },
          { id: "description", label: "Description", type: { text: {} } },
          { id: "images", label: "Images", type: { text: {} } },
        ],
        outputs: [
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/is" },
            object: { inline: { t: "v", v: "sunwet/1/profile" } },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/add_timestamp" },
            object: { input: "stamp" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/url" },
            object: { input: "url" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/name" },
            object: { input: "name" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/handle" },
            object: { input: "handle" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/description" },
            object: { input: "description" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/images" },
            object: { input: "images" },
          },
        ],
      },
      [BROWSEREXT_FORM_IMAGE]: {
        fields: [
          { id: "id", label: "", type: "id" },
          { id: "stamp", label: "", type: "datetime_now" },
          { id: "url", label: "URL", type: { text: {} } },
          { id: "source_url", label: "Source URL", type: { text: {} } },
          { id: "artist_name", label: "Artist name", type: { text: {} } },
          { id: "artist_url", label: "Artist URL", type: { text: {} } },
          { id: "file", label: "File", type: { text: {} } },
        ],
        outputs: [
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/is" },
            object: { inline: { t: "v", v: "sunwet/1/image" } },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/add_timestamp" },
            object: { input: "stamp" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/url" },
            object: { input: "url" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/source_url" },
            object: { input: "source_url" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/artist_name" },
            object: { input: "artist_name" },
          },
          {
            subject: { input: "id" },
            predicate: { inline: "sunwet/1/artist_url" },
            object: { input: "artist_url" },
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
};
