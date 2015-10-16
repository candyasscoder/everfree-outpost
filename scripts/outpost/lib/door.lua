local action = require('core.action')
local tools = require('outpost.lib.tools')
local structure_items = require('outpost.lib.structure_items')
local timer = require('outpost.ext.timer')

function make_door(item, base, tool)
    local t_open = base .. '/open'
    local t_closed = base .. '/closed'
    local t_opening = base .. '/opening'
    local t_closing = base .. '/closing'

    action.use[t_closed] = function(c, s)
        s:replace(t_opening)
        s:set_timer(250)
    end

    timer.handler[t_opening] = function(s)
        s:replace(t_open)
        s:set_timer(3000)
    end

    timer.handler[t_open] = function(s)
        s:replace(t_closing)
        s:set_timer(250)
    end

    timer.handler[t_closing] = function(s)
        s:replace(t_closed)
    end

    local function take(c, s, inv)
        return structure_items.use_structure(c, s, item)
    end
    tools.handler[tool][t_open] = take
    tools.handler[tool][t_closed] = take
    tools.handler[tool][t_opening] = take
    tools.handler[tool][t_closing] = take
end


-- TODO: integrate `register_anims` and `make_door` variants
local BASE_MAP = {}
local DELAY_MAP = {}

local function register_anims(base, delay)
    timer.handler[base .. '/opening'] = function(s)
        s:replace(base .. '/open')
    end

    timer.handler[base .. '/closing'] = function(s)
        s:replace(base .. '/closed')
    end

    BASE_MAP[base .. '/open'] = base
    BASE_MAP[base .. '/opening'] = base
    BASE_MAP[base .. '/closed'] = base
    BASE_MAP[base .. '/closing'] = base
    DELAY_MAP[base] = delay
end

local function open(s)
    local base = BASE_MAP[s:template()]
    local delay = DELAY_MAP[base]
    s:replace(base .. '/opening')
    s:set_timer(delay)
end

local function close(s)
    local base = BASE_MAP[s:template()]
    local delay = DELAY_MAP[base]
    s:replace(base .. '/closing')
    s:set_timer(delay)
end


return {
    make_door = make_door,

    register_anims = register_anims,
    open = open,
    close = close,
}
