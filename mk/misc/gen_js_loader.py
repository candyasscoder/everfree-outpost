import os
import sys
import textwrap

def main(files):
    root = files[-1]
    root_dir = os.path.dirname(root)

    print(textwrap.dedent('''
        (function() {
            window.exports = {};
            window.require = function() { return window.exports; };

            function load(f) {
                document.write('<script src="' + f + '"></script>');
            }
    '''))

    for f in files:
        rel = os.path.relpath(f, root_dir)
        print('    load(%r);' % os.path.join('js', rel))

    print(textwrap.dedent('''
        })();
    '''))

if __name__ == '__main__':
    main(sys.argv[1:])
