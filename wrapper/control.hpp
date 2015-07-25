#ifndef OUTPOST_WRAPPER_CONTROL_HPP
#define OUTPOST_WRAPPER_CONTROL_HPP

#include <boost/asio.hpp>
#include <map>
#include <vector>

#include "platform.hpp"


class server;
class control_client;

class control {
    server& owner;
    platform::local_stream::acceptor acceptor;
    platform::local_stream::socket accepted_socket;
    size_t next_id;
    std::map<size_t, control_client> clients;
    int errors;

    void accept();
    void handle_accept();

public:
    control(server& owner, boost::asio::io_service& ios, platform::local_stream::endpoint addr);

    void closed(size_t id);

    void handle_command(size_t id,
            std::vector<uint8_t>::const_iterator begin,
            std::vector<uint8_t>::const_iterator end);
};

class control_client {
    control& owner;
    size_t id;
    platform::local_stream::socket socket;
    std::vector<uint8_t> buf;

    void read();
    void handle_read();
    void close();

public:
    control_client(control& owner, size_t id, platform::local_stream::socket socket);
    control_client(const control_client&) = delete;
};

#endif // OUTPOST_WRAPPER_CONTROL_HPP
