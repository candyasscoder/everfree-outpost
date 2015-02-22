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

return { handler = command_handlers }
