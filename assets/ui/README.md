The PNG files are automatically extracted from the XCFs.  This isn't included
in the build process because it requires having GIMP installed.  After editing
the UI, you should regenerate the PNGs by running:

    rm png/*.png
    GIMP_LAYER_EXPORT_DIR=png ./export_layers.sh xcf/*.xcf

Then commit the modified PNGs.
