"""Fail if a Windows PE imports the MSVC C runtime DLLs.

The release exe links the CRT statically (.cargo/config.toml) so it starts on a
clean Windows install without the Visual C++ redistributable. Usage:
    python check_no_crt.py target/release/cpuzeez.exe
"""
import struct
import sys

FORBIDDEN = ("vcruntime", "msvcp", "api-ms-win-crt-", "ucrtbase")


def imports(path):
    d = open(path, "rb").read()
    u16 = lambda o: struct.unpack_from("<H", d, o)[0]
    u32 = lambda o: struct.unpack_from("<I", d, o)[0]
    pe = u32(0x3C)
    opt = pe + 24
    dd = opt + (112 if u16(opt) == 0x20B else 96)
    so = opt + u16(pe + 20)
    secs = [(u32(so + 40 * i + 12), max(u32(so + 40 * i + 8), u32(so + 40 * i + 16)), u32(so + 40 * i + 20)) for i in range(u16(pe + 6))]
    r2o = lambda r: next(r - va + raw for va, vs, raw in secs if va <= r < va + vs)
    o = r2o(u32(dd + 8))
    while u32(o + 12):
        n = r2o(u32(o + 12))
        yield d[n : d.index(b"\0", n)].decode()
        o += 20


bad = [dll for dll in imports(sys.argv[1]) if dll.lower().startswith(FORBIDDEN)]
if bad:
    sys.exit(f"{sys.argv[1]} imports the dynamic C runtime: {', '.join(bad)} (expected +crt-static)")
print(f"{sys.argv[1]}: no dynamic C runtime imports")
