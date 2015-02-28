local action = require('outpost.action')
local util = require('outpost.util')
local ward = require('ward')

local replacements = {}

action.use_item.mallet = function(c, inv)
    print('whack')
    local s = util.hit_structure(c:pawn())
    if s == nil then
        return
    end

    if not ward.check(c, s:pos()) then
        return
    end

    new_template = replacements[s:template()]
    if new_template ~= nil then
        s:replace(new_template)
    end
end

return {
    replacements = replacements,
}
