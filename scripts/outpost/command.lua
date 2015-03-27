local outpost_ffi = require('outpost_ffi')

local command_handlers = {}
function outpost_ffi.callbacks.command(client, msg)
    if msg:sub(1, 1) ~= '/' then
        return
    end

    local index = msg:find(' ')
    local command = nil
    local args = ''
    if index == nil then
        command = msg:sub(2)
    else
        command = msg:sub(2, index - 1)
        args = msg:sub(index + 1)
    end

    local handler = command_handlers[command]
    if handler == nil then
        client:send_message('unknown command: ' .. command)
        return
    end

    handler(client, args)
end

local command_help = {}
function command_handlers.help(client, args)
    while args:sub(1, 1) == ' ' do
        args = args:sub(2)
    end
    if args:sub(1, 1) == '/' then
        args = args:sub(2)
    end

    if args == '' then
        names = {}
        for k, v in pairs(command_help) do
            names[#names + 1] = k
        end
        table.sort(names)

        names_str = ''
        for i, name in ipairs(names) do
            names_str = names_str .. ' ' .. name
        end

        client:send_message('Commands:' .. names_str)
        client:send_message('Use "/help <command>" for more info')
    else
        name = args

        help = command_help[args]
        if help == nil then
            if command_handlers[name] == nil then
                client:send_message('No such command: /' .. name)
            else
                client:send_message('No help available for /' .. name)
            end
            return
        end

        if type(help) == 'string' then
            client:send_message(help)
        else
            for i, line in ipairs(help) do
                client:send_message(line)
            end
        end
    end
end

command_help.help = '/help <command>: Show detailed info about <command>'

return {
    handler = command_handlers,
    help = command_help
}
