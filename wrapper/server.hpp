#ifndef OUTPOST_WRAPPER_SERVER_HPP
#define OUTPOST_WRAPPER_SERVER_HPP

#include <boost/asio.hpp>
#include <memory>
#include <string>
#include <vector>

#include "backend.hpp"
#include "repl.hpp"
#include "websocket.hpp"


class server {
    std::unique_ptr<backend> backend_;
    std::unique_ptr<repl> repl_;
    std::unique_ptr<websocket> websocket_;

public:
    server(boost::asio::io_service& ios, int to_backend, int from_backend);

    void handle_backend_response(uint16_t client_id, std::vector<uint8_t> msg);
    void handle_repl_command(std::vector<uint8_t> command);

    void handle_websocket_connect(uint16_t client_id);
    void handle_websocket_disconnect(uint16_t client_id);
    void handle_websocket_request(uint16_t client_id, const std::string& msg);
};

#endif // OUTPOST_WRAPPER_SERVER_HPP
