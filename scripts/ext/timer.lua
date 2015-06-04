local outpost_ffi = require('outpost_ffi')

local timer = require('outpost.timer')


local handlers = {}

local function mk_callback(s)
    return function(t)
        local template = s:template()
        if template == nil then
            -- Structure ID is no longer valid.
            return
        end

        if not rawequal(t, s:extra().pending_timer) then
            -- Structure ID no longer refers to the same structure.
            return
        end

        s:extra().pending_timer = nil

        local h = handlers[s:template()]
        if h == nil then
            return
        end
        h(s)
    end
end

function outpost_ffi.types.Structure.table.set_timer(s, delay)
    s:extra().pending_timer = timer.set_timer(delay, mk_callback(s))
end

function outpost_ffi.types.Structure.table.cancel_timer(s)
    local t = s:extra().pending_timer
    if t == nil then
        return
    end
    s:extra().pending_timer = nil
    s:cancel()
end


return {
    handler = handlers,
}
