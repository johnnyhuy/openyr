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
#ifdef BOOTSTRAP
typedef struct {
    DWORD size; char *reserved, *desktop, *title;
    DWORD x, y, xsize, ysize, xchars, ychars, fill, flags;
    unsigned short show, reserved_size; unsigned char *reserved_data;
    HANDLE input, output, error;
} STARTUP;
typedef struct { HANDLE process, thread; DWORD pid, tid; } PROCESS;
_Static_assert(sizeof(STARTUP) == 68, "STARTUPINFOA ABI");
static void bootstrap(void) {
    HANDLE kernel = LoadLibraryA("kernel32.dll");
    int (CALL *create)(const char *, char *, void *, void *, int, DWORD, void *, const char *, STARTUP *, PROCESS *) =
        (void *)GetProcAddress(kernel, "CreateProcessA");
    DWORD (CALL *wait)(HANDLE, DWORD) = (void *)GetProcAddress(kernel, "WaitForSingleObject");
    int (CALL *exit_code)(HANDLE, DWORD *) = (void *)GetProcAddress(kernel, "GetExitCodeProcess");
    int (CALL *close)(HANDLE) = (void *)GetProcAddress(kernel, "CloseHandle");
    if (!create || !wait || !exit_code || !close) ExitProcess(2);
    STARTUP info = {0}; PROCESS child = {0}; info.size = sizeof(info);
    char command[] = "\"C:\\openyr-oracle\\fixture32.exe\"";
    if (!create("C:\\openyr-oracle\\fixture32.exe", command, 0, 0, 0, 0, 0,
                "C:\\openyr-oracle", &info, &child)) ExitProcess(6);
    DWORD code = 0;
    if (wait(child.process, 0xffffffff) != 0 || !exit_code(child.process, &code)) ExitProcess(5);
    close(child.thread); close(child.process); ExitProcess(code);
}
#endif
void mainCRTStartup(void) {
#ifdef BOOTSTRAP
    bootstrap();
#endif
#ifdef NO_WINDOW
    ExitProcess(0);
#endif
    HANDLE user = LoadLibraryA("user32.dll"), kernel = LoadLibraryA("kernel32.dll");
    HANDLE (CALL *create)(DWORD, const char *, const char *, DWORD, int, int, int, int, HANDLE, HANDLE, HANDLE, void *) =
        (void *)GetProcAddress(user, "CreateWindowExA");
    int (CALL *destroy)(HANDLE) = (void *)GetProcAddress(user, "DestroyWindow");
    int (CALL *show)(HANDLE, int) = (void *)GetProcAddress(user, "ShowWindow");
    int (CALL *peek)(MSG *, HANDLE, unsigned, unsigned, unsigned) = (void *)GetProcAddress(user, "PeekMessageA");
    int (CALL *translate)(const MSG *) = (void *)GetProcAddress(user, "TranslateMessage");
    long (CALL *dispatch)(const MSG *) = (void *)GetProcAddress(user, "DispatchMessageA");
    DWORD (CALL *ticks)(void) = (void *)GetProcAddress(kernel, "GetTickCount");
    void (CALL *sleep_ms)(DWORD) = (void *)GetProcAddress(kernel, "Sleep");
    if (!create || !destroy || !show || !peek || !translate || !dispatch || !ticks || !sleep_ms) ExitProcess(2);
    HANDLE root = create(0, "STATIC", "OpenYR synthetic oracle fixture", 0x00cf0000, 80, 80, 1000, 740, 0, 0, 0, 0);
    HANDLE page = create(0, "#32770", "", 0x50000000, 20, 40, 800, 600, root, (HANDLE)100, 0, 0);
    HANDLE header = create(0, "STATIC", "GUI:FixtureHeading", 0x50000000, 640, 3, 150, 20, page, (HANDLE)101, 0, 0);
    HANDLE panel = create(0, "STATIC", "", 0x50000004, 40, 50, 145, 116, page, (HANDLE)102, 0, 0);
    HANDLE edit = create(0, "EDIT", "PRIVATE_FIXTURE_VALUE_7d39", 0x50800000, 30, 220, 180, 24, page, (HANDLE)103, 0, 0);
    if (!root || !page || !header || !panel || !edit) ExitProcess(5);
    /* Publish the complete control tree before an observer can sample it. */
    show(root, 5);
    DWORD start = ticks(); MSG msg = {0};
    while ((DWORD)(ticks() - start) < 5000) {
        while (peek(&msg, 0, 0, 0, 1)) { translate(&msg); dispatch(&msg); }
        sleep_ms(10);
    }
    destroy(root); ExitProcess(0);
}
