#ifndef OUTPOST_WRAPPER_BACKEND_HPP
#define OUTPOST_WRAPPER_BACKEND_HPP

#include <boost/asio.hpp>
#include <vector>

#include "platform.hpp"


class server;

class backend {
    server& owner;
    const char* backend_path;
    platform::child_stream pipe_to;
    platform::child_stream pipe_from;

    struct header {
        uint16_t client_id;
        uint16_t data_len;
    };

    header header_buf;
    std::vector<uint8_t> msg_buf;

    bool suspended;
    std::vector<std::pair<uint16_t, std::vector<uint8_t>>> pending_msgs;

    void read_header();
    void read_data();
    void handle_message();
    void handle_shutdown();

public:
    backend(server& owner,
            boost::asio::io_service& ios,
            const char* backend_path);

    void start();

    void write(uint16_t client_id, std::vector<uint8_t> msg);

    void suspend();
    void resume();
};

#endif // OUTPOST_WRAPPER_BACKEND_HPP
