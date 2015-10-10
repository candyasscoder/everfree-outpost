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

-- Define a function that will run every time a player interacts (presses "A")
-- with the cornucopia.  The `client` argument is a reference to the client
-- object of the player who used the cornucopia, and `structure` is a reference
-- to the cornucopia structure itself.
action.use.cornucopia = function(client, structure)
    -- If this player has used a cornucopia before, don't allow them to use it
    -- again.  The method `client:extra()` returns a table of extra data
    -- attached to the client, which this mod uses to track which players have
    -- used cornucopias before.
    if client:extra().used_cornucopia then
        return
    end
    client:extra().used_cornucopia = true

    -- Choose an item from the `options` table to give to the player.
    local index = math.floor(math.random() * #options) + 1
    -- `client:pawn()` returns a reference to the entity (character) that the
    -- player controls.  `inventory('main')` gets the entity's main inventory
    -- (the one that appears when the player presses "E").  Finally,
    -- `update(item_name, count)` adjusts the amount of some item in the
    -- inventory, in this case increasing the amount of the chosen vegetable by
    -- five.  (Negative numbers can be used to reduce the amount.)
    --
    -- Once this script finishes, the player will see the normal "+5 Tomato"
    -- (or whatever vegetable) popup on the corner of their screen.
    client:pawn():inventory('main'):update(options[index], 5)

    client:pawn():inventory('main'):update('party_hat', 1)
end
