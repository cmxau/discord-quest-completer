<template>
    <div class="text-sm text-zinc-700 dark:text-zinc-300">
        <ol class="list-decimal space-y-1.5 pl-5">
            <li><span class="font-medium">Add the quest's game.</span> Search for it above the list and press Add.</li>
            <li><span class="font-medium">Press Play.</span> Newer quests that need a Steam install: use <span class="font-medium">Launch in Steam library</span> on the game's page.</li>
            <li><span class="font-medium">Keep Discord open</span> and wait for the quest's time (usually 15 minutes). The game stops by itself when auto-stop is up.</li>
            <li><span class="font-medium">Several quests?</span> Press Select, tick the games, then Run selected.</li>
            <li><span class="font-medium">Done?</span> Stop (or Stop all) in the sidebar. Close the window to the tray if you like.</li>
        </ol>
        <p class="mt-3 text-xs text-zinc-500 dark:text-zinc-400">
            Not counting? Check that Discord is running and that activity sharing / game detection is on, try the Steam library launch, and
            see Diagnostics for the log.
        </p>
        <p class="mt-3 text-xs text-zinc-500 dark:text-zinc-400">
            Shortcuts: <kbd class="rounded bg-zinc-100 px-1.5 py-0.5 font-mono dark:bg-zinc-800">Ctrl</kbd>+<kbd class="rounded bg-zinc-100 px-1.5 py-0.5 font-mono dark:bg-zinc-800">F</kbd> filter your games,
            <kbd class="rounded bg-zinc-100 px-1.5 py-0.5 font-mono dark:bg-zinc-800">Ctrl</kbd>+<kbd class="rounded bg-zinc-100 px-1.5 py-0.5 font-mono dark:bg-zinc-800">Enter</kbd> run the selected games,
            <kbd class="rounded bg-zinc-100 px-1.5 py-0.5 font-mono dark:bg-zinc-800">Esc</kbd> clear the filter or leave Select.
        </p>
    </div>
</template>
