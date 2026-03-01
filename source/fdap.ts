import * as sunwet from "./generated/ts/index.ts";
import * as process from "process";

export const sendFdap = async (
  globalConfig: sunwet.GlobalConfig,
  userConfig: {
    [_: string]: {
      // "fdap-login": fdap_login.UserConfig;
      "fdap-login": any;
      sunwet: sunwet.UserConfig;
    };
  },
) => {
  const config = {
    user: userConfig,
    sunwet: globalConfig,
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
};
