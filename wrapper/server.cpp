#include "backend.hpp"
#include "opcode.hpp"
#include "repl.hpp"
#include "server.hpp"
#include "websocket.hpp"

using namespace std;
using namespace boost::asio;


server::server(io_service& ios, int to_backend, int from_backend)
    : backend_(new backend(*this, ios, to_backend, from_backend)),
      repl_(new repl(*this, ios, "repl")),
      websocket_(new websocket(*this, ios)) {
}

void server::handle_backend_response(uint16_t client_id, vector<uint8_t> msg) {
    if (client_id == 0) {
        const uint16_t* msg16 = (const uint16_t*)&msg[0];

        assert(msg.size() >= 2 && "control message has no opcode");
        uint16_t op = msg16[0];

        if (op == opcode::OP_CLIENT_REMOVED) {
            assert(msg.size() == 4);
            websocket_->handle_client_removed(msg16[1]);
        } else if (op == opcode::OP_REPL_RESULT) {
            repl_->handle_response(msg.begin() + 2, msg.end());
        }
    } else {
        websocket_->send_message(client_id, move(msg));
    }
}

void server::handle_repl_command(vector<uint8_t> command) {
    backend_->write(0, move(command));
}

void server::handle_websocket_connect(uint16_t client_id) {
    vector<uint8_t> msg(4);
    uint16_t* msg16 = (uint16_t*)&msg[0];
    msg16[0] = opcode::OP_ADD_CLIENT;
    msg16[1] = client_id;
    backend_->write(0, move(msg));
}

void server::handle_websocket_disconnect(uint16_t client_id) {
    vector<uint8_t> msg(4);
    uint16_t* msg16 = (uint16_t*)&msg[0];
    msg16[0] = opcode::OP_REMOVE_CLIENT;
    msg16[1] = client_id;
    backend_->write(0, move(msg));
}

void server::handle_websocket_request(uint16_t client_id, const std::string& msg) {
    vector<uint8_t> msg_u8(msg.begin(), msg.end());
    backend_->write(client_id, move(msg_u8));
}
