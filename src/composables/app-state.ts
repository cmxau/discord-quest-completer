import { createGlobalState } from '@vueuse/core'
import { computed, ComputedRef, Ref, ShallowRef, shallowRef } from 'vue'

export const Pages = {
    HOME: 'home',
    DIAGNOSTICS: 'diagnostics',
    SETTINGS: 'settings',
} as const
export type Pages = typeof Pages[keyof typeof Pages]
export interface AppLogObject {
    type: 'info' | 'error' | 'warning' | 'debug';
    message: string;
    timestamp: Date;
}
export interface UseGlobalStateReturn {
    page: ShallowRef<Pages>,
    count: ShallowRef<number>,
    doubleCount: ComputedRef<number>,
    setPage: (newPage: Pages) => void,
    increment: () => void,
    logs: ShallowRef<AppLogObject[]>,
    addLog: {
        (type: 'info' | 'error' | 'warning' | 'debug', newLog: string): void;
        (newLog: string): void;
    };
    clearLogs: () => void,
    refreshRequest: ShallowRef<number>,
    /** Describes the selected game for bug reports, e.g. "AION 2 (Steam 3393110)"; empty when none. */
    selectedGameLabel: ShallowRef<string>,
    isGameListLoading: ShallowRef<boolean>,
    requestGameListRefresh: () => void,
}
export const useGlobalState = createGlobalState(
  () => {
    // state
    const page = shallowRef<Pages>(Pages.HOME)

    const logs = shallowRef<AppLogObject[]>([])

    // game list refresh: header button bumps the counter, HomeView watches it
    const refreshRequest = shallowRef(0)
    const selectedGameLabel = shallowRef('')
    const isGameListLoading = shallowRef(false)
    function requestGameListRefresh() {
      refreshRequest.value++
    }

    const count = shallowRef(0)

    // getters
    const doubleCount = computed(() => count.value * 2)

    // actions
    function increment() {
      count.value++
    }

    function setPage(newPage: Pages) {
      page.value = newPage
    }

    
    function addLog(type: string | 'info' | 'error' | 'warning' | 'debug' , newLog?: string) {
      if (!newLog) {
        newLog = type;
        type = 'info';
      }
      const formattedLog = `${newLog}`;
      logs.value.push({ type: type as 'info' | 'error' | 'warning' | 'debug', message: formattedLog, timestamp: new Date() });
    }

    function clearLogs() {
      logs.value = []
    }

    return {
        page,
        count, 
        doubleCount,
        setPage, 
        increment,
        logs,
        addLog,
        clearLogs,
        refreshRequest,
        selectedGameLabel,
        isGameListLoading,
        requestGameListRefresh
    } as UseGlobalStateReturn
  }
)

