/INSERT_EMSCRIPTEN_FUNCTIONS/ {
    while ((getline < "asmjs/asmlibs.1.js") > 0) {
        if ($0 == "// EMSCRIPTEN_END_FUNCTIONS")
            break;
        print;
    }
}

/INSERT_EMSCRIPTEN_STATIC/ {
    getline < "asmjs/asmlibs.1.js";
    getline < "asmjs/asmlibs.1.js";
    start = index($0, "[");
    end = index($0, "]");
    print substr($0, start, end - start + 1);
    getline;
}

{ print }
