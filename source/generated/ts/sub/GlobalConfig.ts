// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { ConfigIamGrants } from "./ConfigIamGrants";
import type { ConfigIamGrantsLimited } from "./ConfigIamGrantsLimited";
import type { Form } from "./Form";
import type { FormId } from "./FormId";
import type { ServerConfigMenuItem } from "./ServerConfigMenuItem";
import type { View } from "./View";
import type { ViewId } from "./ViewId";

export type GlobalConfig = { api_tokens: { [key in string]?: ConfigIamGrants }, public_iam_grants: ConfigIamGrantsLimited, menu: Array<ServerConfigMenuItem>, 
/**
 * View ids to view definitions
 */
views: { [key in ViewId]?: View }, 
/**
 * Form ids to form definitions
 */
forms: { [key in FormId]?: Form }, };
