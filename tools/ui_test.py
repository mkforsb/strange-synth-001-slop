"""Headless UI test: boots audio, plays notes, checks locks survive RND,
loads presets and writes screenshots to out/.

Usage: python3 tools/ui_test.py <public dir>
"""
import functools, http.server, os, sys, threading, time
from playwright.sync_api import sync_playwright

public = sys.argv[1]
os.makedirs("out", exist_ok=True)
handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=public)
handler.log_message = lambda *a: None
srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
threading.Thread(target=srv.serve_forever, daemon=True).start()
url = f"http://127.0.0.1:{srv.server_address[1]}/"
failures = []

def check(cond, msg):
    print(("PASS " if cond else "FAIL ") + msg)
    if not cond: failures.append(msg)

with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/chromium", args=["--autoplay-policy=no-user-gesture-required", "--no-sandbox", "--use-gl=swiftshader"])
    page = b.new_page(viewport={"width": 1500, "height": 1000})
    logs = []
    page.on("console", lambda m: logs.append(f"[{m.type}] {m.text}"))
    page.on("pageerror", lambda e: logs.append(f"[pageerror] {e}"))
    page.goto(url)
    page.wait_for_selector("#start")
    page.screenshot(path="out/0-splash.png")
    page.click("#start")
    page.wait_for_selector(".splash", state="detached", timeout=15000)
    check(page.locator(".status").inner_text().endswith("kHz"), "audio running: " + page.locator(".status").inner_text())
    check(page.locator(".knob").count() >= 60, f"knobs rendered: {page.locator('.knob').count()}")
    check(page.locator(".choice").count() == 2, "choice widgets rendered")

    # Play a chord on the computer keyboard and hold it.
    for k in ["a", "d", "g"]:
        page.keyboard.down(k)
    time.sleep(1.2)
    check(page.locator(".piano .lit").count() == 3, f"3 keys lit: {page.locator('.piano .lit').count()}")
    meter = page.evaluate("document.getElementById('meter-fill').style.width")
    check(meter not in ("", "0%", "0.0%"), f"meter moving: {meter}")
    page.screenshot(path="out/1-playing.png")
    for k in ["a", "d", "g"]:
        page.keyboard.up(k)

    # Shift-click lock on the first knob (Strike), then RND a few times.
    first = page.locator(".knob").first
    before = first.locator(".knob-value").inner_text()
    page.keyboard.down("Shift"); first.click(); page.keyboard.up("Shift")
    check(page.locator(".knob.locked").count() == 1, "shift-click locks")
    values_before = page.locator(".knob-value").all_inner_texts()
    for _ in range(3):
        page.click(".rnd"); time.sleep(0.15)
    values_after = page.locator(".knob-value").all_inner_texts()
    check(page.locator(".knob").first.locator(".knob-value").inner_text() == before, "locked knob unchanged by RND")
    changed = sum(1 for a, c in zip(values_before, values_after) if a != c)
    check(changed > 30, f"RND changed {changed} knobs")
    check(page.locator(".knob.locked").count() == 1, "lock survives RND")
    page.keyboard.down("s"); time.sleep(0.8)
    page.screenshot(path="out/2-random.png")
    page.keyboard.up("s")

    # Load the elevator preset via the select.
    page.select_option(".preset-main select", label="Metal-on-Metal Screeching Industrial Elevator")
    time.sleep(0.2)
    check("Elevator" in page.locator(".preset-name").inner_text(), "preset loads via select")
    page.keyboard.down("a"); page.keyboard.down("h"); time.sleep(2.0)
    page.screenshot(path="out/3-elevator.png")
    page.keyboard.up("a"); page.keyboard.up("h")

    # Arrow keys step presets.
    page.keyboard.press("ArrowRight"); time.sleep(0.1)
    check("Struck Bowed" in page.locator(".preset-name").inner_text(), "ArrowRight steps preset: " + page.locator(".preset-name").inner_text())

    # Drag a knob upward.
    k = page.locator(".knob").nth(3)
    box = k.bounding_box()
    v0 = k.locator(".knob-value").inner_text()
    page.mouse.move(box["x"] + box["width"] / 2, box["y"] + 20)
    page.mouse.down(); page.mouse.move(box["x"] + box["width"] / 2, box["y"] - 60, steps=6); page.mouse.up()
    check(k.locator(".knob-value").inner_text() != v0, f"drag changes knob: {v0} -> {k.locator('.knob-value').inner_text()}")

    # Persistence: reload keeps the lock and the preset.
    page.reload(); page.wait_for_selector("#start")
    check(page.locator(".knob.locked").count() == 1, "lock persisted across reload")
    check("edited" in page.locator(".preset-meta").inner_text().lower(), "edited flag persisted")

    errors = [l for l in logs if "error" in l.lower() and "favicon" not in l and "404" not in l]
    check(not errors, "no console errors")
    for l in errors[:20]: print("   ", l)
    b.close()

print("\nFAILURES:", failures if failures else "none")
sys.exit(1 if failures else 0)
