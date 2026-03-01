import fs from "node:fs";
import * as sunwet from "./generated/ts/index.ts";
import * as default_config from "./default_config.ts";

(async () => {
  const globalConfig = await default_config.buildGlobal();
  const value: sunwet.Config = {
    bind_addr: "127.0.0.1:8080",
    temp_dir: "./temp",
    cache_dir: "./cache",
    graph_dir: "./persistent/graph",
    files_dir: "./persistent/files",
    global: {
      local: {
        public_iam_grants: "admin",
        api_tokens: {},
        views: globalConfig.views,
        forms: globalConfig.forms,
        menu: globalConfig.menu,
      },
    },
  };
  fs.writeFileSync("./config.json", JSON.stringify(value, null, 4));
})();
