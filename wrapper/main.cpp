#include <boost/asio.hpp>

#include "server.hpp"

using namespace std;
using namespace boost::asio;
using boost::system::error_code;


#ifndef _WIN32

pair<int, int> spawn_backend(const char* path) {
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

    return make_pair(to_backend[1], from_backend[0]);
}

#else

pair<HANDLE, HANDLE> spawn_backend(const char* path) {
}

#endif


int main(int argc, char *argv[]) {
    io_service ios;

    auto backend_fds = spawn_backend("bin/backend");

#ifndef _WIN32
    local::stream_protocol::endpoint control_addr("control");
    local::stream_protocol::endpoint repl_addr("repl");
#else
    ip::tcp::endpoint control_addr(ip::address_v4::loopback(), 8890);
    ip::tcp::endpoint repl_addr(ip::address_v4::loopback(), 8891);
#endif

    server s(ios,
             backend_fds.first,
             backend_fds.second,
             control_addr,
             repl_addr,
             8888);

    ios.run();
}
