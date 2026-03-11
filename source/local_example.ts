import fs from "node:fs";
import * as sunwet from "./generated/ts/index.ts";
import * as default_config from "./default_config.ts";

(async () => {
  const globalConfig = await default_config.buildGlobal();
  const value: sunwet.Config = {
    bind_addr: "127.0.0.1:8080",
    temp_dir: `${process.env.SUNWET_TEMP_DIR}`,
    cache_dir: `${process.env.SUNWET_CACHE_DIR}`,
    graph_dir: `${process.env.SUNWET_PERSISTENT_DIR}/graph`,
    files_dir: `${process.env.SUNWET_PERSISTENT_DIR}/files`,
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
