import sys

def main(template_path, asmjs_path):
    with open(template_path) as template, \
            open(asmjs_path) as asmjs:
        for line in template:
            if line.strip() == '// INSERT_EMSCRIPTEN_FUNCTIONS':
                for code_line in asmjs:
                    if code_line.strip() == '// EMSCRIPTEN_END_FUNCTIONS':
                        break
                    else:
                        sys.stdout.write(code_line)
            elif line.strip() == '// INSERT_EMSCRIPTEN_STATIC':
                for code_line in asmjs:
                    if 'allocate(' in code_line:
                        i = code_line.find('[')
                        j = code_line.find(']', i)
                        sys.stdout.write(code_line[i : j + 1] + '\n')
                        break
            else:
                sys.stdout.write(line)

if __name__ == '__main__':
    template_path, asmjs_path = sys.argv[1:]
    main(template_path, asmjs_path)
