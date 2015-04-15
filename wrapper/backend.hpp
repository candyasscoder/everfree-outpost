#include <boost/asio.hpp>
#include <vector>


class server;

class backend {
    server& owner;
    boost::asio::posix::stream_descriptor pipe_to;
    boost::asio::posix::stream_descriptor pipe_from;

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
    backend(server& owner, boost::asio::io_service& ios, int fd_to, int fd_from);

    void write(uint16_t client_id, std::vector<uint8_t> msg);
};
