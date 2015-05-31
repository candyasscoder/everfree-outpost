local action = require('outpost.action')
local util = require('outpost.util')
local structure_items = require('lib.structure_items')
local tools = require('lib.tools')
local ward = require('lib.ward')


function action.use.ward(c, s)
    local owner = s:extra().owner
    if c == owner:get() then
        if structure_items.use_structure(c, s, 'ward') then
            ward.remove_ward(c)
        end
    else
        owner_name = ward.ward_info(owner).name
        c:send_message('That ward belongs to ' .. owner_name)
    end
end

function action.use_item.ward(c, inv)
    if not check_forest(c) then return end

    if ward.ward_info(c:stable_id()) ~= nil then
        c:send_message('You may only place one ward at a time.')
        return
    end

    local pos = util.hit_tile(c:pawn())
    local other_info = ward.find_ward(c, pos, ward.WARD_SPACING)
    if other_info ~= nil then
        c:send_message('This area is too close to land belonging to ' .. other_info.name)
        return
    end

    local s = structure_items.use_item(c, inv, 'ward', 'ward')
    if s ~= nil then
        ward.add_ward(c, s)
    end
end
