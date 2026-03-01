import * as process from "process";
import * as default_config from "./default_config.ts";
import * as fdap from "./fdap.ts";

(async () => {
  const globalConfig = await default_config.buildGlobal();
  await fdap.sendFdap(globalConfig, {
    you: {
      "fdap-login": { password: process.env.YOUR_PASSWORD },
      sunwet: { iam_grants: "admin" },
    },
    youmobile: {
      "fdap-login": { password: process.env.YOUR_MOBILE_PASSWORD },
      sunwet: {
        iam_grants: {
          limited: {
            menu_items: [
              "audio_group",
              "comics_group",
              "books_group",
              "notes_group",
              "playlists_group",
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
  });
})();
