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
                cerr << "error reading header from backend: " << ec << endl;
                assert(0);
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

backend::backend(server& owner, io_service& ios, int fd_to, int fd_from)
  : owner(owner),
    pipe_to(ios, fd_to), pipe_from(ios, fd_from) {
    read_header();
}

void backend::write(uint16_t client_id, vector<uint8_t> msg) {
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
