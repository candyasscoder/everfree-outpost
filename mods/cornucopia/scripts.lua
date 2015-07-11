local action = require('core.action')

local options = {
    'tomato',
    'potato',
    'carrot',
    'artichoke',
    'pepper',
    'cucumber',
    'corn',
}

action.use.cornucopia = function(client, structure)
    if client:extra().used_cornucopia then
        return
    end
    client:extra().used_cornucopia = true

    local index = math.floor(math.random() * #options) + 1
    client:pawn():inventory('main'):update(options[index], 5)
end
