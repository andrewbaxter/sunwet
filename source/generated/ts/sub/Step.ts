// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { StepJunction } from "./StepJunction";
import type { StepMove } from "./StepMove";
import type { StepRecurse } from "./StepRecurse";

export type Step = { "move": StepMove } | { "recurse": StepRecurse } | { "junction": StepJunction };
