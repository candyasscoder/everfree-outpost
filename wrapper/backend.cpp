#include "backend.hpp"
#include "server.hpp"

using namespace std;
using namespace boost::asio;


void backend::read_header() {
    async_read(pipe_from, buffer(&header_buf, sizeof(header)),
        [this] (boost::system::error_code ec, size_t len) {
            if (!ec) {
                read_data();
            } else {
                handle_shutdown();
            }
        });
}

void backend::read_data() {
    msg_buf.resize((size_t)header_buf.data_len);
    async_read(pipe_from, buffer(msg_buf),
        [this] (boost::system::error_code ec, size_t len) {
            if (!ec) {
                handle_message();
                read_header();
            } else {
                cerr << "error reading data from backend: " << ec << endl;
                assert(0);
            }
        });
}

void backend::handle_message() {
    owner.handle_backend_response(header_buf.client_id, move(msg_buf));
}

void backend::handle_shutdown() {
    owner.handle_backend_shutdown();
}

backend::backend(server& owner,
                 io_service& ios,
                 const char* backend_path)
  : owner(owner), backend_path(backend_path), pipe_from(ios), pipe_to(ios) {
}

void backend::start() {
    auto fds = platform::spawn_backend(backend_path);
    pipe_from = platform::child_stream(pipe_from.get_io_service(), fds.first);
    pipe_to = platform::child_stream(pipe_to.get_io_service(), fds.second);
    read_header();
}

void backend::write(uint16_t client_id, vector<uint8_t> msg) {
    if (suspended) {
        pending_msgs.emplace_back(client_id, move(msg));
        return;
    }

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
        [header_ptr, msg_ptr] (boost::system::error_code ec, size_t len) {
            if (ec) {
                cerr << "error writing to backend: " << ec << endl;
                assert(0);
            }
        });
}

void backend::suspend() {
    suspended = true;
}

void backend::resume() {
    suspended = false;
    for (auto&& p : pending_msgs) {
        write(p.first, move(p.second));
    }
    pending_msgs.clear();
}
