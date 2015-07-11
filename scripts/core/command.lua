local outpost_ffi = require('outpost_ffi')

local command_handlers = {}
local super_command_handlers = {}
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

    local handler = nil
    if client:extra().superuser then
        handler = super_command_handlers[command]
        if handler == nil then
            handler = command_handlers[command]
        end
    else
        handler = command_handlers[command]
    end

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
        for k, v in pairs(command_handlers) do
            names[#names + 1] = k
        end
        if client:extra().superuser then
            for k, v in pairs(super_command_handlers) do
                names[#names + 1] = k
            end
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

        if command_handlers[name] == nil and
                (not client:extra().superuser or super_command_handlers[name] == nil) then
            client:send_message('No such command: /' .. name)
            return
        end

        help = command_help[args]
        if help == nil then
            client:send_message('No help available for /' .. name)
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
    su_handler = super_command_handlers,
    help = command_help
}
