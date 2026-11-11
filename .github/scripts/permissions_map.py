"""Check or regenerate (--write) the permissions table in README.md and
README.pt-BR.md from core::availability::MAP (S0-B04).

    python .github/scripts/permissions_map.py           # check
    python .github/scripts/permissions_map.py --write   # rewrite the blocks
"""
import os
import subprocess
import sys

env = dict(os.environ)
if "--write" in sys.argv[1:]:
    env["CZ_WRITE_PERMISSIONS"] = "1"
test = "availability::tests::the_readmes_show_this_map"
sys.exit(subprocess.call(["cargo", "test", "-q", "-p", "cpuzeez-core", "--lib", test, "--", "--exact"], env=env))
