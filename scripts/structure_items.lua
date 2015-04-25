local action = require('outpost.action')
local util = require('outpost.util')
local ward = require('ward')
local mallet = require('mallet')

local function place_structure(world, inv, plane, pos, item_name, template_name)
    if inv:count(item_name) == 0 then
        return
    end

    s, err = world:create_structure(plane, pos, template_name)

    if s ~= nil then
        s:attach_to_chunk()
        inv:update(item_name, -1)
    else
        print('error placing structure ' .. template_name .. ': ' .. tostring(err))
    end
    return s
end

local function take_structure(s, inv, item_name)
    if inv:count(item_name) == 255 then
        return
    end

    err = s:destroy()
    if err == nil then
        inv:update(item_name, 1)
    end
    return err == nil
end

local function use_item(c, inv, item_name, template_name)
    local pawn = c:pawn()
    local plane = pawn:plane()
    local pos = util.hit_tile(c:pawn())

    if not ward.check(c, pos) then
        return
    end

    return place_structure(c:world(), inv, plane, pos, item_name, template_name)
end

local function use_structure(c, s, item_name)
    if not ward.check(c, s:pos()) then
        return
    end
    return take_structure(s, c:pawn():inventory('main'), item_name)
end

local function add_structure_item(item_name, template_name)
    if template_name == nil then
        template_name = item_name
    end

    action.use_item[item_name] = function(c, inv)
        use_item(c, inv, item_name, template_name)
    end

    action.use[template_name] = function(c, s)
        use_structure(c, s, item_name)
    end
end

add_structure_item('fence', 'fence/edge/horiz')
add_structure_item('fence_tee', 'fence/tee/e')
add_structure_item('fence_post', 'fence/end/fancy/e')

add_structure_item('house_wall/side', 'house_wall/edge/horiz/in')
add_structure_item('house_wall/corner', 'house_wall/corner/nw/in')
add_structure_item('house_wall/tee', 'house_wall/tee/n/in')
add_structure_item('house_wall/cross', 'house_wall/cross/in_in')
add_structure_item('house_door', 'house_wall/door/in')

add_structure_item('house_floor', 'wood_floor/center/v0')
add_structure_item('road', 'road/center/v0')
add_structure_item('bed')
add_structure_item('table')
add_structure_item('statue', 'statue/e')


local function mallet_cycle(base, xs)
    for i = 1, #xs - 1 do
        mallet.replacements[base .. xs[i]] = base .. xs[i + 1]
        action.use[base .. xs[i + 1]] = action.use[base .. xs[i]]
    end
    mallet.replacements[base .. xs[#xs]] = base .. xs[1]
end

mallet_cycle('fence/', {
    'edge/horiz', 'edge/vert',
    'corner/nw', 'corner/ne', 'corner/se', 'corner/sw',
})
mallet_cycle('fence/', { 'tee/e', 'tee/s', 'tee/w', 'tee/n', 'cross' })
mallet_cycle('fence/end/fancy/', { 'e', 'w' })

mallet_cycle('house_wall/edge/', { 'horiz/in', 'horiz/out', 'vert' })
mallet_cycle('house_wall/corner/', {
    'nw/in', 'ne/in', 'se/out', 'sw/out',
    'nw/out', 'ne/out', 'se/in', 'sw/in',
})
mallet_cycle('house_wall/tee/', {
    'n/in', 'n/out',
    'e/in', 'e/out',
    's/in_in', 's/in_out', 's/out_out', 's/out_in',
    'w/in', 'w/out',
})
mallet_cycle('house_wall/cross/', {
    'in_in', 'in_out', 'out_out', 'out_in',
})

mallet_cycle('house_wall/door/', { 'in', 'out' })


local terrain_cycle = {
    'center/v0',
    'edge/n', 'corner/outer/ne', 'edge/e', 'corner/outer/se',
    'edge/s', 'corner/outer/sw', 'edge/w', 'corner/outer/nw',
    'corner/inner/nw', 'corner/inner/ne', 'corner/inner/se', 'corner/inner/sw',
}
mallet_cycle('wood_floor/', terrain_cycle)
mallet_cycle('road/', terrain_cycle)


mallet_cycle('statue/', { 'e', 's', 'w', 'n' })

return {
    place_structure = place_structure,
    take_structure = take_structure,
    use_item = use_item,
    use_structure = use_structure,
}
