local tools = require('outpost.lib.tools')
local ward = require('outpost.lib.ward')

tools.handler.pick['ore_vein/copper'] = function(c, s, inv)
    if not ward.check(c, s:pos()) then
        return
    end
    s:destroy()
    inv:update('ore/copper', 1)
end
