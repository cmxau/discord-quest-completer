#include <windows.h>
#include <shellapi.h>

#define ID_BTN_HIDE       1002
#define ID_BTN_STOP       1005
#define WM_TRAY_MESSAGE   (WM_USER + 1)
#define ID_TRAY_EXIT      1003
#define ID_TRAY_TOGGLE    1004

typedef BOOL(WINAPI* P_Shell_NotifyIconW)(DWORD, PNOTIFYICONDATAW);
typedef HFONT(WINAPI* P_CreateFontW)(int, int, int, int, int, DWORD, DWORD, DWORD, DWORD, DWORD, DWORD, DWORD, DWORD, LPCWSTR);
typedef BOOL(WINAPI* P_DeleteObject)(HGDIOBJ);
typedef HWND(WINAPI* P_CreateWindowExW)(DWORD, LPCWSTR, LPCWSTR, DWORD, int, int, int, int, HWND, HMENU, HINSTANCE, LPVOID);
typedef HRESULT(WINAPI* P_DwmSetWindowAttribute)(HWND, DWORD, LPCVOID, DWORD);

P_Shell_NotifyIconW f_Shell_NotifyIconW;
P_CreateFontW f_CreateFontW;
P_DeleteObject f_DeleteObject;

wchar_t g_szGameName[256] = L"Discord Quest Completer";
NOTIFYICONDATAW nid = { 0 };
HFONT hFontSmall = NULL, hFontName = NULL, hFontText = NULL;

// Palette (zinc / indigo / emerald, matching the main app)
#define COLOR_BG      RGB(24, 24, 27)
#define COLOR_CARD    RGB(39, 39, 42)
#define COLOR_TEXT    RGB(250, 250, 250)
#define COLOR_MUTED   RGB(161, 161, 170)
#define COLOR_ACCENT  RGB(99, 102, 241)
#define COLOR_ACCENT_DOWN RGB(79, 70, 229)
#define COLOR_GREEN   RGB(34, 197, 94)
#define COLOR_STOP        RGB(220, 38, 38)
#define COLOR_STOP_DOWN   RGB(185, 28, 28)

wchar_t* FindStringW(const wchar_t* str, const wchar_t* substr) {
    while (*str) {
        const wchar_t* h = str, * n = substr;
        while (*h && *n && *h == *n) { h++; n++; }
        if (!*n) return (wchar_t*)str;
        str++;
    }
    return NULL;
}

void ToggleWindow(HWND hWnd) {
    if (IsWindowVisible(hWnd)) {
        ShowWindow(hWnd, SW_HIDE);
    }
    else {
        ShowWindow(hWnd, SW_SHOW);
        ShowWindow(hWnd, SW_RESTORE);
        SetForegroundWindow(hWnd);
    }
}

static void DrawLabel(HDC hdc, const wchar_t* text, RECT* rc, HFONT font, COLORREF color, UINT flags) {
    SelectObject(hdc, font);
    SetTextColor(hdc, color);
    DrawTextW(hdc, text, -1, rc, flags | DT_NOPREFIX);
}

static void PaintWindow(HWND hWnd) {
    PAINTSTRUCT ps;
    HDC hdc = BeginPaint(hWnd, &ps);

    RECT client;
    GetClientRect(hWnd, &client);

    HBRUSH bg = CreateSolidBrush(COLOR_BG);
    FillRect(hdc, &client, bg);
    DeleteObject(bg);

    SetBkMode(hdc, TRANSPARENT);

    // Accent strip along the top edge
    RECT strip = { 0, 0, client.right, 4 };
    HBRUSH accent = CreateSolidBrush(COLOR_ACCENT);
    FillRect(hdc, &strip, accent);
    DeleteObject(accent);

    // Card holding the game name and status
    RECT card = { 20, 24, client.right - 20, 124 };
    HBRUSH cardBrush = CreateSolidBrush(COLOR_CARD);
    HPEN noPen = (HPEN)GetStockObject(NULL_PEN);
    HGDIOBJ oldBrush = SelectObject(hdc, cardBrush);
    HGDIOBJ oldPen = SelectObject(hdc, noPen);
    RoundRect(hdc, card.left, card.top, card.right, card.bottom, 16, 16);
    SelectObject(hdc, oldBrush);
    SelectObject(hdc, oldPen);
    DeleteObject(cardBrush);

    RECT rcApp = { card.left + 18, card.top + 12, card.right - 18, card.top + 30 };
    DrawLabel(hdc, L"DISCORD QUEST COMPLETER", &rcApp, hFontSmall, COLOR_MUTED, DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS);

    RECT rcName = { card.left + 18, card.top + 32, card.right - 18, card.top + 68 };
    DrawLabel(hdc, g_szGameName, &rcName, hFontName, COLOR_TEXT, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);

    // Status: green dot + text
    int dotY = card.bottom - 20;
    HBRUSH green = CreateSolidBrush(COLOR_GREEN);
    oldBrush = SelectObject(hdc, green);
    oldPen = SelectObject(hdc, noPen);
    Ellipse(hdc, card.left + 18, dotY - 4, card.left + 26, dotY + 4);
    SelectObject(hdc, oldBrush);
    SelectObject(hdc, oldPen);
    DeleteObject(green);

    RECT rcStatus = { card.left + 34, dotY - 10, card.right - 18, dotY + 10 };
    DrawLabel(hdc, L"Running \x2014 Discord should detect this game", &rcStatus, hFontText, COLOR_GREEN, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);

    RECT rcHint = { 20, 136, client.right - 20, 156 };
    DrawLabel(hdc, L"Keep this window running while the quest completes.", &rcHint, hFontText, COLOR_MUTED, DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS);

    EndPaint(hWnd, &ps);
}

static void DrawButton(DRAWITEMSTRUCT* dis) {
    bool isStop = (dis->CtlID == ID_BTN_STOP);
    bool pressed = (dis->itemState & ODS_SELECTED) != 0;
    COLORREF fill = isStop ? (pressed ? COLOR_STOP_DOWN : COLOR_STOP) : (pressed ? COLOR_ACCENT_DOWN : COLOR_ACCENT);
    HBRUSH brush = CreateSolidBrush(fill);
    HPEN noPen = (HPEN)GetStockObject(NULL_PEN);
    HGDIOBJ oldBrush = SelectObject(dis->hDC, brush);
    HGDIOBJ oldPen = SelectObject(dis->hDC, noPen);
    RoundRect(dis->hDC, dis->rcItem.left, dis->rcItem.top, dis->rcItem.right, dis->rcItem.bottom, 10, 10);
    SelectObject(dis->hDC, oldBrush);
    SelectObject(dis->hDC, oldPen);
    DeleteObject(brush);

    SetBkMode(dis->hDC, TRANSPARENT);
    RECT rc = dis->rcItem;
    DrawLabel(dis->hDC, isStop ? L"Stop" : L"Hide to tray", &rc, hFontText, COLOR_TEXT, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
}

LRESULT CALLBACK WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam) {
    switch (message) {
    case WM_CREATE: {
        hFontSmall = f_CreateFontW(14, 0, 0, 0, FW_SEMIBOLD, 0, 0, 0, DEFAULT_CHARSET, 0, 0, CLEARTYPE_QUALITY, 0, L"Segoe UI");
        hFontName = f_CreateFontW(26, 0, 0, 0, FW_BOLD, 0, 0, 0, DEFAULT_CHARSET, 0, 0, CLEARTYPE_QUALITY, 0, L"Segoe UI");
        hFontText = f_CreateFontW(16, 0, 0, 0, FW_NORMAL, 0, 0, 0, DEFAULT_CHARSET, 0, 0, CLEARTYPE_QUALITY, 0, L"Segoe UI");

        RECT client;
        GetClientRect(hWnd, &client);
        CreateWindowExW(0, L"BUTTON", L"Hide to tray", WS_VISIBLE | WS_CHILD | BS_OWNERDRAW,
            client.right - 20 - 130, 164, 130, 32, hWnd, (HMENU)ID_BTN_HIDE, NULL, NULL);
        CreateWindowExW(0, L"BUTTON", L"Stop", WS_VISIBLE | WS_CHILD | BS_OWNERDRAW,
            client.right - 20 - 130 - 10 - 90, 164, 90, 32, hWnd, (HMENU)ID_BTN_STOP, NULL, NULL);

        nid.cbSize = sizeof(NOTIFYICONDATAW);
        nid.hWnd = hWnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY_MESSAGE;
        nid.hIcon = LoadIcon(NULL, IDI_APPLICATION);
        lstrcpynW(nid.szTip, g_szGameName, 128);
        f_Shell_NotifyIconW(NIM_ADD, &nid);
    } break;

    case WM_ERASEBKGND:
        return 1; // fully painted in WM_PAINT

    case WM_PAINT:
        PaintWindow(hWnd);
        break;

    case WM_DRAWITEM:
        if (wParam == ID_BTN_HIDE || wParam == ID_BTN_STOP) {
            DrawButton((DRAWITEMSTRUCT*)lParam);
            return TRUE;
        }
        return DefWindowProcW(hWnd, message, wParam, lParam);

    case WM_TRAY_MESSAGE:
        if (lParam == WM_RBUTTONUP) {
            POINT cur; GetCursorPos(&cur);
            HMENU hMenu = CreatePopupMenu();
            InsertMenuW(hMenu, 0, MF_BYPOSITION | MF_STRING, ID_TRAY_TOGGLE, IsWindowVisible(hWnd) ? L"Hide" : L"Show");
            InsertMenuW(hMenu, 2, MF_SEPARATOR, 0, NULL);
            InsertMenuW(hMenu, 3, MF_BYPOSITION | MF_STRING, ID_TRAY_EXIT, L"Exit");
            SetForegroundWindow(hWnd);
            TrackPopupMenu(hMenu, TPM_BOTTOMALIGN | TPM_LEFTALIGN, cur.x, cur.y, 0, hWnd, NULL);
            DestroyMenu(hMenu);
        }
        else if (lParam == WM_LBUTTONDBLCLK) ToggleWindow(hWnd);
        break;

    case WM_COMMAND:
        if (LOWORD(wParam) == ID_BTN_HIDE) ToggleWindow(hWnd);
        if (LOWORD(wParam) == ID_BTN_STOP) DestroyWindow(hWnd);
        if (LOWORD(wParam) == ID_TRAY_EXIT) DestroyWindow(hWnd);
        if (LOWORD(wParam) == ID_TRAY_TOGGLE) ToggleWindow(hWnd);
        break;

    case WM_DESTROY:
        f_Shell_NotifyIconW(NIM_DELETE, &nid);
        if (hFontSmall) f_DeleteObject(hFontSmall);
        if (hFontName) f_DeleteObject(hFontName);
        if (hFontText) f_DeleteObject(hFontText);
        PostQuitMessage(0);
        break;

    default: return DefWindowProcW(hWnd, message, wParam, lParam);
    }
    return 0;
}

extern "C" {
    #pragma function(memset)
    void* memset(void* dest, int c, size_t count) {
        char* bytes = (char*)dest;
        while (count--) {
            *bytes++ = (char)c;
        }
        return dest;
    }
}

extern "C" void mainEntryPoint() {
    HINSTANCE hInst = GetModuleHandleW(NULL);

    HMODULE hShell32 = LoadLibraryW(L"shell32.dll");
    HMODULE hGdi32 = LoadLibraryW(L"gdi32.dll");
    HMODULE hUser32 = LoadLibraryW(L"user32.dll");

    f_Shell_NotifyIconW = (P_Shell_NotifyIconW)GetProcAddress(hShell32, "Shell_NotifyIconW");
    f_CreateFontW = (P_CreateFontW)GetProcAddress(hGdi32, "CreateFontW");
    f_DeleteObject = (P_DeleteObject)GetProcAddress(hGdi32, "DeleteObject");
    P_CreateWindowExW f_CreateWindowExW = (P_CreateWindowExW)GetProcAddress(hUser32, "CreateWindowExW");

    LPWSTR lpCmdLine = GetCommandLineW();
    const wchar_t* flag = L"--title";
    wchar_t* pos = FindStringW(lpCmdLine, flag);

    if (pos) {
        pos += 7;
        while (*pos == L' ') pos++;

        if (*pos == L'"') {
            pos++;
            int i = 0;
            while (*pos != L'"' && *pos != L'\0' && i < 255) {
                g_szGameName[i++] = *pos++;
            }
            g_szGameName[i] = L'\0';
        } else {
            int i = 0;
            while (*pos != L' ' && *pos != L'\0' && i < 255) {
                g_szGameName[i++] = *pos++;
            }
            g_szGameName[i] = L'\0';
        }
    }

    WNDCLASSW wc = { 0 };
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInst;
    wc.lpszClassName = L"DQCTray";
    wc.hbrBackground = NULL;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    RegisterClassW(&wc);

    HWND hWnd = f_CreateWindowExW(0, wc.lpszClassName, g_szGameName, WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX, CW_USEDEFAULT, CW_USEDEFAULT, 440, 250, NULL, NULL, hInst, NULL);

    // Dark title bar (Windows 10 1809+ / 11); silently ignored where unsupported.
    HMODULE hDwm = LoadLibraryW(L"dwmapi.dll");
    if (hDwm) {
        P_DwmSetWindowAttribute f_Dwm = (P_DwmSetWindowAttribute)GetProcAddress(hDwm, "DwmSetWindowAttribute");
        if (f_Dwm) {
            BOOL dark = TRUE;
            f_Dwm(hWnd, 20 /* DWMWA_USE_IMMERSIVE_DARK_MODE */, &dark, sizeof(dark));
        }
    }

    ShowWindow(hWnd, SW_SHOWNORMAL);

    MSG msg;
    while (GetMessageW(&msg, NULL, 0, 0)) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    ExitProcess(0);
}
