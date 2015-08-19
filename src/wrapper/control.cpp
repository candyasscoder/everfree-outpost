#include <string>

#include "opcode.hpp"
#include "control.hpp"
#include "server.hpp"

using namespace std;
using namespace boost::asio;


void control::accept() {
    acceptor.async_accept(accepted_socket,
        [this] (boost::system::error_code ec) {
            if (!ec) {
                errors = 0;
                handle_accept();
                accept();
            } else {
                cerr << "error accepting control client: " << ec << endl;
                ++errors;
                assert(errors < 5);
            }
        });
}

control::control(server& owner, io_service& ios, platform::local_stream::endpoint addr)
    : owner(owner),
      acceptor(ios, addr),
      accepted_socket(ios),
      next_id(0),
      clients(),
      errors(0) {
    accept();
}

void control::closed(size_t id) {
    clients.erase(id);
}

void control::handle_accept() {
    size_t id = next_id++;
    clients.emplace(piecewise_construct,
            forward_as_tuple(id),
            forward_as_tuple(*this, id, move(accepted_socket)));
}

void control::handle_command(size_t id,
        vector<uint8_t>::const_iterator begin,
        vector<uint8_t>::const_iterator end) {
    string s(begin, end);
    if (s == "shutdown") {
        owner.handle_control_command(opcode::OP_SHUTDOWN);
    } else if (s == "restart_server") {
        owner.handle_control_command(opcode::OP_RESTART_SERVER);
    } else if (s == "restart_client") {
        owner.handle_control_command(opcode::OP_RESTART_CLIENT);
    } else if (s == "restart_both") {
        owner.handle_control_command(opcode::OP_RESTART_BOTH);
    } else {
        cerr << "unknown control command" << endl;
    }
}


void control_client::read() {
    size_t old_size = buf.size();
    buf.resize(old_size + 128);
    socket.async_read_some(buffer(&buf[old_size], buf.size() - old_size),
        [this, old_size] (boost::system::error_code ec, size_t count) {
            if (!ec) {
                buf.resize(old_size + count);
                handle_read();
                if (buf.size() < 128) {
                    read();
                } else {
                    cerr << "control client " << id << " disconnected: message too long" << endl;
                    close();
                }
            } else {
                cerr << "control client " << id << " disconnected: " << ec << endl;
                close();
            }
        });
}

void control_client::handle_read() {
    // Consume as many commands as possible.  Usually there will be at most
    // one command in the buffer.
    while (true) {
        auto eol = find(buf.begin(), buf.end(), '\n');
        if (eol == buf.end()) {
            return;
        }
        owner.handle_command(id, buf.begin(), eol);

        size_t new_len = buf.end() - eol - 1;
        copy(eol + 1, buf.end(), buf.begin());
        buf.resize(new_len);
    }
}

void control_client::close() {
    socket.close();
    owner.closed(id);
}

control_client::control_client(control& owner, size_t id, platform::local_stream::socket socket)
    : owner(owner),
      id(id),
      socket(move(socket)) {
    read();
}
