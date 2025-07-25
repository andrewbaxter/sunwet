// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { FormFieldBool } from "./FormFieldBool";
import type { FormFieldComment } from "./FormFieldComment";
import type { FormFieldConstEnum } from "./FormFieldConstEnum";
import type { FormFieldNumber } from "./FormFieldNumber";
import type { FormFieldQueryEnum } from "./FormFieldQueryEnum";
import type { FormFieldRgbU8 } from "./FormFieldRgbU8";
import type { FormFieldText } from "./FormFieldText";

export type FormFieldType = "id" | { "comment": FormFieldComment } | { "text": FormFieldText } | { "number": FormFieldNumber } | { "bool": FormFieldBool } | "date" | "time" | "datetime" | { "rgb_u8": FormFieldRgbU8 } | { "const_enum": FormFieldConstEnum } | { "query_enum": FormFieldQueryEnum } | "file";
