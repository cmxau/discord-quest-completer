import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { formatLogs, type LogLine } from '@/composables/feedback';

export interface LogReportInfo {
    version?: string;
    windows?: string;
    userName?: string;
    game?: string;
    /** When the report was made (ISO text); the caller supplies it so the output is predictable. */
    exportedAt: string;
}

/**
 * The whole app log as a text report: the app and Windows versions, then every line, with the
 * Windows user name and personal folder names removed so it is safe to attach to a public issue.
 */
export function formatLogReport(info: LogReportInfo, logs: LogLine[]): string {
    const header = [
        'Discord Quest Completer log',
        `App version: ${info.version || 'unknown'}`,
        `Windows: ${info.windows || 'unknown'}`,
        info.game ? `Selected game: ${info.game}` : '',
        `Exported: ${info.exportedAt}`,
        `Lines: ${logs.length}`,
        'Personal folder names are removed.',
    ].filter(Boolean);
    return `${header.join('\n')}\n\n${formatLogs(logs, info.userName ?? '', Infinity) || '(the log is empty)'}\n`;
}

async function buildLogReport(logs: LogLine[], game: string): Promise<string> {
    const [version, windows, userName] = await Promise.all([
        getVersion().catch(() => ''),
        invoke<string>('windows_version').catch(() => ''),
        invoke<string>('current_user_name').catch(() => ''),
    ]);
    return formatLogReport({ version, windows, userName, game, exportedAt: new Date().toISOString() }, logs);
}

/** Ask where to save the log and write it there. Resolves to the saved path, or null if cancelled. */
export async function exportLog(logs: LogLine[], game = ''): Promise<string | null> {
    const contents = await buildLogReport(logs, game);
    const day = new Date().toISOString().slice(0, 10);
    return invoke<string | null>('export_log', { contents, file_name: `quest-completer-log-${day}.txt` });
}

/** Put the log on the clipboard. */
export async function copyLog(logs: LogLine[], game = ''): Promise<void> {
    await navigator.clipboard.writeText(await buildLogReport(logs, game));
}
