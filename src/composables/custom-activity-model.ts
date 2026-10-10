// What a custom Rich Presence status is and how it is checked, cleaned up and sent to the backend.
// No Vue or Tauri in here, so it can be checked on its own.

export type ActivityKind = 'playing' | 'listening' | 'watching' | 'competing';

export const ACTIVITY_KINDS: { value: ActivityKind; label: string; code: number }[] = [
    { value: 'playing', label: 'Playing', code: 0 },
    { value: 'listening', label: 'Listening to', code: 2 },
    { value: 'watching', label: 'Watching', code: 3 },
    { value: 'competing', label: 'Competing in', code: 5 },
];

/** What each type looks like on a profile, so the page can show an example that reads correctly. */
export const ACTIVITY_EXAMPLES: Record<ActivityKind, { name: string; details: string; state: string }> = {
    playing: { name: 'Red Dead Redemption', details: 'Free roam', state: 'Chapter 2' },
    listening: { name: 'Lo-fi Beats', details: 'Chill study mix', state: 'by Night Owl' },
    watching: { name: 'Netflix', details: 'Stranger Things', state: 'Season 4, Episode 2' },
    competing: { name: 'Rocket League Ranked', details: 'Semi-finals', state: 'Round 3 of 5' },
};

/** The sentence Discord shows for a type and a name: "Playing X", "Listening to X", "Watching X", "Competing in X". */
export function activityHeadline(kind: ActivityKind, name: string): string {
    const label = ACTIVITY_KINDS.find(k => k.value === kind)?.label ?? 'Playing';
    return `${label} ${name}`;
}

export interface CustomActivity {
    id: string;
    /** The name of a saved preset. */
    label: string;
    /** The Discord application whose name Discord shows after "Playing", "Watching", ... */
    appId: string;
    kind: ActivityKind;
    /** First line under the name. */
    details: string;
    /** Second line under the name. */
    state: string;
    /** Show "elapsed" time counting up from when the status started. */
    showTimer: boolean;
}

/** Discord limits the details and state lines to this many characters. */
export const MAX_LINE = 128;
const MAX_PRESETS = 50;

export function emptyActivity(): CustomActivity {
    return { id: '', label: '', appId: '', kind: 'playing', details: '', state: '', showTimer: true };
}

function text(value: unknown, max: number): string {
    return typeof value === 'string' ? value.slice(0, max) : '';
}

/** Why this status can't be started, or null when it is fine. */
export function validateActivity(activity: Pick<CustomActivity, 'appId' | 'details' | 'state'>): string | null {
    if (!/^\d{15,20}$/.test(activity.appId.trim())) {
        return 'Enter the Application ID: a number of 15 to 20 digits.';
    }
    for (const [name, value] of [['Details', activity.details], ['State', activity.state]] as const) {
        const length = Array.from(value.trim()).length;
        if (length === 1) {
            return `${name} needs at least 2 characters (Discord's rule), or leave it empty.`;
        }
        if (length > MAX_LINE) {
            return `${name} can be at most ${MAX_LINE} characters.`;
        }
    }
    return null;
}

/** The JSON the backend's Rich Presence command takes. `nowSeconds` is the start time of the timer. */
export function buildActivityJson(activity: CustomActivity, nowSeconds: number): string {
    const kind = ACTIVITY_KINDS.find(k => k.value === activity.kind) ?? ACTIVITY_KINDS[0];
    return JSON.stringify({
        app_id: activity.appId.trim(),
        details: activity.details.trim() || undefined,
        state: activity.state.trim() || undefined,
        activity_kind: kind.code,
        timestamp: activity.showTimer ? Math.floor(nowSeconds) : undefined,
    });
}

/** Saved presets from whatever was stored: anything unreadable is dropped instead of throwing. */
export function normalizeActivities(raw: unknown): CustomActivity[] {
    if (!Array.isArray(raw)) {
        return [];
    }
    const seen = new Set<string>();
    const out: CustomActivity[] = [];
    for (const item of raw) {
        if (!item || typeof item !== 'object' || out.length >= MAX_PRESETS) {
            continue;
        }
        const source = item as Record<string, unknown>;
        const appId = typeof source.appId === 'string' ? source.appId.trim() : '';
        const id = typeof source.id === 'string' && source.id ? source.id : '';
        if (!/^\d{15,20}$/.test(appId) || !id || seen.has(id)) {
            continue;
        }
        seen.add(id);
        out.push({
            id,
            label: text(source.label, 60),
            appId,
            kind: ACTIVITY_KINDS.some(k => k.value === source.kind) ? (source.kind as ActivityKind) : 'playing',
            details: text(source.details, MAX_LINE),
            state: text(source.state, MAX_LINE),
            showTimer: typeof source.showTimer === 'boolean' ? source.showTimer : true,
        });
    }
    return out;
}

/** A name for a preset when the user didn't give one. */
export function activityTitle(activity: CustomActivity): string {
    const kind = ACTIVITY_KINDS.find(k => k.value === activity.kind)?.label ?? 'Playing';
    return activity.label.trim() || activity.details.trim() || `${kind} (app ${activity.appId.trim()})`;
}
