#include "platform.hpp"

using namespace std;

#ifndef _WIN32

pair<int, int> platform::spawn_backend(const char* path) {
    int from_backend[2];
    int to_backend[2];

    pipe(from_backend);
    pipe(to_backend);

    if (!fork()) {
        dup2(to_backend[0], 0);
        dup2(from_backend[1], 1);
        close(to_backend[0]);
        close(to_backend[1]);
        close(from_backend[0]);
        close(from_backend[1]);
        execl(path, path, ".", NULL);
        assert(0 && "backend failed to start");
    } else {
        close(to_backend[0]);
        close(from_backend[1]);
    }

    return make_pair(from_backend[0], to_backend[1]);
}

#else

#include <windows.h>

static pair<HANDLE, HANDLE> create_overlapped_pipe(const char* name,
                                                   bool inherit_read,
                                                   bool inherit_write) {
    SECURITY_ATTRIBUTES inherit_attrs;
    inherit_attrs.nLength = sizeof(SECURITY_ATTRIBUTES);
    inherit_attrs.bInheritHandle = true;
    inherit_attrs.lpSecurityDescriptor = NULL;

    HANDLE read = CreateNamedPipe(
            name,
            PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            1024,   // buffer size
            1024,
            0,      // timeout: default
            inherit_read ? &inherit_attrs : NULL);

    HANDLE write = CreateFile(
            name,
            GENERIC_WRITE,
            0,  // forbid sharing
            inherit_write ? &inherit_attrs : NULL,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
            NULL);  // no template file

    return make_pair(read, write);
}

pair<HANDLE, HANDLE> platform::spawn_backend(const char* path) {

    char name_from[64] = {0};
    snprintf(name_from, sizeof(name_from),
            "\\\\.\\pipe\\outpost-wrapper-%08x-from",
            GetCurrentProcessId());

    char name_to[64] = {0};
    snprintf(name_to, sizeof(name_to),
            "\\\\.\\pipe\\outpost-wrapper-%08x-to",
            GetCurrentProcessId());

    auto from_backend = create_overlapped_pipe(name_from, false, true);
    auto to_backend = create_overlapped_pipe(name_to, true, false);

    PROCESS_INFORMATION proc = {0};
    STARTUPINFO info = {0};
    info.cb = sizeof(STARTUPINFO);
    info.hStdInput = to_backend.first;
    info.hStdOutput = from_backend.second;
    info.hStdError = GetStdHandle(STD_ERROR_HANDLE);
    info.dwFlags |= STARTF_USESTDHANDLES;

    const char* suffix = ".exe .";
    if (strlen(path) + strlen(suffix) >= 256) {
        abort();
    }

    char buf[256] = {0};
    strcpy(buf, path);
    strcat(buf, suffix);

    CreateProcess(
            NULL,
            buf,
            NULL,
            NULL,
            true,
            0,
            NULL,
            NULL,
            &info,
            &proc);

    CloseHandle(proc.hProcess);
    CloseHandle(proc.hThread);
    CloseHandle(from_backend.second);
    CloseHandle(to_backend.first);

    return make_pair(from_backend.first, to_backend.second);
}

#endif
