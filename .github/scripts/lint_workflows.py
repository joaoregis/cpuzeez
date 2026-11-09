"""Fail on a workflow that GitHub would refuse before running anything.

A duplicate key (two `with:` on one step) is valid for most YAML loaders, which
keep the last one, and GitHub rejects the whole file ("workflow file issue"),
so a release would not start. This loader refuses duplicates.

    python .github/scripts/lint_workflows.py
"""
import glob
import sys

import yaml


class Strict(yaml.SafeLoader):
    pass


def mapping(loader, node, deep=False):
    seen = set()
    for key_node, _ in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in seen:
            raise ValueError(f"duplicate key {key!r} at line {key_node.start_mark.line + 1}")
        seen.add(key)
    return yaml.SafeLoader.construct_mapping(loader, node, deep)


Strict.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, mapping)

bad = 0
for path in sorted(glob.glob(".github/workflows/*.y*ml") + glob.glob(".github/*.y*ml")):
    try:
        with open(path, encoding="utf-8") as f:
            doc = yaml.load(f, Strict)
        if path.startswith(".github/workflows") and not (isinstance(doc, dict) and "jobs" in doc):
            raise ValueError("no jobs")
        print("ok", path)
    except Exception as e:  # noqa: BLE001
        bad += 1
        print(f"FAIL {path}: {e}")
sys.exit(1 if bad else 0)
