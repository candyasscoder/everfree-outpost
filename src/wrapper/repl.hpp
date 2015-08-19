#ifndef OUTPOST_WRAPPER_REPL_HPP
#define OUTPOST_WRAPPER_REPL_HPP

#include <boost/asio.hpp>
#include <map>
#include <vector>

#include "platform.hpp"


class server;
class repl_client;

class repl {
    server& owner;
    platform::local_stream::acceptor acceptor;
    platform::local_stream::socket accepted_socket;
    size_t next_id;
    std::map<size_t, repl_client> clients;
    uint16_t next_cookie;
    std::map<uint16_t, size_t> pending;
    int errors;

    void accept();
    void handle_accept();

public:
    repl(server& owner, boost::asio::io_service& ios, platform::local_stream::endpoint addr);

    void closed(size_t id);

    void handle_command(size_t id,
            std::vector<uint8_t>::const_iterator begin,
            std::vector<uint8_t>::const_iterator end);
    void handle_response(
            std::vector<uint8_t>::const_iterator begin,
            std::vector<uint8_t>::const_iterator end);
};

class repl_client {
    repl& owner;
    size_t id;
    platform::local_stream::socket socket;
    std::vector<uint8_t> buf;

    void read();
    void handle_read();
    void close();

public:
    repl_client(repl& owner, size_t id, platform::local_stream::socket socket);
    repl_client(const repl_client&) = delete;

    bool operator <(const repl_client& other) const;

    void handle_response(
            std::vector<uint8_t>::const_iterator begin,
            std::vector<uint8_t>::const_iterator end);
};

#endif // OUTPOST_WRAPPER_REPL_HPP
