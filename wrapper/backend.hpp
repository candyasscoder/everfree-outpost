#ifndef OUTPOST_WRAPPER_BACKEND_HPP
#define OUTPOST_WRAPPER_BACKEND_HPP

#include <boost/asio.hpp>
#include <vector>

#include "config.hpp"


class server;

class backend {
    server& owner;
    platform::child_stream pipe_to;
    platform::child_stream pipe_from;

    struct header {
        uint16_t client_id;
        uint16_t data_len;
    };

    header header_buf;
    std::vector<uint8_t> msg_buf;

    void read_header();
    void read_data();
    void handle_message();

public:
    backend(server& owner,
            boost::asio::io_service& ios,
            platform::child_stream::native_handle_type fd_to,
            platform::child_stream::native_handle_type fd_from);

    void write(uint16_t client_id, std::vector<uint8_t> msg);
};

#endif // OUTPOST_WRAPPER_BACKEND_HPP
