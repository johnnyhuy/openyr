/* Public Win32 window observations only. No game code or process memory reads. */
typedef unsigned long DWORD;
typedef int BOOL;
typedef void *HANDLE;
typedef long LPARAM;
typedef struct { long left, top, right, bottom; } RECT;
typedef struct { long x, y; } POINT;
_Static_assert(sizeof(DWORD) == 4 && sizeof(RECT) == 16 && sizeof(POINT) == 8, "Win32 ABI");
#define CALL __stdcall
#define IMP __declspec(dllimport)
#ifndef TARGET_IMAGE
#define TARGET_IMAGE "C:\\Westwood\\RA2\\gamemd.exe"
#endif
#ifndef LAUNCH
#define LAUNCH 0
#endif
#ifndef TARGET_DIRECTORY
#define TARGET_DIRECTORY "C:\\Westwood\\RA2"
#endif
IMP HANDLE CALL GetStdHandle(DWORD);
IMP BOOL CALL WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
IMP __declspec(noreturn) void CALL ExitProcess(DWORD);
IMP HANDLE CALL LoadLibraryA(const char *);
IMP void *CALL GetProcAddress(HANDLE, const char *);
IMP DWORD CALL GetLastError(void);
IMP HANDLE CALL OpenProcess(DWORD, BOOL, DWORD);
IMP BOOL CALL CloseHandle(HANDLE);
IMP BOOL CALL QueryFullProcessImageNameA(HANDLE, DWORD, char *, DWORD *);
static HANDLE output, root, page;
static DWORD total, matched, roots, pages, target_pid, launched_pid;
static POINT origin;
static BOOL (CALL *enumerate)(BOOL (CALL *)(HANDLE, LPARAM), LPARAM);
static BOOL (CALL *children)(HANDLE, BOOL (CALL *)(HANDLE, LPARAM), LPARAM);
static DWORD (CALL *window_pid)(HANDLE, DWORD *);
static BOOL (CALL *visible)(HANDLE);
static BOOL (CALL *window_rect)(HANDLE, RECT *);
static BOOL (CALL *client_rect)(HANDLE, RECT *);
static BOOL (CALL *client_to_screen)(HANDLE, POINT *);
static int (CALL *class_name)(HANDLE, char *, int);
static int (CALL *window_text)(HANDLE, char *, int);
static int (CALL *control_id)(HANDLE);
static long (CALL *window_style)(HANDLE, int);
static BOOL (CALL *enabled)(HANDLE);
static void put(const char *s) {
    DWORD n = 0, written = 0;
    while (s[n]) n++;
    if (!WriteFile(output, s, n, &written, 0) || written != n) ExitProcess(3);
}
static void number(DWORD v) {
    char b[12]; unsigned n = 0;
    do { b[n++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (n) { char c[2] = {b[--n], 0}; put(c); }
}
static void signed_number(long v) {
    if (v < 0) { put("-"); number(0 - (DWORD)v); } else number((DWORD)v);
}
static void hex(DWORD v) {
    const char digits[] = "0123456789abcdef";
    char b[11] = "0x00000000";
    for (int i = 0; i < 8; i++) b[2 + i] = digits[(v >> (4 * (7 - i))) & 15];
    put(b);
}
static BOOL equal(const char *a, const char *b) {
    while (*a && *b) {
        char x = *a++, y = *b++;
        if (x >= 'A' && x <= 'Z') x += 32;
        if (y >= 'A' && y <= 'Z') y += 32;
        if (x != y) return 0;
    }
    return *a == *b;
}
static void rectangle(RECT r) {
    signed_number(r.left); put(","); signed_number(r.top); put(",");
    signed_number(r.right); put(","); signed_number(r.bottom);
}
static BOOL CALL find_root(HANDLE w, LPARAM unused) {
    (void)unused; total++;
    DWORD pid = 0; window_pid(w, &pid);
    HANDLE process = OpenProcess(0x1000, 0, pid);
    if (!process) return 1;
    char path[512]; DWORD size = sizeof(path);
    BOOL ok = QueryFullProcessImageNameA(process, 0, path, &size);
    CloseHandle(process);
    if (!ok || size >= sizeof(path) || !equal(path, TARGET_IMAGE)) return 1;
    matched++;
    RECT r = {0};
    if (!visible(w) || !client_rect(w, &r) || r.right < 640 || r.bottom < 480 ||
        r.right > 8192 || r.bottom > 8192) return 1;
    roots++; root = w; target_pid = pid;
    return 1;
}
static BOOL CALL find_page(HANDLE w, LPARAM unused) {
    (void)unused; DWORD pid = 0; window_pid(w, &pid);
    if (pid != target_pid || !visible(w)) return 1;
    char name[128] = {0}; RECT r = {0};
    class_name(w, name, sizeof(name));
    if (equal(name, "#32770") && client_rect(w, &r) && r.right == 800 && r.bottom == 600) {
        pages++; page = w;
    }
    return 1;
}
static BOOL CALL inventory(HANDLE w, LPARAM unused) {
    (void)unused; DWORD pid = 0; window_pid(w, &pid);
    if (pid != target_pid || !visible(w)) return 1;
    char name[128] = {0}; RECT wr = {0}, cr = {0}; POINT pt = {0};
    if (!class_name(w, name, sizeof(name)) || !window_rect(w, &wr) ||
        !client_rect(w, &cr) || !client_to_screen(w, &pt)) return 0;
    wr.left -= origin.x; wr.right -= origin.x;
    wr.top -= origin.y; wr.bottom -= origin.y;
    put("control id="); signed_number(control_id(w)); put(" class="); put(name);
    put(" enabled="); number(enabled(w)); put(" style="); hex((DWORD)window_style(w, -16));
    put(" window_xyxy="); rectangle(wr); put(" client_xywh=");
    signed_number(pt.x - origin.x); put(","); signed_number(pt.y - origin.y); put(",");
    signed_number(cr.right); put(","); signed_number(cr.bottom);
    /* Avoid player names, edit values, selected items and arbitrary captions. */
    if (equal(name, "Static") && wr.top < 40) {
        char text[128] = {0}; window_text(w, text, sizeof(text));
        if (text[0] == 'G' && text[1] == 'U' && text[2] == 'I' && text[3] == ':') {
            put(" label_key="); put(text);
        }
    }
    put("\n"); return 1;
}
static BOOL snapshot(BOOL report) {
    total = matched = roots = pages = target_pid = 0; root = page = 0;
    if (!enumerate(find_root, 0)) { put("EnumWindows failed\n"); ExitProcess(5); }
    if (report) {
        put("total_windows="); number(total); put(" matched_image_windows="); number(matched);
        put(" eligible_roots="); number(roots); put("\n");
    }
    if (roots != 1 || (launched_pid && target_pid != launched_pid)) return 0;
    if (!children(root, find_page, 0) || pages != 1 || !client_to_screen(page, &origin)) return 0;
    return 1;
}
static void report_page(void) {
    put("verified target="); put(TARGET_IMAGE); put(" pid="); number(target_pid);
    put(" page_client=800,600\n");
    if (!children(root, inventory, 0)) { put("control inventory failed\n"); ExitProcess(5); }
}
typedef struct {
    DWORD size; char *reserved, *desktop, *title;
    DWORD x, y, xsize, ysize, xchars, ychars, fill, flags;
    unsigned short show, reserved_size; unsigned char *reserved_data;
    HANDLE input, output, error;
} STARTUP;
typedef struct { HANDLE process, thread; DWORD pid, tid; } PROCESS;
_Static_assert(sizeof(STARTUP) == 68, "STARTUPINFOA ABI");
void mainCRTStartup(void) {
    output = GetStdHandle((DWORD)-11);
    put("OpenYR public UI observer; no input, game code or process memory reads\n");
    HANDLE user = LoadLibraryA("user32.dll"), kernel = LoadLibraryA("kernel32.dll");
    if (!user || !kernel) ExitProcess(2);
#define API(target, name) target = (void *)GetProcAddress(user, name); if (!target) ExitProcess(2)
    API(enumerate, "EnumWindows"); API(children, "EnumChildWindows");
    API(window_pid, "GetWindowThreadProcessId"); API(visible, "IsWindowVisible");
    API(window_rect, "GetWindowRect"); API(client_rect, "GetClientRect");
    API(client_to_screen, "ClientToScreen"); API(class_name, "GetClassNameA");
    API(window_text, "GetWindowTextA"); API(control_id, "GetDlgCtrlID");
    API(window_style, "GetWindowLongA"); API(enabled, "IsWindowEnabled");
#undef API
    BOOL found = snapshot(1);
    if (!LAUNCH) {
        if (!found) { put("no unique verified 800x600 target page\n"); ExitProcess(4); }
        report_page(); ExitProcess(0);
    }
    if (matched) { put("target already has windows; use observe instead of launch\n"); ExitProcess(4); }
    BOOL (CALL *create)(const char *, char *, void *, void *, BOOL, DWORD, void *, const char *, STARTUP *, PROCESS *) =
        (void *)GetProcAddress(kernel, "CreateProcessA");
    BOOL (CALL *exit_code)(HANDLE, DWORD *) = (void *)GetProcAddress(kernel, "GetExitCodeProcess");
    DWORD (CALL *wait)(HANDLE, DWORD) = (void *)GetProcAddress(kernel, "WaitForSingleObject");
    DWORD (CALL *ticks)(void) = (void *)GetProcAddress(kernel, "GetTickCount");
    void (CALL *sleep_ms)(DWORD) = (void *)GetProcAddress(kernel, "Sleep");
    if (!create || !exit_code || !wait || !ticks || !sleep_ms) ExitProcess(2);
    STARTUP startup = {0}; PROCESS child = {0}; startup.size = sizeof(startup);
    char command[] = "\"" TARGET_IMAGE "\"";
    if (!create(TARGET_IMAGE, command, 0, 0, 0, 0, 0, TARGET_DIRECTORY, &startup, &child)) {
        put("CreateProcess failed error="); number(GetLastError()); put("\n"); ExitProcess(6);
    }
    launched_pid = child.pid; put("created target pid="); number(child.pid); put("\n");
    DWORD start = ticks();
    for (;;) {
        if (snapshot(0)) { report_page(); CloseHandle(child.thread); CloseHandle(child.process); ExitProcess(0); }
        DWORD status = wait(child.process, 0);
        if (status == 0) {
            DWORD code = 0;
            if (!exit_code(child.process, &code)) { put("GetExitCodeProcess failed\n"); ExitProcess(5); }
            put("target exited without verified page; child_exit_code="); number(code); put("\n");
            CloseHandle(child.thread); CloseHandle(child.process); ExitProcess(4);
        }
        if (status != 258) { put("process wait failed\n"); ExitProcess(5); }
        if ((DWORD)(ticks() - start) >= 60000) {
            put("observation deadline reached; child still running; no termination sent\n");
            CloseHandle(child.thread); CloseHandle(child.process); ExitProcess(7);
        }
        sleep_ms(200);
    }
}
