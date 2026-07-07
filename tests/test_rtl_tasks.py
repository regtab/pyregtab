"""RTL task suite: all benchmark tasks x all fixture variants
(port of RtlTask001..150Test). RTL sources come from the conformance corpus."""

import pytest

from task_runner import compile_task, run_task_variant, task_ids, variants_of

PARAMS = [
    pytest.param(t, v, id=f"task_{t}-variant_{v}")
    for t in task_ids()
    for v in variants_of(t)
]


@pytest.mark.parametrize("task_id,variant", PARAMS)
def test_rtl_task(task_id, variant):
    pattern = compile_task(task_id)
    run_task_variant(task_id, variant, pattern)
