#include <boost/asio.hpp>

#include "server.hpp"

using namespace std;
using namespace boost::asio;
using boost::system::error_code;


int main(int argc, char *argv[]) {
    io_service ios;

#ifndef _WIN32
    local::stream_protocol::endpoint control_addr("control");
    local::stream_protocol::endpoint repl_addr("repl");
#else
    ip::tcp::endpoint control_addr(ip::address_v4::loopback(), 8890);
    ip::tcp::endpoint repl_addr(ip::address_v4::loopback(), 8891);
#endif

    server s(ios,
             "bin/backend",
             control_addr,
             repl_addr,
             8888);

    ios.run();
}
