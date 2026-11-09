#!/usr/bin/env python3
"""Author classic MIX metadata fixtures; no retail file bytes are used."""
import json
from pathlib import Path
import struct

root = Path(__file__).resolve().parent
groups = {
    "yuri-authored.mix": [
        (0x827a6765, "BKGDSMY.SHP"), (0x7ed301af, "BKGDMDY.SHP"),
        (0x83911bac, "BKGDLGY.SHP"), (0x799d088d, "RADARY.SHP"),
        (0x0d7c6d7c, "UIBKGDY.PAL"), (0x0b8d57c4, "RADARYURI.PAL"),
        (0x763c81dd, "KEY.INI"),
    ],
    "allied-authored.mix": [
        (0x69b1fbfa, "BKGDSM.SHP"), (0x95189d30, "BKGDMD.SHP"),
        (0x685a8733, "BKGDLG.SHP"),
    ],
}
for name, entries in groups.items():
    body, table = bytearray(), bytearray()
    for entry_id, label in entries:
        payload = ("Authored metadata fixture for " + label + "\n").encode()
        table.extend(struct.pack("<III", entry_id, len(body), len(payload)))
        body.extend(payload)
    (root / name).write_bytes(struct.pack("<HI", len(entries), len(body)) + table + body)
(root / "expected-names.json").write_text(json.dumps(groups, indent=2) + "\n")
print("Authored classic MIX fixtures: 7 Yuri IDs, 3 Allied IDs. Payloads are not retail art or decoder fixtures.")
