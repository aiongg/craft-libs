#!/usr/bin/env python3
"""speed-diag driver: python3 -I drive.py <bin> <out.log> <level> <steps>
steps: ';'-separated: wait=S | move=x,y | click=x,y | mark=name | shot=path. Runs the gallery under Xvfb with CRAFT_DIAG=1;
stderr DIAG lines go to <out.log> with MARK lines at each step."""
import ctypes, os, subprocess, sys, tempfile, threading, time, shutil

def inner():
    binary, out, steps = sys.argv[2], sys.argv[3], sys.argv[4]
    log = open(out, "w"); lock = threading.Lock()
    p = subprocess.Popen([binary], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    def pump():
        for line in p.stderr:
            with lock: log.write(line); log.flush()
    threading.Thread(target=pump, daemon=True).start()
    def mark(m):
        with lock: log.write("MARK %s\n" % m); log.flush()
    x11 = ctypes.CDLL("libX11.so.6"); xtst = ctypes.CDLL("libXtst.so.6")
    x11.XOpenDisplay.restype = ctypes.c_void_p
    d = ctypes.c_void_p(x11.XOpenDisplay(None))
    for st in steps.split(";"):
        k, _, v = st.partition("=")
        if k == "wait": time.sleep(float(v))
        elif k == "mark": mark(v)
        elif k == "move":
            x, y = map(int, v.split(",")); xtst.XTestFakeMotionEvent(d, -1, x, y, 0); x11.XFlush(d)
        elif k == "click":
            x, y = map(int, v.split(","))
            xtst.XTestFakeMotionEvent(d, -1, x, y, 0); x11.XFlush(d); time.sleep(0.5)
            mark("click %d,%d" % (x, y))
            xtst.XTestFakeButtonEvent(d, 1, 1, 0); x11.XFlush(d); time.sleep(0.15)
            xtst.XTestFakeButtonEvent(d, 1, 0, 0); x11.XFlush(d)
        elif k == "shot": subprocess.run(["import", "-window", "root", v], check=True)
    p.terminate()
    try: p.wait(5)
    except subprocess.TimeoutExpired: p.kill()

def main():
    binary, out, level, steps = sys.argv[1:5]
    scratch = tempfile.mkdtemp(prefix="diag-", dir=os.path.expanduser("~/.cache"))
    cfg = scratch + "/config/craft-ui-gallery"; os.makedirs(cfg); os.makedirs(scratch + "/data")
    open(cfg + "/config.toml", "w").write('schema_version = 2\nmode = "dark"\nfilter = "%s"\nlevel = "%s"\nopen_levels = ["%s"]\n' % (os.environ.get("DIAG_FILTER", ""), level.title(), level))
    open(cfg + "/window.json", "w").write('{"schema_version":1,"windows":{"main":{"normal":{"pos":[0,0],"size":[1280,800]},"scale":1.0}}}')
    env = dict(os.environ, XDG_CONFIG_HOME=scratch + "/config", XDG_DATA_HOME=scratch + "/data", WINIT_UNIX_BACKEND="x11",
               LIBGL_ALWAYS_SOFTWARE="1", CRAFT_DIAG="1")
    env.pop("WAYLAND_DISPLAY", None)
    subprocess.run(["timeout", "120", "xvfb-run", "-a", "-s", "-screen 0 1400x900x24", sys.executable, "-I", os.path.abspath(__file__),
                    "--inner", os.path.abspath(binary), os.path.abspath(out), steps], env=env)
    shutil.rmtree(scratch, ignore_errors=True)

if __name__ == "__main__":
    inner() if len(sys.argv) > 1 and sys.argv[1] == "--inner" else main()
