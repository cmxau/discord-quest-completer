import { Game } from "@/types/types";
import { InjectionKey } from "vue";

// Only Windows is supported.
export const EXECUTABLE_OS = {
    WINDOWS: 'win32',
} as const;

export const GameActionsKey = Symbol() as InjectionKey<string>;

export function getCurrentOS(): string {
    return EXECUTABLE_OS.WINDOWS;
}
