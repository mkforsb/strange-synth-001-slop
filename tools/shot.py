"""Screenshot a preset while holding notes.
Usage: python3 tools/shot.py <public dir> "<preset name>" <out.png> [keys] [seconds]"""
import functools, http.server, sys, threading, time
from playwright.sync_api import sync_playwright
public, preset, out = sys.argv[1:4]
keys = sys.argv[4] if len(sys.argv) > 4 else "adg"
secs = float(sys.argv[5]) if len(sys.argv) > 5 else 3.0
handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=public); handler.log_message = lambda *a: None
srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler); threading.Thread(target=srv.serve_forever, daemon=True).start()
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/chromium", args=["--autoplay-policy=no-user-gesture-required", "--no-sandbox"])
    page = b.new_page(viewport={"width": 1500, "height": 1000})
    page.goto(f"http://127.0.0.1:{srv.server_address[1]}/"); page.wait_for_selector("#start"); page.click("#start")
    page.wait_for_selector(".splash", state="detached")
    page.select_option(".preset-main select", label=preset)
    for k in keys: page.keyboard.down(k)
    time.sleep(secs)
    page.screenshot(path=out)
    b.close()
