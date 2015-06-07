local outpost_ffi = require('outpost_ffi')


local timers = {}
local free_slots = {}

local function clear_slot(slot)
    timers[slot] = nil
    free_slots[#free_slots + 1] = slot
end


local timer_table = {
    cancel = function(t)
        if t.slot == nil then
            return
        end

        local slot = t.slot
        t.slot = nil

        t.timer:cancel()
        t.timer = nil
        clear_slot(slot)
    end,
}

local timer_metatable = {
    __index = timer_table,
}


local function set_timer_at(time, cb)
    local slot
    if #free_slots == 0 then
        slot = #timers + 1
    else
        slot = free_slots[#free_slots]
        free_slots[#free_slots] = nil
    end

    local t = {
        timer = Timer.schedule(time, slot),
        when = time,
        callback = cb,
        slot = slot,
    }
    setmetatable(t, timer_metatable)
    timers[slot] = t

    return t
end

local function set_timer(delay, cb)
    return set_timer_at(Time.now() + delay, cb)
end

function outpost_ffi.callbacks.timeout(slot)
    local t = timers[slot]
    clear_slot(slot)

    t.slot = nil
    t.timer = nil
    t:callback()
end


return {
    set_timer = set_timer,
    set_timer_at = set_timer_at,
}
