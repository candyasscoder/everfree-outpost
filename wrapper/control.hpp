#ifndef OUTPOST_WRAPPER_CONTROL_HPP
#define OUTPOST_WRAPPER_CONTROL_HPP

#include <boost/asio.hpp>
#include <map>
#include <vector>


class server;
class control_client;

class control {
    server& owner;
    boost::asio::local::stream_protocol::acceptor acceptor;
    boost::asio::local::stream_protocol::socket accepted_socket;
    size_t next_id;
    std::map<size_t, control_client> clients;
    int errors;

    void accept();
    void handle_accept();

public:
    control(server& owner, boost::asio::io_service& ios, const char* path);

    void closed(size_t id);

    void handle_command(size_t id,
            std::vector<uint8_t>::const_iterator begin,
            std::vector<uint8_t>::const_iterator end);
};

class control_client {
    control& owner;
    size_t id;
    boost::asio::local::stream_protocol::socket socket;
    std::vector<uint8_t> buf;

    void read();
    void handle_read();
    void close();

public:
    control_client(control& owner, size_t id, boost::asio::local::stream_protocol::socket socket);
    control_client(const control_client&) = delete;
};

#endif // OUTPOST_WRAPPER_CONTROL_HPP
