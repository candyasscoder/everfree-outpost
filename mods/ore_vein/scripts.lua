-- Import some modules from the main Everfree Outpost `scripts/` directory.
local tools = require('outpost.lib.tools')
local ward = require('outpost.lib.ward')

-- Set up a handler function that will be called when the player uses a `pick`
-- on an `ore_vein/copper` structure.  The function will be called with three
-- arguments: the `Client` object corresponding to the player, the `Structure`
-- object representing the ore vein, and the player character's inventory.
tools.handler.pick['ore_vein/copper'] = function(c, s, inv)
    -- First, call a library function to make sure the area containing the ore
    -- vein is not protected by a ward (or if it is, it should be the player's
    -- own ward).
    if not ward.check(c, s:pos()) then
        -- There is a ward preventing the player from mining this ore.  The
        -- `ward.check` function already sent the message ("This area belongs
        -- to ..."), so all we need to do here is return instead of going ahead
        -- with the mining.
        return
    end

    -- Do the actual mining: destroy the ore vein structure, and update the
    -- player character's inventory by adding one `ore/copper` item.
    s:destroy()
    inv:update('ore/copper', 1)
end

-- For more info on the available scripting functions, look in the `scripts/`
-- directory in the main Everfree Outpost source code.  Files under
-- `core/` and `outpost/lib/` are libraries designed to be imported into other
-- scripts.  The `.lua` files directly under `outpost/` define the behavior of
-- various objects in the base game.  Finally, the Rust source file
-- `src/server/script/userdata/world.rs` defines the basic operations for game
-- object types such as Client, Structure, and Inventory.  (This should be
-- documented somewhere but I haven't done it yet.)
