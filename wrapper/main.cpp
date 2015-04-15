#include <algorithm>
#include <array>
#include <boost/asio.hpp>
#include <iostream>
#include <map>
#include <memory>
#include <set>
#include <vector>
//#include <websocketpp/config/asio_no_tls.hpp>
//#include <websocketpp/server.hpp>

using namespace std;
using namespace boost::asio;
using boost::system::error_code;


class backend;
class repl;


class server {
    unique_ptr<backend> backend;
    unique_ptr<repl> repl;

public:
    server(io_service& ios, int to_backend, int from_backend);

    void handle_backend_response(uint16_t client_id, vector<uint8_t> msg);
    void handle_repl_command(vector<uint8_t> command);
};


class backend {
    server& owner;
    posix::stream_descriptor pipe_to;
    posix::stream_descriptor pipe_from;

    struct header {
        uint16_t client_id;
        uint16_t data_len;
    };

    header header_buf;
    std::vector<uint8_t> msg_buf;

    void read_header() {
        async_read(pipe_from, buffer(&header_buf, sizeof(header)),
            [this] (error_code ec, size_t len) {
                if (!ec) {
                    read_data();
                } else {
                    cerr << "error reading header from backend: " << ec << endl;
                    assert(0);
                }
            });
    }

    void read_data() {
        msg_buf.resize((size_t)header_buf.data_len);
        async_read(pipe_from, buffer(msg_buf),
            [this] (error_code ec, size_t len) {
                if (!ec) {
                    handle_message();
                    read_header();
                } else {
                    cerr << "error reading data from backend: " << ec << endl;
                    assert(0);
                }
            });
    }

    void handle_message() {
        owner.handle_backend_response(header_buf.client_id, move(msg_buf));
    }

public:
    backend(server& owner, io_service& ios, int fd_to, int fd_from)
      : owner(owner),
        pipe_to(ios, fd_to), pipe_from(ios, fd_from) {
        read_header();
    }

    void write(uint16_t client_id, vector<uint8_t> msg) {
        auto header_ptr = make_shared<header>();
        header_ptr->client_id = client_id;
        assert(msg.size() <= UINT16_MAX);
        header_ptr->data_len = msg.size();

        auto msg_ptr = make_shared<vector<uint8_t>>(move(msg));

        array<mutable_buffer, 2> bufs {{
            { &*header_ptr, sizeof(*header_ptr) },
            { &(*msg_ptr)[0], msg_ptr->size() },
        }};

        async_write(pipe_to, bufs,
            [header_ptr, msg_ptr] (error_code ec, size_t len) {
                if (ec) {
                    cerr << "error writing to backend: " << ec << endl;
                    assert(0);
                }
            });
    }
};


class repl_client;

class repl {
    server& owner;
    local::stream_protocol::acceptor acceptor;
    local::stream_protocol::socket accepted_socket;
    size_t next_id;
    map<size_t, repl_client> clients;
    uint16_t next_cookie;
    map<uint16_t, size_t> pending;
    int errors;

    void accept() {
        acceptor.async_accept(accepted_socket,
            [this] (error_code ec) {
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

    void handle_accept();

public:
    repl(server& owner, io_service& ios, const char* path)
      : owner(owner),
        acceptor(ios, local::stream_protocol::endpoint(path)),
        accepted_socket(ios),
        next_id(0),
        clients(),
        next_cookie(0),
        pending(),
        errors(0) {
        accept();
    }

    void closed(size_t id) {
        clients.erase(id);
    }

    void handle_command(size_t id,
            vector<uint8_t>::const_iterator begin,
            vector<uint8_t>::const_iterator end) {
        vector<uint8_t> buf;
        buf.reserve(end - begin + 4);
        buf.resize(4);

        uint16_t cookie = next_cookie++;
        *(uint16_t*)&buf[0] = 0xff03;
        *(uint16_t*)&buf[2] = cookie;
        buf.insert(buf.end(), begin, end);
        owner.handle_repl_command(move(buf));

        pending.insert(make_pair(cookie, id));
    }

    void handle_response(
            vector<uint8_t>::const_iterator begin,
            vector<uint8_t>::const_iterator end);
};

class repl_client {
    repl& owner;
    size_t id;
    local::stream_protocol::socket socket;
    vector<uint8_t> buf;

    void read() {
        size_t old_size = buf.size();
        buf.resize(old_size + 1024);
        socket.async_read_some(buffer(&buf[old_size], buf.size() - old_size),
            [this, old_size] (error_code ec, size_t count) {
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

    void handle_read() {
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

    void close() {
        socket.close();
        owner.closed(id);
    }

public:
    repl_client(repl& owner, size_t id, local::stream_protocol::socket socket)
      : owner(owner),
        id(id),
        socket(move(socket)) {
        read();
    }

    repl_client(const repl_client&) = delete;

    bool operator <(const repl_client& other) const {
        return id < other.id;
    }

    void handle_response(
            vector<uint8_t>::const_iterator begin,
            vector<uint8_t>::const_iterator end) {
        auto msg_ptr = make_shared<vector<uint8_t>>(begin, end);

        async_write(socket, buffer(*msg_ptr),
            [msg_ptr, this] (error_code ec, size_t len) {
                if (ec) {
                    cerr << "error writing to client: " << ec << endl;
                    close();
                }
            });
    }
};

void repl::handle_accept() {
    size_t id = next_id++;
    clients.emplace(piecewise_construct,
            forward_as_tuple(id),
            forward_as_tuple(*this, id, move(accepted_socket)));
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


server::server(io_service& ios, int to_backend, int from_backend)
  : backend(new class backend(*this, ios, to_backend, from_backend)),
    repl(new class repl(*this, ios, "repl")) {
}

void server::handle_backend_response(uint16_t client_id, vector<uint8_t> msg) {
    if (client_id != 0) {
        return;
    }

    assert(msg.size() >= 2 && "control message has no opcode");
    uint16_t opcode = *(const uint16_t*)&msg[0];
    if (opcode == 0xff04) {
        repl->handle_response(msg.begin() + 2, msg.end());
    }
}

void server::handle_repl_command(vector<uint8_t> command) {
    backend->write(0, move(command));
}



int main() {
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
        execl("dist/bin/backend", "dist/bin/backend", "dist", NULL);
    }

    io_service ios;

    server s(ios, to_backend[1], from_backend[0]);

    ios.run();
}
