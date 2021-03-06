#include "opcode.hpp"
#include "repl.hpp"
#include "server.hpp"

using namespace std;
using namespace boost::asio;


void repl::accept() {
    acceptor.async_accept(accepted_socket,
        [this] (boost::system::error_code ec) {
            if (!ec) {
                errors = 0;
                handle_accept();
                accept();
            } else {
                cerr << "error accepting repl client: " << ec << endl;
                ++errors;
                assert(errors < 5);
            }
        });
}

repl::repl(server& owner, io_service& ios, platform::local_stream::endpoint addr)
  : owner(owner),
    acceptor(ios, addr),
    accepted_socket(ios),
    next_id(0),
    clients(),
    next_cookie(0),
    pending(),
    errors(0) {
    accept();
}

void repl::closed(size_t id) {
    clients.erase(id);
}

void repl::handle_accept() {
    size_t id = next_id++;
    clients.emplace(piecewise_construct,
            forward_as_tuple(id),
            forward_as_tuple(*this, id, move(accepted_socket)));
}

void repl::handle_command(size_t id,
        vector<uint8_t>::const_iterator begin,
        vector<uint8_t>::const_iterator end) {
    vector<uint8_t> buf;
    buf.reserve(end - begin + 6);
    buf.resize(6);

    uint16_t cookie = next_cookie++;
    *(uint16_t*)&buf[0] = opcode::OP_REPL_COMMAND;
    *(uint16_t*)&buf[2] = cookie;
    *(uint16_t*)&buf[4] = end - begin;
    buf.insert(buf.end(), begin, end);
    owner.handle_repl_command(move(buf));

    pending.insert(make_pair(cookie, id));
}

void repl::handle_response(
        vector<uint8_t>::const_iterator begin,
        vector<uint8_t>::const_iterator end) {
    if (end - begin < 2) {
        cerr << "ReplReply has no cookie" << endl;
        return;
    }
    uint16_t cookie = *(uint16_t*)&*begin;

    auto pending_iter = pending.find(cookie);
    if (pending_iter == pending.end()) {
        cerr << "ReplReply has invalid cookie: " << cookie << endl;
        return;
    }
    size_t client_id = pending_iter->second;

    auto client_iter = clients.find(client_id);
    if (client_iter == clients.end()) {
        cerr << "ReplReply cookie " << cookie << " refers to bad client: " << client_id << endl;
        return;
    }
    client_iter->second.handle_response(begin + 2, end);
}


void repl_client::read() {
    size_t old_size = buf.size();
    buf.resize(old_size + 1024);
    socket.async_read_some(buffer(&buf[old_size], buf.size() - old_size),
        [this, old_size] (boost::system::error_code ec, size_t count) {
            if (!ec) {
                buf.resize(old_size + count);
                handle_read();
                if (buf.size() < UINT16_MAX) {
                    read();
                } else {
                    cerr << "repl client " << id << " disconnected: message too long" << endl;
                    close();
                }
            } else {
                cerr << "repl client " << id << " disconnected: " << ec << endl;
                close();
            }
        });
}

void repl_client::handle_read() {
    // Consume as many commands as possible.  Usually there will be at most
    // one command in the buffer.
    while (true) {
        auto eol = find(buf.begin(), buf.end(), '\n');
        if (eol == buf.end()) {
            return;
        }

        if (eol - buf.begin() == 1 && buf[0] == '{') {
            // Look for a line containing only a closing brace.
            auto first_eol = eol;
            auto prev_eol = eol;
            while (true) {
                prev_eol = eol;
                eol = find(eol + 1, buf.end(), '\n');
                if (eol == buf.end()) {
                    // Ending brace isn't in the buffer yet.
                    return;
                }
                if (eol - prev_eol == 2 && *(eol - 1) == '}') {
                    break;
                }
            }

            owner.handle_command(id, first_eol + 1, prev_eol + 1);
        } else {
            owner.handle_command(id, buf.begin(), eol);
        }

        size_t new_len = buf.end() - eol - 1;
        copy(eol + 1, buf.end(), buf.begin());
        buf.resize(new_len);
    }
}

void repl_client::close() {
    socket.close();
    owner.closed(id);
}

repl_client::repl_client(repl& owner, size_t id, platform::local_stream::socket socket)
  : owner(owner),
    id(id),
    socket(move(socket)) {
    read();
}

bool repl_client::operator <(const repl_client& other) const {
    return id < other.id;
}

void repl_client::handle_response(
        vector<uint8_t>::const_iterator begin,
        vector<uint8_t>::const_iterator end) {
    uint16_t len = *(const uint16_t*)&*begin;
    assert(len <= end - begin - 2 &&
            "not enough bytes in repl response");
    auto msg_ptr = make_shared<vector<uint8_t>>(begin + 2, end);

    async_write(socket, buffer(*msg_ptr),
        [msg_ptr, this] (boost::system::error_code ec, size_t len) {
            if (ec) {
                cerr << "error writing to client: " << ec << endl;
                close();
            }
        });
}
