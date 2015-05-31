local action = require('outpost.action')
local util = require('outpost.util')
local ward = require('lib.ward')
local mallet = require('lib.mallet')

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







return {
    place_structure = place_structure,
    take_structure = take_structure,
    use_item = use_item,
    use_structure = use_structure,
    add_structure_item = add_structure_item,

    attachment_map = attachment_map,
    use_attachment_item = use_attachment_item,
    add_attachment_item = add_attachment_item,
}
