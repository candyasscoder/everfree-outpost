The entity `appearance` field is currently a 32-bit integer.  The bits are allocated as follows:

    Color of the pony.  Both the body and the mane/tail are tinted this color.
     0 -  1:    Red
     2 -  3:    Green
     4 -  5:    Blue
     6:         Wings.  1 if the pony has wings.
     7:         Horn.  1 if the pony has a horn.
     8:         Hat.  1 if the pony has a hat.
     9:         Light.  1 if the pony is glowing (due to the unicorn "Light" spell.  
    10 - 30:    Unused
    31:         Reserved (must be zero).  May be used in the future for non-pony entities.


Planned future allocation:

    Color of the pony.  Both the body and the mane/tail are tinted this color.
     0 -  1:    Red
     2 -  3:    Green
     4 -  5:    Blue
     6:         Wings.  1 if the pony has wings.
     7:         Horn.  1 if the pony has a horn.
     8:         Stallion.  1 if the pony is a stallion, 0 if a mare.
     9:         Light.  1 if the pony is glowing (due to the unicorn "Light" spell.  
    10 - 12:    Mane variant (0-7)
    13 - 15:    Tail variant (0-7)
    16 - 17:    Eye variant (0-3)
    18 - 21:    Head equipment variant (0-15; 0 indicates no equipment)
    22 - 25:    Front equipment variant (0-15; 0 indicates no equipment)
    26 - 29:    Back equipment variant (0-15; 0 indicates no equipment)
    30:         Unused
    31:         Reserved (must be zero).  May be used in the future for non-pony entities.
