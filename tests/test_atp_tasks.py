"""ATP task suite: all benchmark tasks built with the Python fluent spec API
instead of RTL (port of AtpTask001..150Test; builders are mechanically
translated from the Java sources, see tools/translate_atp.py)."""

import pytest

from atp_patterns import PATTERNS
from task_runner import run_task_variant, variants_of

PARAMS = [
    pytest.param(t, v, id=f"task_{t}-variant_{v}")
    for t in sorted(PATTERNS)
    for v in variants_of(t)
]


@pytest.mark.parametrize("task_id,variant", PARAMS)
def test_atp_task(task_id, variant):
    run_task_variant(task_id, variant, PATTERNS[task_id]())
