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

add_structure_item('wood_wall/side', 'wood_wall/edge/horiz')
add_structure_item('wood_wall/corner', 'wood_wall/corner/nw')
add_structure_item('wood_wall/tee', 'wood_wall/tee/n')
add_structure_item('wood_wall/cross', 'wood_wall/cross')
add_structure_item('wood_door', 'wood_wall/door')

add_structure_item('stone_wall/side', 'stone_wall/edge/horiz')
add_structure_item('stone_wall/corner', 'stone_wall/corner/nw')
add_structure_item('stone_wall/tee', 'stone_wall/tee/n')
add_structure_item('stone_wall/cross', 'stone_wall/cross')
add_structure_item('stone_door', 'stone_wall/door')

add_structure_item('house_floor', 'wood_floor/center/v0')
add_structure_item('road', 'road/center/v0')
add_structure_item('bed')
add_structure_item('table')
add_structure_item('statue', 'statue/e')
add_structure_item('trophy', 'trophy')
add_structure_item('fountain', 'fountain')
add_structure_item('torch', 'torch')
add_structure_item('stair', 'stair/n')


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

mallet_cycle('wood_wall/edge/', { 'horiz', 'vert' })
mallet_cycle('wood_wall/corner/', { 'nw', 'ne', 'se', 'sw', })
mallet_cycle('wood_wall/tee/', { 'n', 'e', 's', 'w', })

mallet_cycle('stone_wall/', { 'edge/horiz', 'edge/vert', 'window/v0', 'window/v1' })
mallet_cycle('stone_wall/corner/', { 'nw', 'ne', 'se', 'sw', })
mallet_cycle('stone_wall/tee/', { 'n', 'e', 's', 'w', })



local terrain_cycle = {
    'center/v0',
    'edge/n', 'corner/outer/ne', 'edge/e', 'corner/outer/se',
    'edge/s', 'corner/outer/sw', 'edge/w', 'corner/outer/nw',
    'corner/inner/nw', 'corner/inner/ne', 'corner/inner/se', 'corner/inner/sw',
}
mallet_cycle('wood_floor/', terrain_cycle)
mallet_cycle('road/', terrain_cycle)


mallet_cycle('statue/', { 'e', 's', 'w', 'n' })



attachment_map = {}
local function check_attachment(attach, s)
    if s == nil then
        return true
    end

    if s:layer() == 0 then
        return true
    else if s:layer() > 1 then
        return false
    end end

    if not attachment_map[attach][s:template()] then
        return false
    else
        return true
    end
end


local function use_attachment_item(c, inv, item_name, template_name)
    local pawn = c:pawn()
    local plane = pawn:plane()
    local s = util.hit_structure(pawn)

    local pos
    if s == nil then
        pos = util.hit_tile(c:pawn())
    else
        pos = s:pos()
    end

    if not ward.check(c, pos) then
        return
    end

    if not check_attachment(template_name, s) then
        return nil
    end

    return place_structure(c:world(), inv, plane, pos, item_name, template_name)
end

local function add_attachment_item(item_name, template_name)
    if template_name == nil then
        template_name = item_name
    end

    action.use_item[item_name] = function(c, inv)
        use_attachment_item(c, inv, item_name, template_name)
    end

    action.use[template_name] = function(c, s)
        use_structure(c, s, item_name)
    end
end


horiz_walls = {
    ['house_wall/edge/horiz/in'] = true,
    ['house_wall/edge/horiz/out'] = true,
    ['house_wall/tee/n/in'] = true,
    ['house_wall/tee/n/out'] = true,
    ['wood_wall/edge/horiz'] = true,
    ['wood_wall/tee/n'] = true,
}

attachment_map['cabinets'] = horiz_walls
attachment_map['bookshelf/0'] = horiz_walls

add_attachment_item('bookshelf', 'bookshelf/0')





return {
    place_structure = place_structure,
    take_structure = take_structure,
    use_item = use_item,
    use_attachment_item = use_attachment_item,
    use_structure = use_structure,
}
