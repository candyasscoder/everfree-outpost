#include "backend.hpp"
#include "repl.hpp"
#include "server.hpp"

using namespace std;
using namespace boost::asio;


server::server(io_service& ios, int to_backend, int from_backend)
  : backend_(new backend(*this, ios, to_backend, from_backend)),
    repl_(new repl(*this, ios, "repl")) {
}

void server::handle_backend_response(uint16_t client_id, vector<uint8_t> msg) {
    if (client_id != 0) {
        return;
    }

    assert(msg.size() >= 2 && "control message has no opcode");
    uint16_t opcode = *(const uint16_t*)&msg[0];
    if (opcode == 0xff04) {
        repl_->handle_response(msg.begin() + 2, msg.end());
    }
}

void server::handle_repl_command(vector<uint8_t> command) {
    backend_->write(0, move(command));
}
