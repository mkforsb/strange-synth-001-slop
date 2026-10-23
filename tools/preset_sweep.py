"""Load every factory preset in the browser, play a note and confirm the
worklet produces sound without errors. Usage: python3 tools/preset_sweep.py <public dir>"""
import functools, http.server, sys, threading, time
from playwright.sync_api import sync_playwright
handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=sys.argv[1]); handler.log_message = lambda *a: None
srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler); threading.Thread(target=srv.serve_forever, daemon=True).start()
silent = []
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/chromium", args=["--autoplay-policy=no-user-gesture-required", "--no-sandbox"])
    page = b.new_page(viewport={"width": 1500, "height": 1000})
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.on("console", lambda m: errors.append(m.text) if m.type == "error" and "404" not in m.text else None)
    page.goto(f"http://127.0.0.1:{srv.server_address[1]}/"); page.wait_for_selector("#start"); page.click("#start")
    page.wait_for_selector(".splash", state="detached")
    n = int(page.locator(".preset-meta .num").inner_text().split("/")[1])
    for i in range(n):
        name = page.locator(".preset-name").inner_text()
        page.keyboard.press("Escape")
        page.keyboard.down("a"); page.keyboard.down("g")
        best = 0.0
        for _ in range(8):
            time.sleep(0.12)
            w = page.evaluate("parseFloat(document.getElementById('meter-fill').style.width) || 0")
            best = max(best, w)
        page.keyboard.up("a"); page.keyboard.up("g")
        print(f"{i+1:2} {best:5.1f}%  {name}")
        if best < 20: silent.append(name)
        page.keyboard.press("ArrowRight")
    b.close()
print("quiet:", silent or "none"); print("errors:", errors or "none")
