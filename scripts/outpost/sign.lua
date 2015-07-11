local action = require('core.action')
local structure_items = require('outpost.lib.structure_items')
local tools = require('outpost.lib.tools')

local SIGN_TEXT_DIALOG_ID = 0

function action.use.sign(c, s)
    c:send_message('Sign: ' .. s:extra().message)
end

function action.use_item.sign(c, inv, args)
    if args == nil then
        local id = c:world():item_name_to_id('sign')
        local parts = ExtraArg.map()
        c:get_use_item_args(id, SIGN_TEXT_DIALOG_ID, parts)
    else
        local s = structure_items.use_item(c, inv, 'sign', 'sign')
        s:extra().message = args:get('msg'):as_str()
    end
end

function tools.handler.axe.sign(c, s, inv)
    structure_items.use_structure(c, s, 'sign')
end
