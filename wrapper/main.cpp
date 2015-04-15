#include <boost/asio.hpp>
//#include <websocketpp/config/asio_no_tls.hpp>
//#include <websocketpp/server.hpp>

#include "server.hpp"

using namespace std;
using namespace boost::asio;
using boost::system::error_code;


int main(int argc, char *argv[]) {
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
        execl("bin/backend", "bin/backend", ".", NULL);
        assert(0 && "backend failed to start");
    }

    io_service ios;

    server s(ios, to_backend[1], from_backend[0], 8889);

    ios.run();
}
