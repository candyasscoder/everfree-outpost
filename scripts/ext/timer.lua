local outpost_ffi = require('outpost_ffi')
local extra = require('outpost.extra')
local timer = require('outpost.timer')


local handlers = {}

local function mk_callback(sid)
    return function(t)
        local s = World.get():get_structure(sid)
        if s == nil then
            -- Structure ID is no longer valid.
            return
        end
        local template = s:template()

        if not rawequal(t, s:extra().pending_timer) then
            -- Structure ID no longer refers to the same structure.
            return
        end

        s:extra().pending_timer = nil
        s:set_has_save_hooks(false)

        local h = handlers[s:template()]
        if h == nil then
            return
        end
        h(s)
    end
end

function outpost_ffi.types.Structure.table.set_timer(s, delay)
    s:extra().pending_timer = timer.set_timer(delay, mk_callback(s:id()))
    s:set_has_save_hooks(true)
end

function outpost_ffi.types.Structure.table.cancel_timer(s)
    local t = s:extra().pending_timer
    if t == nil then
        return
    end
    s:extra().pending_timer = nil
    s:set_has_save_hooks(false)
    s:cancel()
end


function pre_unload(e, id)
    if e.pending_timer ~= nil then
        local t = e.pending_timer
        e.pending_timer = {
            when = t.when,
        }
        t:cancel()
    end
end

function post_load(e, id)
    if e.pending_timer ~= nil then
        local when = e.pending_timer.when
        e.pending_timer = timer.set_timer_at(when, mk_callback(id))
    end
end

extra.register_structure_hooks({ unload = pre_unload, load = post_load })


return {
    handler = handlers,
}
