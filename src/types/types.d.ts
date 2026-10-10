
export interface GameExecutable {
  is_launcher: boolean;
  name: string;
  os: string;
  filename?: string;
  path?: string;
  is_running?: boolean;
  is_installed?: boolean;
  /** Generated from the game title because Discord lists no executables for it. */
  is_auto_generated?: boolean;
}
export interface Game {
    uid?: string;
    id: string;
    name: string;
    executables: GameExecutable[];
    aliases?: string[];
    themes?: string[];
    third_party_skus?: { distributor: string; id: string }[];
    /** File name of the dummy exe running from the Steam library (set while launched that way). */
    steam_exe?: string;
    is_running?: boolean;
    is_installed?: boolean;
    /** Pinned to the top of the sidebar. Saved with the list. */
    favorite?: boolean;
    /** When it was last started (ms since epoch). Saved with the list. */
    last_used?: number;
}

export interface GameActionsProvider {
  isExecutableRunning: (executable: GameExecutable) => boolean;
  isGameExecutableInstalled: (executable: GameExecutable) => boolean;
}