import json
import os
import time
import typing as tp
from unittest import mock

import pytest
import zetch

from .helpers import cli, utils
from .helpers.tmp_file_manager import TmpFileManager
from .helpers.types import InputConfig


def cfg_str(config: InputConfig) -> str:
    return zetch._toml_create(config)


@pytest.mark.parametrize(
    "env,config_var,cfg_str,expected",
    [
        (
            {},
            "ctx",
            cfg_str(
                {
                    "context": {
                        "cli": {
                            "FOO": {
                                "commands": ['echo "Hello, World!"'],
                            }
                        }
                    }
                }
            ),
            {"FOO": "Hello, World!"},
        ),
        (
            {},
            "ctx",
            cfg_str(
                {
                    "context": {
                        "cli": {
                            "FOO": {
                                "commands": [
                                    'echo "Ignore me I\'m different!"',
                                    'echo "Hello, World!"',
                                ]
                            }
                        }
                    }
                }
            ),
            {"FOO": "Hello, World!"},
        ),
        (
            {},
            "ctx",
            cfg_str({"context": {"static": {"FOO": {"value": "Hello, World!"}}}}),
            {"FOO": "Hello, World!"},
        ),
        (
            {"FOO": "abc"},
            "ctx",
            cfg_str({"context": {"env": {"FOO": {}}}}),
            {"FOO": "abc"},
        ),
        (
            {"BAR": "def"},
            "ctx",
            cfg_str({"context": {"env": {"FOO": {"env_name": "BAR"}}}}),
            {"FOO": "def"},
        ),
        # Should still use env var if available despite default given:
        (
            {"FOO": "abc"},
            "ctx",
            cfg_str({"context": {"env": {"FOO": {"default": True}}}}),
            {"FOO": "abc"},
        ),
        # Should only use default when no env var:
        (
            {},
            "ctx",
            cfg_str({"context": {"env": {"FOO": {"env_name": "BAR", "default": True}}}}),
            {"FOO": True},
        ),
        (
            {},
            "ctx",
            cfg_str(
                {
                    "context": {
                        "cli": {
                            "FOO": {
                                "commands": ['echo "Hello, World!"'],
                            },
                            "BAR": {
                                "commands": ['echo "Goodbye, World!"'],
                            },
                        },
                        "static": {"BAZ": {"value": "INLINE"}},
                    },
                }
            ),
            {"FOO": "Hello, World!", "BAR": "Goodbye, World!", "BAZ": "INLINE"},
        ),
        (
            {},
            "ignore_files",
            cfg_str({"ignore_files": ["ignorefile.txt"]}),
            lambda root_dir: [os.path.join(root_dir, "ignorefile.txt")],
        ),
        ({}, "exclude", cfg_str({"exclude": [".*", "foo.bar"]}), [".*", "foo.bar"]),
        (
            {},
            "engine",
            cfg_str(
                {
                    "engine": {
                        "comment_start": "|||/",
                        "comment_end": "/|||",
                    }
                }
            ),
            {
                "block_start": "{%",
                "block_end": "%}",
                "variable_start": "{{",
                "variable_end": "}}",
                "comment_start": "|||/",
                "comment_end": "/|||",
                "custom_extensions": [],
            },
        ),
        # Matchers:
        (
            {},
            "matchers",
            # Defaults to zetch
            cfg_str({}),
            ["zetch"],
        ),
        (
            {},
            "matchers",
            # lowercase, numbers, dashes, underscores should be allowed:
            cfg_str({"matchers": ["foo", "foo-bar_ree", "d77"]}),
            ["foo", "foo-bar_ree", "d77"],
        ),
        # Tasks:
        (
            {},
            "tasks",
            cfg_str({}),
            {
                "pre": [],
                "post": [],
            },
        ),
        (
            {},
            "tasks",
            cfg_str(
                {
                    "tasks": {
                        "post": [],
                    }
                }
            ),
            {
                "pre": [],
                "post": [],
            },
        ),
        (
            {},
            "tasks",
            cfg_str(
                {
                    "tasks": {
                        "pre": [
                            {
                                "commands": ["echo 'Building...'"],
                            }
                        ]
                    }
                }
            ),
            {
                "pre": [
                    {
                        "commands": ["echo 'Building...'"],
                    }
                ],
                "post": [],
            },
        ),
    ],
)
def test_read_config(
    env: "dict[str, str]",
    config_var: str,
    cfg_str: str,
    expected: tp.Union[str, tp.Callable[[str], tp.List[str]]],
):
    """Confirm various config setups are all read and processed correctly."""
    with TmpFileManager() as manager:
        with mock.patch.dict(os.environ, env):
            # Make sure a .gitignore exists as one of the variations needs it:
            if config_var == "ignore_files":
                manager.tmpfile("foo", full_name="ignorefile.txt")

            final_expected = expected(manager.root_dir) if callable(expected) else expected

            debug = cli.render(manager.root_dir, manager.tmpfile(cfg_str, suffix=".toml"))["debug"]
            # Some things moved to "conf" that used to be on state:
            out = debug[config_var] if config_var in debug else debug["conf"][config_var]
            assert out == final_expected


def test_parallelized_context_cli_commands():
    """Confirm cli commands are processed in parallel for different variables.

    External commands are one of the slowest parts of the system, zetch should attempts to remedy by running different ctx commands in parallel.
    """
    with TmpFileManager() as manager:
        before = time.time()
        utils.check_single(
            manager,
            manager.create_cfg(
                {
                    "context": {
                        "cli": {
                            "FOO": {
                                "commands": [
                                    "sleep 0.5",
                                    'echo "MY_FOO"',
                                ],
                            },
                            "BAR": {
                                "commands": [
                                    "sleep 0.5",
                                    'echo "MY_BAR"',
                                ],
                            },
                            "BAZ": {
                                "commands": [
                                    "sleep 0.5",
                                    'echo "MY_BAZ"',
                                ],
                            },
                            "QUX": {
                                "commands": [
                                    "sleep 0.5",
                                    'echo "MY_QUX"',
                                ],
                            },
                        }
                    }
                }
            ),
            "{{ FOO }} {{ BAR }} {{ BAZ }} {{ QUX }}",
            "MY_FOO MY_BAR MY_BAZ MY_QUX",
        )
        time_taken = time.time() - before
        # Should be just above 0.5, but allow decent leeway:
        assert time_taken < 1


@pytest.mark.parametrize(
    "as_type,input_val,expected",
    [
        ("str", "123", "123"),
        ("int", "123", 123),
        ("int", "123.34", 123),
        ("int", 123.34, 123),
        ("float", "123.456", 123.456),
        ("bool", "true", True),
        ("bool", "True", True),
        ("bool", "y", True),
        ("bool", "false", False),
        ("json", json.dumps({"foo": "bar"}), {"foo": "bar"}),
    ],
)
def test_valid_coercion(as_type: tp.Any, input_val: tp.Any, expected: tp.Any):
    """Confirm value conversion works correctly when valid in all input types."""
    with TmpFileManager() as manager:
        # Test static, cli and env variants:

        assert (
            cli.render(
                manager.root_dir,
                manager.create_cfg(
                    {"context": {"static": {"FOO": {"value": input_val, "coerce": as_type}}}}
                ),
            )["debug"]["ctx"]["FOO"]
            == expected
        )

        tmpfile = manager.tmpfile(str(input_val), suffix=".txt")
        assert (
            cli.render(
                manager.root_dir,
                manager.create_cfg(
                    {
                        "context": {
                            "cli": {
                                "FOO": {
                                    "commands": [
                                        '{} "{}"'.format(
                                            utils.cat_cmd_cross(),
                                            utils.str_path_for_tmpl_writing(tmpfile),
                                        )
                                    ],
                                    "coerce": as_type,
                                }
                            }
                        }
                    }
                ),
            )["debug"]["ctx"]["FOO"]
            == expected
        )

        with mock.patch.dict(
            os.environ,
            {
                "FOO": str(input_val),
            },
        ):
            assert (
                cli.render(
                    manager.root_dir,
                    manager.create_cfg(
                        {"context": {"env": {"FOO": {"env_name": "FOO", "coerce": as_type}}}},
                    ),
                )["debug"]["ctx"]["FOO"]
                == expected
            )
