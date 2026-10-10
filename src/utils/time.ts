/** 754000 -> "12:34", 3_754_000 -> "1:02:34". Never negative. */
export function formatCountdown(ms: number): string {
    const total = Math.max(0, Math.ceil(ms / 1000));
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    const seconds = total % 60;
    const pad = (n: number) => String(n).padStart(2, '0');
    return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${minutes}:${pad(seconds)}`;
}

/** "just now", "5 min ago", "3 h ago", "2 d ago". */
export function formatAgo(timestamp: number, now: number = Date.now()): string {
    const minutes = Math.floor(Math.max(0, now - timestamp) / 60_000);
    if (minutes < 1) return 'just now';
    if (minutes < 60) return `${minutes} min ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours} h ago`;
    return `${Math.floor(hours / 24)} d ago`;
}
