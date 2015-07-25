#ifndef OUTPOST_WRAPPER_CONFIG_H
#define OUTPOST_WRAPPER_CONFIG_H

#include <utility>
#include <boost/asio.hpp>

namespace platform {
#ifndef _WIN32
    typedef boost::asio::local::stream_protocol local_stream;
    typedef boost::asio::posix::stream_descriptor child_stream;
#else
    typedef boost::asio::ip::tcp local_stream;
    typedef boost::asio::windows::stream_handle child_stream;
#endif

    std::pair<child_stream::native_handle_type, child_stream::native_handle_type>
        spawn_backend(const char* path);
}

#endif // OUTPOST_WRAPPER_CONFIG_H
