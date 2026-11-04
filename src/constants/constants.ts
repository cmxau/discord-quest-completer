import { Game } from "@/types/types";
import { InjectionKey } from "vue";

// The `os` values in Discord's detectable list.
export const EXECUTABLE_OS = {
    WINDOWS: 'win32',
    MAC: 'darwin',
    LINUX: 'linux',
} as const;

export const isMac = /Mac/i.test(navigator.userAgent);

export const GameActionsKey = Symbol() as InjectionKey<string>;

export function getCurrentOS(): string {
    if (isMac) return EXECUTABLE_OS.MAC;
    return /Linux|X11/i.test(navigator.userAgent) ? EXECUTABLE_OS.LINUX : EXECUTABLE_OS.WINDOWS;
}
