#include <boost/asio.hpp>
#include <map>
#include <vector>


class server;
class repl_client;

class repl {
    server& owner;
    boost::asio::local::stream_protocol::acceptor acceptor;
    boost::asio::local::stream_protocol::socket accepted_socket;
    size_t next_id;
    std::map<size_t, repl_client> clients;
    uint16_t next_cookie;
    std::map<uint16_t, size_t> pending;
    int errors;

    void accept();
    void handle_accept();

public:
    repl(server& owner, boost::asio::io_service& ios, const char* path);

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
    boost::asio::local::stream_protocol::socket socket;
    std::vector<uint8_t> buf;

    void read();
    void handle_read();
    void close();

public:
    repl_client(repl& owner, size_t id, boost::asio::local::stream_protocol::socket socket);
    repl_client(const repl_client&) = delete;

    bool operator <(const repl_client& other) const;

    void handle_response(
            std::vector<uint8_t>::const_iterator begin,
            std::vector<uint8_t>::const_iterator end);
};
