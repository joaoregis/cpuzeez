import struct, sys, re
d=open(sys.argv[1],'rb').read()
u16=lambda o:struct.unpack_from('<H',d,o)[0]; u32=lambda o:struct.unpack_from('<I',d,o)[0]
pe=u32(0x3c); opt=pe+24; dd=opt+112; so=opt+u16(pe+20)
secs=[(u32(so+40*i+12),max(u32(so+40*i+8),u32(so+40*i+16)),u32(so+40*i+20)) for i in range(u16(pe+6))]
r2o=lambda r:next((r-va+raw for va,vs,raw in secs if va<=r<va+vs),None)
cs=lambda o:d[o:d.index(b'\0',o)].decode(errors='replace')
def walk(dir_rva, delay):
    o=r2o(dir_rva); step=32 if delay else 20
    while o and u32(o+(4 if delay else 12)):
        name=cs(r2o(u32(o+(4 if delay else 12)))); th=r2o(u32(o+(16 if delay else 0)) or u32(o+16)); n=[]
        while th:
            v=struct.unpack_from('<Q',d,th)[0]
            if not v: break
            n.append(f"#{v&0xffff}" if v>>63 else cs(r2o(v&0x7fffffff)+2)); th+=8
        print(f"{'[delay] ' if delay else ''}{name} ({len(n)}): {', '.join(n)}"); o+=step
if u32(dd+8): walk(u32(dd+8),False)
if u32(dd+13*8): walk(u32(dd+13*8),True)
