"""Headless browser smoke test.

Serves the built app, boots audio, plays notes and checks the DSP worklet
reports output. Usage: python3 tools/smoke.py <public dir> [script.json]
"""
import functools, http.server, json, sys, threading, time
from playwright.sync_api import sync_playwright

public = sys.argv[1]
handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=public)
handler.log_message = lambda *a: None
srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
threading.Thread(target=srv.serve_forever, daemon=True).start()
url = f"http://127.0.0.1:{srv.server_address[1]}/"

with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/chromium", args=["--autoplay-policy=no-user-gesture-required", "--no-sandbox"])
    page = b.new_page()
    logs = []
    page.on("console", lambda m: logs.append(f"[{m.type}] {m.text}"))
    page.on("pageerror", lambda e: logs.append(f"[pageerror] {e}"))
    page.goto(url)
    page.wait_for_selector("#start")
    page.click("#start")
    page.wait_for_function("document.querySelector('#status').textContent.includes('Running') || document.querySelector('#status').textContent.includes('Failed')", timeout=15000)
    print("status:", page.inner_text("#status"))
    page.click("#on"); time.sleep(0.6); page.click("#read")
    print("peak while held:", page.inner_text("#peak"))
    page.click("#off"); time.sleep(3.0); page.click("#read")
    print("peak after release:", page.inner_text("#peak"))
    for l in logs: print(l)
    b.close()
