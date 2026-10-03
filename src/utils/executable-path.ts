// Discord lists executables as relative paths such as `win64/cs2.exe` (always forward slashes, but
// be lenient about backslashes too).

/** "win64/cs2.exe" -> ["win64", "cs2.exe"] */
export function pathSegments(name: string): string[] {
    return name.split(/[\\/]/);
}

/** "win64/cs2.exe" -> "cs2.exe" */
export function executableFileName(name: string): string {
    return pathSegments(name).pop() as string;
}

/** "win64/cs2.exe" -> "win64" (joined with `separator`), "" when there is no folder. */
export function executableDirectory(name: string, separator: string): string {
    return pathSegments(name).slice(0, -1).join(separator);
}

/** Breadcrumb parts for display: the folders, then the file name without its extension. */
export function executableBreadcrumbs(name: string): string[] {
    const segments = pathSegments(name);
    const file = segments.pop() as string;
    // keep the full name when there is no extension to strip
    const withoutExtension = file.split('.').slice(0, -1).join('.') || file;
    return [...segments, withoutExtension];
}
