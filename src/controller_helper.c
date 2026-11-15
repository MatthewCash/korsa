/* Keep controller enumeration and AC in the same Proton/Wine session.
 * The Linux parent validates this manifest and rewrites only controller IDs
 * before granting permission to start the race. No Wine-specific GUID guessing.
 */
#define COBJMACROS
#define DIRECTINPUT_VERSION 0x0800
#include <windows.h>
#include <shellapi.h>
#include <dinput.h>
#include <stdio.h>
#include <stdlib.h>
#include <wchar.h>

static IDirectInput8W *input;
static HRESULT enum_error;

static void json_string(FILE *file, const WCHAR *text)
{
    int size = WideCharToMultiByte(CP_UTF8, 0, text, -1, NULL, 0, NULL, NULL);
    char *utf8 = malloc(size > 0 ? (size_t)size : 1);
    if (!utf8) ExitProcess(2);
    if (size > 0) WideCharToMultiByte(CP_UTF8, 0, text, -1, utf8, size, NULL, NULL);
    else utf8[0] = 0;
    fputc('"', file);
    for (const unsigned char *p = (const unsigned char *)utf8; *p; ++p) {
        if (*p == '"' || *p == '\\') fprintf(file, "\\%c", *p);
        else if (*p < 32) fprintf(file, "\\u%04x", *p);
        else fputc(*p, file);
    }
    fputc('"', file);
    free(utf8);
}

static void json_guid(FILE *file, const GUID *guid)
{
    WCHAR text[40];
    StringFromGUID2(guid, text, 40);
    json_string(file, text);
}

struct enumeration { FILE *file; BOOL first; };

static BOOL CALLBACK enumerate(const DIDEVICEINSTANCEW *instance, void *context)
{
    struct enumeration *enumeration = context;
    IDirectInputDevice8W *device;
    DIPROPGUIDANDPATH path = {0};
    HRESULT result = IDirectInput8_CreateDevice(input, &instance->guidInstance, &device, NULL);
    if (FAILED(result)) {
        enum_error = result;
        return DIENUM_STOP;
    }
    path.diph.dwSize = sizeof(path);
    path.diph.dwHeaderSize = sizeof(path.diph);
    path.diph.dwHow = DIPH_DEVICE;
    /* Some drivers lack a path. Product identity must then be unambiguous. */
    if (FAILED(IDirectInputDevice8_GetProperty(device, DIPROP_GUIDANDPATH, &path.diph)))
        path.wszPath[0] = 0;
    IDirectInputDevice8_Release(device);

    FILE *file = enumeration->file;
    if (!enumeration->first) fputc(',', file);
    enumeration->first = FALSE;
    fputs("{\"name\":", file);
    json_string(file, instance->tszInstanceName);
    fputs(",\"instance_guid\":", file);
    json_guid(file, &instance->guidInstance);
    fputs(",\"product_guid\":", file);
    json_guid(file, &instance->guidProduct);
    fputs(",\"path\":", file);
    json_string(file, path.wszPath);
    fputc('}', file);
    return DIENUM_CONTINUE;
}

static BOOL manifest(const WCHAR *filename)
{
    FILE *file = _wfopen(filename, L"wb");
    if (!file) return FALSE;
    struct enumeration enumeration = {file, TRUE};
    enum_error = S_OK;
    fputc('[', file);
    HRESULT result = IDirectInput8_EnumDevices(input, DI8DEVCLASS_GAMECTRL, enumerate,
                                              &enumeration, DIEDFL_ATTACHEDONLY);
    fputs("]\n", file);
    BOOL written = !ferror(file);
    if (fclose(file)) written = FALSE;
    return written && SUCCEEDED(result) && SUCCEEDED(enum_error);
}

static int fail(const char *message)
{
    FILE *file = _wfopen(L"error.txt", L"wb");
    if (file) { fputs(message, file); fclose(file); }
    if (input) IDirectInput8_Release(input);
    return 1;
}

static BOOL files_equal(const WCHAR *a, const WCHAR *b)
{
    FILE *left = _wfopen(a, L"rb"), *right = _wfopen(b, L"rb");
    BOOL equal = left && right;
    if (equal) {
        int x, y;
        do { x = fgetc(left); y = fgetc(right); } while (x == y && x != EOF);
        equal = x == y && !ferror(left) && !ferror(right);
    }
    if (left) fclose(left);
    if (right) fclose(right);
    return equal;
}

static int run_helper(int argc, WCHAR **argv)
{
    if (argc != 3 || !SetCurrentDirectoryW(argv[1])) return 1;
    HRESULT result = DirectInput8Create(GetModuleHandleW(NULL), DIRECTINPUT_VERSION,
                                        &IID_IDirectInput8W, (void **)&input, NULL);
    if (FAILED(result)) return fail("DirectInput initialization failed; AC was not started.");
    if (!manifest(L"devices.pending") ||
        !MoveFileExW(L"devices.pending", L"devices.json", MOVEFILE_REPLACE_EXISTING))
        return fail("Could not enumerate attached DirectInput controllers; AC was not started.");

    /* Fail closed on parent cancellation, crashes, or a missing approval file. */
    ULONGLONG deadline = GetTickCount64() + 60000;
    while (GetFileAttributesW(L"continue") == INVALID_FILE_ATTRIBUTES) {
        if (GetFileAttributesW(L"abort") != INVALID_FILE_ATTRIBUTES ||
            GetFileAttributesW(argv[1]) == INVALID_FILE_ATTRIBUTES) {
            IDirectInput8_Release(input);
            return 1;
        }
        if (GetTickCount64() >= deadline)
            return fail("Controller validation timed out; AC was not started.");
        Sleep(50);
    }
    /* Detect hotplug during the handshake rather than using an obsolete map. */
    if (!manifest(L"devices.check") || !files_equal(L"devices.json", L"devices.check"))
        return fail("Controllers changed during launch. Reconnect them and try again.");

    WCHAR *directory = _wcsdup(argv[2]);
    if (!directory) return fail("Out of memory.");
    WCHAR *separator = wcsrchr(directory, L'\\');
    if (!separator) { free(directory); return fail("Invalid AC executable path."); }
    *separator = 0;
    size_t length = wcslen(argv[2]) + 3;
    WCHAR *command = calloc(length, sizeof(WCHAR));
    if (!command) { free(directory); return fail("Out of memory."); }
    swprintf(command, length, L"\"%ls\"", argv[2]);
    STARTUPINFOW startup = {0};
    PROCESS_INFORMATION process = {0};
    startup.cb = sizeof(startup);
    BOOL started = CreateProcessW(argv[2], command, NULL, NULL, FALSE, 0, NULL,
                                  directory, &startup, &process);
    free(command);
    free(directory);
    if (!started) return fail("CreateProcessW could not start Assetto Corsa.");
    /* Keep Wine alive until AC exits, including across the pre-launch handshake. */
    WaitForSingleObject(process.hProcess, INFINITE);
    DWORD exit_code = 1;
    GetExitCodeProcess(process.hProcess, &exit_code);
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    IDirectInput8_Release(input);
    return (int)exit_code;
}

/* Use the Windows GUI subsystem without creating a window. Unlike hiding a
 * console after startup, this prevents Wine's console window from flashing.
 * Keep AC's STARTUPINFO unchanged so the game still opens normally.
 */
int WINAPI wWinMain(HINSTANCE instance, HINSTANCE previous, WCHAR *command_line, int show)
{
    (void)instance;
    (void)previous;
    (void)command_line;
    (void)show;
    int argc;
    WCHAR **argv = CommandLineToArgvW(GetCommandLineW(), &argc);
    if (!argv) return 1;
    int result = run_helper(argc, argv);
    LocalFree(argv);
    return result;
}
