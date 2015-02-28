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

local dirs = {'n', 'ne', 'e', 'se', 's', 'sw', 'w', 'nw', 'n'}
for i = 1, 8 do
    local dir = dirs[i]
    add_structure_item('house_wall/' .. dir)
    mallet.replacements['house_wall/' .. dir] = 'house_wall/' .. dirs[i + 1]
end
add_structure_item('house_floor')

return {
    place_structure = place_structure,
    take_structure = take_structure,
    use_item = use_item,
    use_structure = use_structure,
}
