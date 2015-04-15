#ifndef OUTPOST_WRAPPER_OPCODES_HPP
#define OUTPOST_WRAPPER_OPCODES_HPP

enum opcode {
    OP_ADD_CLIENT =         0xff00,
    OP_REMOVE_CLIENT =      0xff01,
    OP_CLIENT_REMOVED =     0xff02,
    OP_REPL_COMMAND =       0xff03,
    OP_REPL_RESULT =        0xff04,
    OP_SHUTDOWN =           0xff05,
    OP_RESTART =            0xff06,
};

#endif // OUTPOST_WRAPPER_OPCODES_HPP
