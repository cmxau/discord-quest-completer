import { EXECUTABLE_OS } from '@/constants/constants';
import type { Game } from '@/types/types';

/** "Shift at Midnight" -> "ShiftAtMidnight". Returns '' if nothing usable is left (e.g. non-Latin titles). */
export function toPascalCase(title: string): string {
    return title
        .normalize('NFKD')
        .replace(/[̀-ͯ]/g, '') // strip accents: é -> e
        .replace(/[^A-Za-z0-9]+/g, ' ')
        .trim()
        .split(' ')
        .filter(Boolean)
        .map(word => word.charAt(0).toUpperCase() + word.slice(1))
        .join('');
}

/**
 * Discord's detectable list leaves `executables` empty for many games. Give those a generated
 * `<PascalCaseTitle>.exe` so there is still something to launch, flagged as auto-generated.
 */
export function withFallbackExecutable(game: Game): Game {
    if (game.executables && game.executables.length > 0) {
        return game;
    }

    const base = toPascalCase(game.name ?? '');
    if (!base) {
        return game;
    }

    return {
        ...game,
        executables: [{
            name: `${base}.exe`,
            os: EXECUTABLE_OS.WINDOWS,
            is_launcher: false,
            is_auto_generated: true,
        }],
    };
}
