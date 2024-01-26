import json
import os
import re
import typing as tp
from unittest import mock

import pytest

from .helpers import cli
from .helpers.tmp_file_manager import TmpFileManager
from .helpers.utils import check_single

# Define a sample TOML file for testing
sample_toml = """
[info]
  name = "John"
  age = 30

[colors]
  primary = "blue"
  secondary = "green"

[fruits]
  list = ["apple", "orange", "banana"]
"""

sample_config = """
[context.static]
  STAT_TEST_VAR = { value = ["Hello", "World"] }

[context.env]
  ENV_TEST_VAR = { default = "World" }

[context.cli]
  CLI_TEST_VAR = { commands = ["echo 1"], coerce = "int" }
"""


def run_read_config(toml: str, path: str, is_json=False) -> str:
    with TmpFileManager() as manager:
        return cli.run(
            [
                "zetch",
                "read-config",
                path,
                "--config",
                str(manager.tmpfile(toml)),
            ]
            + (["--output", "json"] if is_json else [])
        )


def run_read_var(toml: str, var: str, is_json=False) -> str:
    with TmpFileManager() as manager:
        return cli.run(
            [
                "zetch",
                "read-var",
                var,
                "--config",
                str(manager.tmpfile(toml)),
            ]
            + (["--output", "json"] if is_json else [])
        )


@pytest.mark.parametrize(
    "path, expected_json_result, custom_expected_raw_result",
    [
        # Full:
        (
            "",
            {
                "info": {"name": "John", "age": 30},
                "colors": {"primary": "blue", "secondary": "green"},
                "fruits": {"list": ["apple", "orange", "banana"]},
            },
            None,
        ),
        # Table:
        ("info", {"name": "John", "age": 30}, None),
        # Int:
        ("info.age", 30, None),
        # Str:
        (
            "colors.primary",
            "blue",
            "blue",
        ),  # With raw and json this is different, because json comes out as '"json"' whereas raw shouldn't wrap in quotes.
        # Arr:
        ("fruits.list", ["apple", "orange", "banana"], None),
        ("fruits.list.1", "orange", "orange"),  # Same here with the raw difference
    ],
)
def test_read_config_working(
    path: str, expected_json_result: tp.Any, custom_expected_raw_result: tp.Optional[str]
):
    json_result = json.loads(run_read_config(sample_toml, path, is_json=True))
    assert json_result == expected_json_result
    raw_result = run_read_config(sample_toml, path)
    if custom_expected_raw_result is None:
        assert json.loads(raw_result) == expected_json_result
    else:
        assert raw_result == custom_expected_raw_result


@pytest.mark.parametrize(
    "path, error_message",
    [
        (
            "nonexistent",
            "Failed to read toml path: 'nonexistent'. Failed at: 'root' with error: 'Key 'nonexistent' not found in active table. Avail keys: 'colors, fruits, info'.'",
        ),
        (
            "colors.nonexistent",
            "Failed to read toml path: 'colors.nonexistent'. Failed at: 'colors' with error: 'Key 'nonexistent' not found in active table. Avail keys: 'primary, secondary'.'",
        ),
        (
            "fruits.list.5",
            "Failed to read toml path: 'fruits.list.5'. Failed at: 'fruits.list' with error: 'Index '5' is outside the bounds of the array (len 3).",
        ),
        (
            "fruits.list.-1",
            "Failed to read toml path: 'fruits.list.-1'. Failed at: 'fruits.list' with error: 'Table key '-1' cannot be found. Active element is an array.",
        ),
    ],
)
def test_read_config_fail(path: str, error_message: str):
    with pytest.raises(ValueError, match=re.escape(error_message)):
        run_read_config(sample_toml, path)


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


def test_read_as_input_to_env():
    """Confirm the use case of reading the default, and writing it to the env to use on a run (when ban-defaults is used) works fine.

    Should work from both read-config default and read-var directly.
    """
    config = """
[context.env]
  MY_TEST_VAR = { default = "Hello" }
"""
    # With config accessing default:
    with TmpFileManager() as manager:
        read_val = run_read_config(config, "context.env.MY_TEST_VAR.default")
        with mock.patch.dict(
            os.environ,
            {
                "MY_TEST_VAR": read_val,
            },
        ):
            check_single(
                manager,
                manager.tmpfile(config),
                "{{ MY_TEST_VAR }}!",
                "Hello!",
                extra_args=["--ban-defaults"],
            )

    # Should also work with read-var directly:
    with TmpFileManager() as manager:
        read_val = run_read_var(config, "MY_TEST_VAR")
        with mock.patch.dict(
            os.environ,
            {
                "MY_TEST_VAR": read_val,
            },
        ):
            check_single(
                manager,
                manager.tmpfile(config),
                "{{ MY_TEST_VAR }}!",
                "Hello!",
                extra_args=["--ban-defaults"],
            )
