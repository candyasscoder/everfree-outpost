local action = require('core.action')
local structure_items = require('outpost.lib.structure_items')
local timer = require('outpost.ext.timer')
local tools = require('outpost.lib.tools')
local util = require('core.util')
local ward = require('outpost.lib.ward')

local TRIBE_TABLE = {
    [0x00] = 'E',
    [0x40] = 'P',
    [0x80] = 'U',
    [0xc0] = 'A',
}

local function get_tribe(e)
    return TRIBE_TABLE[e:get_appearance_bits(0xc0)]
end


local function set_crop_timer(s, steps)
    local when = s:extra().start_time + steps * s:extra().grow_time
    s:set_timer_at(when)
end


local function mk_crop(name)
    action.use_item[name] = function(c, inv)
        local pos = util.hit_tile(c:pawn())
        if not c:pawn():plane():get_block(pos):startswith('farmland/') then
            return
        end
        local s = structure_items.use_item(c, inv, name, name .. '/0')
        if s ~= nil then
            local tribe = get_tribe(c:pawn())
            local grow_mins
            if tribe == 'E' or tribe == 'A' then
                grow_mins = 6
            else
                grow_mins = 8
            end
            s:extra().grow_time = (grow_mins + math.random() - 0.5) * 60 * 1000
            s:extra().start_time = Time.now()
            set_crop_timer(s, 1)
        end
    end

    timer.handler[name .. '/0'] = function(s)
        s:replace(name .. '/1')
        set_crop_timer(s, 2)
    end

    timer.handler[name .. '/1'] = function(s)
        s:replace(name .. '/2')
        set_crop_timer(s, 3)
    end

    timer.handler[name .. '/2'] = function(s)
        s:replace(name .. '/3')
    end

    local function destroy(c, s)
        if not ward.check(c, s:pos()) then return end
        s:destroy()
    end
    action.use[name .. '/0'] = destroy
    action.use[name .. '/1'] = destroy
    action.use[name .. '/2'] = destroy
    action.use[name .. '/3'] = function(c, s)
        if not ward.check(c, s:pos()) then return end
        c:pawn():inventory('main'):update(name, math.floor(math.random() * 3 + 1))
        s:destroy()
    end
end

mk_crop('tomato')
mk_crop('potato')
mk_crop('carrot')
mk_crop('artichoke')
mk_crop('pepper')
mk_crop('cucumber')
mk_crop('corn')
