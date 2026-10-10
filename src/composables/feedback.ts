import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';

export const REPO_URL = 'https://github.com/cmxau/discord-quest-completer';
const NEW_ISSUE_URL = `${REPO_URL}/issues/new`;

export type FeedbackKind = 'bug' | 'feature';

/** One line of the in-app log, as kept on the Diagnostics page. */
export interface LogLine {
    type: string;
    message: string;
    timestamp: Date | string | number;
}

export interface FeedbackDetails {
    version?: string;
    windows?: string;
    game?: string;
    logs?: string;
}

// The issue forms are in .github/ISSUE_TEMPLATE; the query parameters below are their field ids.
const TEMPLATES: Record<FeedbackKind, string> = {
    bug: 'bug_report.yml',
    feature: 'feature_request.yml',
};
const TITLE_PREFIX: Record<FeedbackKind, string> = {
    bug: '[Bug]: ',
    feature: '[Feature]: ',
};

// GitHub and browsers cut very long links off, so keep well inside the limit the backend allows.
const MAX_URL_LENGTH = 6500;
const MAX_LOG_LINES = 25;
const MAX_LOG_LINE_LENGTH = 300;

function escapeRegExp(text: string): string {
    return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Remove the Windows user name from text that will go into a public report. The machine's own user
 * name is replaced wherever it appears (it can contain spaces), and any other "C:\Users\<name>" path
 * is replaced as well.
 */
export function redactPersonalInfo(text: string, userName = ''): string {
    let out = text;
    const name = userName.trim();
    if (name.length >= 2) {
        // Whole words only: a short user name must not eat part of a game title ("Sam" in "Samurai").
        const word = String.raw`(?<![\p{L}\p{N}])` + escapeRegExp(name) + String.raw`(?![\p{L}\p{N}])`;
        out = out.replace(new RegExp(word, 'giu'), '<user>');
    }
    return out.replace(/([A-Za-z]:[\\/]+Users[\\/]+)[^\\/\s"'<>|]+/gi, '$1<user>');
}

/** The most recent log lines as plain text, newest last, with personal folder names removed. */
export function formatLogs(logs: LogLine[], userName = '', limit = MAX_LOG_LINES): string {
    return logs
        .slice(-limit)
        .map(line => {
            const time = new Date(line.timestamp);
            const clock = Number.isNaN(time.getTime()) ? '' : `[${time.toTimeString().slice(0, 8)}] `;
            const text = redactPersonalInfo(`${clock}[${line.type.toUpperCase()}] ${line.message}`, userName).replace(/\s+/g, ' ');
            return text.length > MAX_LOG_LINE_LENGTH ? `${text.slice(0, MAX_LOG_LINE_LENGTH)}...` : text;
        })
        .join('\n');
}

function link(kind: FeedbackKind, details: FeedbackDetails, logs: string): string {
    const params = new URLSearchParams({ template: TEMPLATES[kind], title: TITLE_PREFIX[kind] });
    const fields: Record<string, string | undefined> =
        kind === 'bug'
            ? { version: details.version, windows: details.windows, game: details.game, logs }
            : { version: details.version };
    for (const [id, value] of Object.entries(fields)) {
        if (value && value.trim()) {
            params.set(id, value);
        }
    }
    // URLSearchParams writes spaces as "+", which the issue forms read correctly.
    return `${NEW_ISSUE_URL}?${params.toString()}`;
}

/**
 * The link to a new, pre-filled issue. Long logs are shortened from the oldest end until the link
 * fits, so the form always opens.
 */
export function buildIssueUrl(kind: FeedbackKind, details: FeedbackDetails = {}): string {
    const lines = kind === 'bug' && details.logs ? details.logs.split('\n') : [];
    let kept = lines.length;
    for (;;) {
        const url = link(kind, details, lines.slice(lines.length - kept).join('\n'));
        if (url.length <= MAX_URL_LENGTH || kept === 0) {
            return url;
        }
        kept = Math.max(0, Math.floor(kept * 0.8) - (kept > 1 ? 0 : 1));
    }
}

/** What the app knows that helps a report: its version, the Windows version and what it was doing. */
export interface FeedbackContext {
    game?: string;
    logs?: LogLine[];
}

/** Open a pre-filled bug report or feature request in the browser. */
export async function openFeedback(kind: FeedbackKind, context: FeedbackContext = {}): Promise<void> {
    const [version, windows, userName] = await Promise.all([
        getVersion().catch(() => ''),
        invoke<string>('windows_version').catch(() => ''),
        invoke<string>('current_user_name').catch(() => ''),
    ]);
    const url = buildIssueUrl(kind, {
        version,
        windows,
        game: context.game,
        logs: formatLogs(context.logs ?? [], userName),
    });
    await invoke('open_issue_page', { url });
}
