#include <boost/asio.hpp>
#include <memory>
#include <vector>

#include "backend.hpp"
#include "repl.hpp"


class server {
    std::unique_ptr<backend> backend;
    std::unique_ptr<repl> repl;

public:
    server(boost::asio::io_service& ios, int to_backend, int from_backend);

    void handle_backend_response(uint16_t client_id, std::vector<uint8_t> msg);
    void handle_repl_command(std::vector<uint8_t> command);
};
