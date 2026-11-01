/* Own synthetic UI for observer verification. Contains no retail material. */
typedef unsigned long DWORD;
typedef void *HANDLE;
#define CALL __stdcall
#define IMP __declspec(dllimport)
IMP HANDLE CALL LoadLibraryA(const char *);
IMP void *CALL GetProcAddress(HANDLE, const char *);
IMP __declspec(noreturn) void CALL ExitProcess(DWORD);
typedef struct { HANDLE window; unsigned message, wparam; long lparam; DWORD time; long x, y; } MSG;
_Static_assert(sizeof(MSG) == 28, "Win32 MSG ABI");
void mainCRTStartup(void) {
#ifdef NO_WINDOW
    ExitProcess(0);
#endif
    HANDLE user = LoadLibraryA("user32.dll"), kernel = LoadLibraryA("kernel32.dll");
    HANDLE (CALL *create)(DWORD, const char *, const char *, DWORD, int, int, int, int, HANDLE, HANDLE, HANDLE, void *) =
        (void *)GetProcAddress(user, "CreateWindowExA");
    int (CALL *destroy)(HANDLE) = (void *)GetProcAddress(user, "DestroyWindow");
    int (CALL *peek)(MSG *, HANDLE, unsigned, unsigned, unsigned) = (void *)GetProcAddress(user, "PeekMessageA");
    int (CALL *translate)(const MSG *) = (void *)GetProcAddress(user, "TranslateMessage");
    long (CALL *dispatch)(const MSG *) = (void *)GetProcAddress(user, "DispatchMessageA");
    DWORD (CALL *ticks)(void) = (void *)GetProcAddress(kernel, "GetTickCount");
    void (CALL *sleep_ms)(DWORD) = (void *)GetProcAddress(kernel, "Sleep");
    if (!create || !destroy || !peek || !translate || !dispatch || !ticks || !sleep_ms) ExitProcess(2);
    HANDLE root = create(0, "STATIC", "OpenYR synthetic oracle fixture", 0x10cf0000, 80, 80, 1000, 740, 0, 0, 0, 0);
    HANDLE page = create(0, "#32770", "", 0x50000000, 20, 40, 800, 600, root, (HANDLE)100, 0, 0);
    HANDLE header = create(0, "STATIC", "GUI:FixtureHeading", 0x50000000, 640, 3, 150, 20, page, (HANDLE)101, 0, 0);
    HANDLE panel = create(0, "STATIC", "", 0x50000004, 40, 50, 145, 116, page, (HANDLE)102, 0, 0);
    HANDLE edit = create(0, "EDIT", "PRIVATE_FIXTURE_VALUE_7d39", 0x50800000, 30, 220, 180, 24, page, (HANDLE)103, 0, 0);
    if (!root || !page || !header || !panel || !edit) ExitProcess(5);
    DWORD start = ticks(); MSG msg = {0};
    while ((DWORD)(ticks() - start) < 5000) {
        while (peek(&msg, 0, 0, 0, 1)) { translate(&msg); dispatch(&msg); }
        sleep_ms(10);
    }
    destroy(root); ExitProcess(0);
}
