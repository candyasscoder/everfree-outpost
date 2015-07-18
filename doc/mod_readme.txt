Everfree Outpost (Modpack Builder - Windows)
============================================

This package contains all the components needed to develop Everfree Outpost
mods, or to combine existing mods into a modpack.

To start the modpack builder GUI, run "build_gui.py".  It requires Python 3.4+,
which you can download from http://python.org/.  The top-left pane shows all
the mods that are currently available in the mods\ subdirectory.  The top-right
pane shows mods that will be included in the current build.  The bottom pane
has the "Start Build" button and the build output.  Once the build is done, you
can start the modded server with "server_gui.py" in the dist\ subdirectory.

To transfer the compiled mods to another server, copy the dist\data\,
dist\scripts\, and dist\www\ directories to replace the corresponding
directories of the destination server.  These files should be compatible across
operating systems, so (for example) you can run the modpack builder on Windows
and then copy the files to a Linux server.

For more information on the mods themselves, see mods\README.txt.
