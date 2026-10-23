"""Make a `dx bundle` web build servable from any directory.

dx always emits absolute asset URLs ("/assets/..." or "/./assets/...") and
`base_path` can't express "relative to wherever this is hosted", so rewrite
them after bundling:

  index.html  href="/assets/x"  ->  href="./assets/x"
  assets/*.js "/assets/x.wasm"  ->  new URL("x.wasm",import.meta.url).href

The JS form resolves next to the script itself, so it works even when the
page URL lacks a trailing slash. Idempotent. Usage:
python3 tools/relative_paths.py <public dir>
"""
import pathlib, re, sys

ABS = r"/(?:\./)?assets/"
HTML_ATTR = re.compile(r'((?:href|src)\s*=\s*["\'])' + ABS)
JS_STRING = re.compile(r'(["\'])' + ABS + r'([^"\'/]+)\1')

public = pathlib.Path(sys.argv[1])
index = public / "index.html"
if not index.is_file():
    sys.exit(f"{index}: not found (run `dx bundle --platform web --release` first)")


def rewrite(path, pattern, repl):
    text = path.read_text()
    new, n = pattern.subn(repl, text)
    if n:
        path.write_text(new)
        print(f"{path.relative_to(public)}: {n} path(s) made relative")
    if re.search(r'["\']' + ABS, new):
        sys.exit(f"{path}: absolute asset path left over that this script doesn't handle")


rewrite(index, HTML_ATTR, r"\1./assets/")
for js in sorted((public / "assets").glob("*.js")):
    rewrite(js, JS_STRING, r"new URL(\1\2\1,import.meta.url).href")
