#ifndef _WIN32
# include <signal.h>
# include <sys/types.h>
# include <sys/wait.h>
#else
// TODO
#endif

#include "server.hpp"
#include "signals.hpp"

using namespace std;
using namespace boost::asio;


signals::signals(server& owner, io_service& ios)
    : owner(owner),
      sig_set(ios) {
#ifndef _WIN32
    sig_set.add(SIGCHLD);
    /* TODO: catch these and do a clean shutdown
    sig_set.add(SIGTERM);
    sig_set.add(SIGINT);
    sig_set.add(SIGHUP);
    */
#else
    // TODO
#endif

    wait();
}

void signals::wait() {
    sig_set.async_wait(
        [this] (boost::system::error_code ec, int sig_num) {
            if (!ec) {
                handle_signal(sig_num);
                wait();
            } else {
                cerr << "error handling signal: " << ec << endl;
            }
        });
}

#ifndef _WIN32
void signals::handle_signal(int sig_num) {
    if (sig_num == SIGCHLD) {
        int status;
        pid_t pid = waitpid(-1, &status, WNOHANG);
        cerr << "child " << pid << " exited with status " << status << endl;
    }
}
#else
void signals::handle_signal(int sig_num) {
    // TODO
}
#endif
