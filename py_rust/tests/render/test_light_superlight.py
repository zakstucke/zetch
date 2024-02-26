import os
import pathlib
import typing as tp

import pytest

from ..helpers import utils
from ..helpers.tmp_file_manager import TmpFileManager
from ..helpers.types import InputConfig

# light/superlight TODO:
# - At the moment if e.g. filters like |items are used, light values will have to be configured to prevent breaking these filters, need some way to internally properly render these out.
# - Same as above but for e.g. for loops.
# - If above can be solved, extending that to allow full partial rendering, i.e. only sections not containing cli vars are rendered would make this a very powerful feature.
# - If above 3 can be implemented, --light can be run automatically on a cli var failing, to automatically fix render cycles. This would be kind've possible before, but would then require the user to make all their cli vars light compatible (given the above current restriction with filters, for loops etc.).


@pytest.mark.parametrize(
    "config, src, expected",
    [
        # Should be replaced by empty string when no light value set:
        ({"context": {"cli": {"VAR": {"commands": ["echo foo"]}}}}, "var: {{ VAR }}", "var: "),
        # Light value should work:
        (
            {"context": {"cli": {"VAR": {"commands": ["echo foo"], "light": {"value": "LIGHT"}}}}},
            "var: {{ VAR }}",
            "var: LIGHT",
        ),
        # Env and static vars should work:
        (
            {
                "context": {
                    "static": {"VAR_STATIC": {"value": "STATIC"}},
                    "env": {"VAR_ENV": {"default": {"value": "ENV"}}},
                    "cli": {
                        "VAR": {
                            "commands": ["echo foo"],
                            "light": {"value": "LIGHT"},
                        }
                    },
                }
            },
            "{{ VAR }} {{ VAR_ENV }} {{ VAR_STATIC }}",
            "LIGHT ENV STATIC",
        ),
        # Pre and post tasks should be completely ignored:
        (
            {
                "tasks": {
                    "pre": [{"commands": ["exit 1"]}],
                    "post": [{"commands": ["exit 1"]}],
                },
                "context": {"cli": {"VAR": {"commands": ["echo foo"]}}},
            },
            "var: {{ VAR }}",
            "var: ",
        ),
    ],
)
def test_light_superlight_shared(config: InputConfig, src: str, expected: str):
    """Confirm shared behaviour between light and superlight."""

    def test(superlight: bool):
        with TmpFileManager() as manager:
            utils.check_single(
                manager,
                manager.create_cfg(config),
                src,
                expected,
                extra_args=["--superlight" if superlight else "--light"],
            )

    # These should be the same for superlight and light modes:
    test(False)
    test(True)


@pytest.mark.parametrize(
    "config, src, expected, cb",
    [
        # Custom extensions should still be respected:
        (
            {
                "engine": {"custom_extensions": ["./ext.py"]},
            },
            "out: {{ add_2(2) }}",
            "out: 4",
            lambda m: m.tmpfile(
                """import zetch
@zetch.register_function
def add_2(num):
    return num + 2
""",
                full_name="ext.py",
            ),
        ),
    ],
)
def test_light_only(
    config: InputConfig, src: str, expected: str, cb: tp.Callable[[TmpFileManager], tp.Any]
):
    """Confirm shared behaviour between light and superlight."""
    with TmpFileManager() as manager:
        cb(manager)
        utils.check_single(
            manager,
            manager.create_cfg(config),
            src,
            expected,
            extra_args=["--light"],
        )


@pytest.mark.parametrize(
    "config, src, expected, cb",
    [
        # Custom extensions shouldn't be run and just return empty strings:
        (
            {
                "engine": {"custom_extensions": ["./ext.py"]},
            },
            "out: {{ add_2(2) }}",
            "out: ",
            lambda m: m.tmpfile(
                """import zetch
@zetch.register_function
def add_2(num):
    return num + 2
""",
                full_name="ext.py",
            ),
        ),
    ],
)
def test_superlight_only(
    config: InputConfig, src: str, expected: str, cb: tp.Callable[[TmpFileManager], tp.Any]
):
    """Confirm shared behaviour between light and superlight."""
    with TmpFileManager() as manager:
        cb(manager)
        utils.check_single(
            manager,
            manager.create_cfg(config),
            src,
            expected,
            extra_args=["--superlight"],
        )


def test_light_fixes_circular_dep():
    """Make sure that running with --light or --superlight, then running again normally would fix a circular dependency in cli commands."""

    def run(light: bool):
        with TmpFileManager() as manager:
            manager.tmpfile("Hello, World!", full_name="circ_dep.zetch.txt")
            config: InputConfig = {
                "context": {
                    "cli": {
                        "VAR": {
                            "commands": [
                                '{} "{}"'.format(
                                    utils.cat_cmd_cross(),
                                    utils.str_path_for_tmpl_writing(
                                        pathlib.Path(os.path.join(manager.root_dir, "circ_dep.txt"))
                                    ),
                                )
                            ],
                            "light": {"value": "LIGHT"},
                        },
                    }
                }
            }

            if light:
                utils.check_single(
                    manager,
                    manager.create_cfg(config),
                    "{{ VAR }}",
                    "LIGHT",
                    extra_args=["--light"],
                    ignore_extra_written=True,
                )

            utils.check_single(
                manager,
                manager.create_cfg(config),
                "{{ VAR }}",
                "Hello, World!",
                ignore_extra_written=True,
            )

    # Should fail due to circular dependency without a previous light/superlight run:
    with pytest.raises(ValueError, match=utils.no_file_err_cross("circ_dep.txt")):
        run(False)

    # Should work with light/superlight:
    run(True)
