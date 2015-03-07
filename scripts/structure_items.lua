local action = require('outpost.action')
local util = require('outpost.util')
local ward = require('ward')
local mallet = require('mallet')

local function place_structure(world, inv, pos, item_name, template_name)
    if inv:count(item_name) == 0 then
        return
    end

    s, err = world:create_structure(pos, template_name)

    if s ~= nil then
        s:attach_to_chunk()
        inv:update(item_name, -1)
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
    local pos = util.hit_tile(c:pawn())

    if not ward.check(c, pos) then
        return
    end

    return place_structure(c:world(), inv, pos, item_name, template_name)
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

add_structure_item('fence', 'fence/n')
add_structure_item('fence_post', 'fence/gate_post_w')
add_structure_item('house_wall/side', 'house_wall/side/n')
add_structure_item('house_wall/corner', 'house_wall/corner/inner/nw')
add_structure_item('house_wall/tee', 'house_wall/tee/interior/e')
add_structure_item('house_floor')
add_structure_item('house_door')
add_structure_item('road')
add_structure_item('bed')
add_structure_item('table')


local function mallet_cycle(base, xs)
    for i = 1, #xs - 1 do
        mallet.replacements[base .. xs[i]] = base .. xs[i + 1]
        action.use[base .. xs[i + 1]] = action.use[base .. xs[i]]
    end
    mallet.replacements[base .. xs[#xs]] = base .. xs[1]
end

mallet_cycle('fence/', {'n', 'ne', 'e', 'se', 's', 'sw', 'w', 'nw'})
mallet_cycle('house_wall/side/', {'n', 'e', 'e_interior', 's', 'w', 'w_interior'})
mallet_cycle('house_wall/corner/',
    {'inner/nw', 'inner/ne', 'inner/se', 'inner/sw',
     'outer/nw', 'outer/ne', 'outer/se', 'outer/sw'})
mallet_cycle('house_wall/tee/',
    {'interior/e', 'interior/s', 'interior/w', 'interior/n',
     'exterior/e', 'exterior/s', 'exterior/w', 'exterior/n'})
mallet_cycle('house_door', {'', '_interior'})
mallet_cycle('fence/gate_post_', {'w', 'e'})


return {
    place_structure = place_structure,
    take_structure = take_structure,
    use_item = use_item,
    use_structure = use_structure,
}
