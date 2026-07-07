"""Round-trip: compile(serialize(p)) == p for tasks 001-050
(port of AtpRtlRoundTripTest)."""

import pytest

from pyregtab import AtpToRtlSerializer, RtlCompiler
from task_runner import task_ids, task_rtl

TASKS = [t for t in task_ids() if t.isdigit() and int(t) <= 50]


@pytest.mark.parametrize("task_id", TASKS, ids=lambda t: f"task_{t}")
def test_round_trip(task_id):
    p = RtlCompiler.compile(task_rtl(task_id))
    s = AtpToRtlSerializer.serialize(p)
    p2 = RtlCompiler.compile(s)
    assert p2 == p
