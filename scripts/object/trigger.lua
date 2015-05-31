local action = require('outpost.action')
local tools = require('lib.tools')


local function run_handler(handlers, c, s)
    local key = s:stable_id():id()
    local handler = handlers[key]
    if handler ~= nil then
        handler(c, s)
    else
        c:send_message('No handler for ' .. key .. ' (' .. tostring(s) .. ')')
    end
end


local use_handlers = {}

function action.use.script_trigger(c, s)
    run_handler(use_handlers, c, s)
end


local pick_handlers = {}

function tools.handler.pick.script_trigger(c, s, inv)
    run_handler(pick_handlers, c, s)
end


return {
    use = use_handlers,
    pick = pick_handlers,
}
