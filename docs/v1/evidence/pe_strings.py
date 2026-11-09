import re, sys
d=open(sys.argv[1],'rb').read()
S=set(m.group().decode() for m in re.finditer(rb'[\x20-\x7e]{6,}',d))|set(m.group().decode('utf-16le') for m in re.finditer(rb'(?:[\x20-\x7e]\x00){6,}',d))
for p in sys.argv[2:]:
    h=sorted(s for s in S if re.search(p,s))
    print(f"\n--- /{p}/ ({len(h)}) ---"); print(' | '.join(x[:110] for x in h[:30]))
