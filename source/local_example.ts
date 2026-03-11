import fs from "node:fs";
import * as sunwet from "./generated/ts/index.ts";
import * as default_config from "./default_config.ts";

(async () => {
  const globalConfig = await default_config.buildGlobal();
  const value: sunwet.Config = {
    bind_addr: process.env.SUNWET_BIND_ADDR || "127.0.0.1:8080",
    cache_dir: `${process.env.SUNWET_CACHE_DIR}`,
    persistent_dir: `${process.env.SUNWET_PERSISTENT_DIR}`,
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
