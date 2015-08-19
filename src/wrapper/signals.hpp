#ifndef OUTPOST_WRAPPER_SIGNALS_HPP
#define OUTPOST_WRAPPER_SIGNALS_HPP

#include <boost/asio.hpp>


class server;

class signals {
    server& owner;
    boost::asio::signal_set sig_set;

    void wait();
    void handle_signal(int sig_num);

public:
    signals(server& owner, boost::asio::io_service& ios);
};

#endif // OUTPOST_WRAPPER_SIGNALS_HPP
