import json
import re
import typing as tp

import pytest

from .helpers import cli
from .helpers.tmp_file_manager import TmpFileManager

sample_config = """
[context.static]
  STAT_TEST_VAR = { value = ["Hello", "World"] }

[context.env]
  ENV_TEST_VAR = { default = { value = "World" } }

[context.cli]
  CLI_TEST_VAR = { commands = ["echo 1"], coerce = "int" }
"""


def run_read_var(toml: str, var: str, is_json=False) -> str:
    with TmpFileManager() as manager:
        return cli.run(
            [
                "zetch",
                "var",
                var,
                "--config",
                str(manager.tmpfile(toml)),
            ]
            + (["--output", "json"] if is_json else [])
        )


@pytest.mark.parametrize(
    "var, expected_json_result, custom_expected_raw_result",
    [
        ("STAT_TEST_VAR", ["Hello", "World"], None),
        (
            "ENV_TEST_VAR",
            "World",
            "World",
        ),  # Same as config checks, with raw strings come out unquoted
        ("CLI_TEST_VAR", 1, None),
    ],
)
def test_read_var_working(
    var: str, expected_json_result: tp.Any, custom_expected_raw_result: tp.Optional[str]
):
    res = run_read_var(sample_config, var, is_json=True)
    json_result = json.loads(res)
    assert json_result == expected_json_result
    raw_result = run_read_var(sample_config, var)
    if custom_expected_raw_result is None:
        assert json.loads(raw_result) == expected_json_result
    else:
        assert raw_result == custom_expected_raw_result


@pytest.mark.parametrize(
    "var, error_message",
    [
        (
            "nonexistent",
            "Context variable 'nonexistent' not found in finalised config. All context keys: 'STAT_TEST_VAR, ENV_TEST_VAR, CLI_TEST_VAR'.",
        ),
    ],
)
def test_read_var_fail(var: str, error_message: str):
    with pytest.raises(ValueError, match=re.escape(error_message)):
        run_read_var(sample_config, var)
