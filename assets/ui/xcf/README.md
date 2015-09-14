How to export XCF layers as PNG:

 1. Create the directory `/tmp/gimp-layers`

 2. Open the target file in GIMP

 3. Paste this script into the GIMP Python console (Filters > Python-Fu >
    Console):

        def export_layers(n, l):
            if 'orig' in l.name or 'slicing' in l.name or 'fill' in l.name:
                return
            base, _, _ = l.name.partition('#')
            base = base.strip().replace(' ', '-')
            name = n + '-' + base
            if len(l.children) == 0:
                filename = '/tmp/gimp-layers/%s.png' % name
                pdb.file_png_save(l.image, l, filename, filename,
                        False, 3, False, False, False, False, False)
            else:
                for c in l.children:
                    go(name, c)

        def export_image_layers(i):
            name, _ = os.path.splitext(os.path.basename(i.filename))
            for l in i.layers:
                go(name, l)

        export_image_layers(gimp.image_list()[0])

    Note that this only exports from the most recently opened image.  Play
    around with the final line if you want something different.

 4. Run `optipng /tmp/gimp-layers/*.png`.

 5. Move all files from `/tmp/gimp-layers` to the target directory.
