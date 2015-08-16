local tools = require('outpost.lib.tools')
local util = require('core.util')
local ward = require('outpost.lib.ward')

local function handler(c, s, inv)
    if not ward.check(c, s:pos()) then
        return
    end
    s:destroy()
    inv:update('stone', 2)
end

tools.handler.pick['cave_junk/0'] = handler
tools.handler.pick['cave_junk/1'] = handler
tools.handler.pick['cave_junk/2'] = handler
