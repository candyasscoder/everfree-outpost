# Art Requests

This is a list of artwork that will be needed for upcoming features.  The list
is sorted roughly by how soon the feature will be implemented.


## Torches

 * Torch attached to a horizontal (east-west) house wall.  It should fit within
   a single 32x32px tile.  The left torch from
   [LPC castle_lightsources.png][castle_lightsources] might work okay for this,
   but it's a little hard to make out exactly what that tile is supposed to be
   (it looks more like a candle than a torch, at least to me).  The middle
   torch looks better but is too large (32x64).

[castle_lightsources]: assets/tiles/lpc-base-tiles/castle_lightsources.png

 * Torch attached to a vertical (north-south) wall.  This should match the
   design of the previous torch, but facing a different direction.  The image
   can be mirrored to handle attachment on either the left or the right side of
   the wall.  The torch itself should fit within 12x32px so it can be placed to
   the side of the wall without extending beyond the 32x32px tile containing
   the wall itself.

 * Torch standing upright on the floor.  This should have a similar design to
   the previous two, aside from not being attached to a wall.  The torch should
   be about the same size as before.


## Cave interior tile variants

 * Variants of the [cave interior tiles][lpc-cave-inside] that support
   one-tile-wide walls.  This basically means extending the
   [cave wall tileset][lpc-cave-walls] with tees and N/S/E/W end caps.


## Cave puzzle elements

 * Stone door that fits inside the existing cave entrance
   ([lpc-cave-walls.png][lpc-cave-walls]).  It should look similar to the
   surrounding rock, so that when the door is closed, the player sees only a
   seam in the stone marking the door's outline.

[lpc-cave-walls]: assets/tiles/lpc-cave-walls.png

 * Stone button.  It should have two frames, a "normal" state plus a second
   where the button is pressed.

 * Stone door opening animation.  The door should slide back slightly and then
   move to the side.


## Ancient ruin tileset

 * Walls that look like the inside of an ancient ruin (think typical RPG
   "legendary ancient race" sort of thing).  Needs interior walls matching the
   tile layout of [lpc-cave-inside.png][lpc-cave-inside]

[lpc-cave-inside]: assets/tiles/lpc-cave-inside.png
