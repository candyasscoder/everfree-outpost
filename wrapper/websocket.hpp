#ifndef OUTPOST_WRAPPER_WEBSOCKET_HPP
#define OUTPOST_WRAPPER_WEBSOCKET_HPP

#include <bitset>
#include <boost/asio.hpp>
#include <map>
#include <memory>
#include <vector>
#include <websocketpp/concurrency/none.hpp>
#include <websocketpp/config/asio_no_tls.hpp>
#include <websocketpp/logger/stub.hpp>
#include <websocketpp/server.hpp>


struct ws_config : public websocketpp::config::asio {
    typedef ws_config type;
    typedef asio base;

    typedef websocketpp::concurrency::none concurrency_type;

    typedef base::request_type request_type;
    typedef base::response_type response_type;

    typedef base::message_type message_type;
    typedef base::con_msg_manager_type con_msg_manager_type;
    typedef base::endpoint_msg_manager_type endpoint_msg_manager_type;

    typedef websocketpp::log::basic<concurrency_type,
        websocketpp::log::elevel> elog_type;
    typedef websocketpp::log::stub alog_type;

    typedef base::rng_type rng_type;

    struct transport_config : public base::transport_config {
        typedef type::base::transport_config base;

        typedef type::concurrency_type concurrency_type;
        typedef type::alog_type alog_type;
        typedef type::elog_type elog_type;
        typedef base::request_type request_type;
        typedef base::response_type response_type;
        typedef base::socket_type socket_type;
    };

    typedef websocketpp::transport::asio::endpoint<transport_config>
        transport_type;
};


class server;

class websocket {
    server& owner;
    typedef websocketpp::server<ws_config> ws_server_asio;
    ws_server_asio ws_server;

    struct client_data {
        uint16_t id;
        bool backend_connected;
        bool client_connected;

        client_data() : backend_connected(true), client_connected(true) {}

        bool dead() const {
            return !backend_connected && !client_connected;
        }
    };

    uint16_t next_id;
    std::map<uint16_t, websocketpp::connection_hdl> id_to_client;
    std::map<websocketpp::connection_hdl, client_data,
        std::owner_less<websocketpp::connection_hdl>> clients;

    void handle_open(websocketpp::connection_hdl conn);
    void handle_message(websocketpp::connection_hdl conn,
            ws_server_asio::message_ptr msg);
    void handle_close(websocketpp::connection_hdl conn);

public:
    websocket(server& owner, boost::asio::io_service& ios);

    void send_message(uint16_t client_id, std::vector<uint8_t> msg);
    void handle_client_removed(uint16_t client_id);
};

#endif // OUTPOST_WRAPPER_WEBSOCKET_HPP
