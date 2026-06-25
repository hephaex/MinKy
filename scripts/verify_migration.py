#!/usr/bin/env python3
"""
Verification script for transcripts migration.

NOTE: verify_transcripts_via_api() is an external API smoke test. It should
not be confused with the core migration verification checks implemented in
this script. Use it only for quick gut-checks against an already-running
Minky instance.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
