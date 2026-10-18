#!/usr/bin/env python3
"""Focused falsification of the internal license-expression gate."""
from audit_dependencies import license_has_permitted_choice

allowed = [
    "MIT",
    "MIT OR Apache-2.0",
    "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
    "(GPL-2.0-only OR MIT) AND Apache-2.0",
]
denied = [
    "GPL-3.0-only",
    "LGPL-2.1-or-later AND MIT",
    "AGPL-3.0-only OR GPL-2.0-only",
    "GPL-2.0-only WITH Classpath-exception",
]
for expression in allowed:
    assert license_has_permitted_choice(expression) is True, expression
for expression in denied:
    assert license_has_permitted_choice(expression) is False, expression
assert license_has_permitted_choice("MIT/Apache-2.0") is None
print("License alternatives tests passed: permissive OR, required AND, copyleft-only and ambiguous syntax")
