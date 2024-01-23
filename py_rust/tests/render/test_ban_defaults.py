import os
import typing as tp
from unittest import mock

import pytest

from ..helpers.tmp_file_manager import TmpFileManager
from ..helpers.utils import check_single


@pytest.mark.parametrize(
    "extra_args, env, expected, err",
    [
        # Sanity check:
        ([], {}, "Hello, World!", None),
        # Basic should fail:
        (
            ["--ban-defaults", "TEST_RAND_1"],
            {},
            "",
            "Could not find environment variable 'TEST_RAND_1' and the default has been banned using the 'ban-defaults' cli option.",
        ),
        # Check multiple work, where both are banned but the first is avail (so only the second should complain):
        (
            ["--ban-defaults", "TEST_RAND_1", "--ban-defaults", "TEST_RAND_2"],
            {"TEST_RAND_1": "REE"},
            "",
            "Could not find environment variable 'TEST_RAND_2' and the default has been banned using the 'ban-defaults' cli option.",
        ),
        # Same again but with only one --ban-defaults should be able to list them with spaces:
        (
            ["--ban-defaults", "TEST_RAND_1", "TEST_RAND_2"],
            {"TEST_RAND_1": "REE"},
            "",
            "Could not find environment variable 'TEST_RAND_2' and the default has been banned using the 'ban-defaults' cli option.",
        ),
        # Same again but should be able to separate with commas:
        (
            ["--ban-defaults", "TEST_RAND_1,TEST_RAND_2"],
            {"TEST_RAND_1": "REE"},
            "",
            "Could not find environment variable 'TEST_RAND_2' and the default has been banned using the 'ban-defaults' cli option.",
        ),
        # Same again and equals should work too:
        (
            ["--ban-defaults", "TEST_RAND_1,TEST_RAND_2"],
            {"TEST_RAND_1": "REE"},
            "",
            "Could not find environment variable 'TEST_RAND_2' and the default has been banned using the 'ban-defaults' cli option.",
        ),
        # When no vars provided, all defaults should be ignored:
        (
            ["--ban-defaults"],
            {"TEST_RAND_1": "REE"},
            "",
            "Could not find environment variable 'TEST_RAND_2' and the default has been banned using the 'ban-defaults' cli option.",
        ),
        # Should now work when both are provided from the env:
        (
            ["--ban-defaults", "TEST_RAND_1,TEST_RAND_2"],
            {"TEST_RAND_1": "REE", "TEST_RAND_2": "ROO"},
            "REE, ROO!",
            None,
        ),
        # Should err when an unrecognized variable is provided:
        (
            ["--ban-defaults", "TEST_RAND_1,IDONTEXIST"],
            {},
            "",
            "Unrecognized context.env var provided to '--ban-defaults': 'IDONTEXIST'. All env vars in config: 'TEST_RAND_1, TEST_RAND_2'.",
        ),
    ],
)
def test_ban_defaults(extra_args: list[str], env: dict, expected: str, err: tp.Optional[str]):
    # Ban a default and should fail as isn't set:
    with TmpFileManager() as manager:
        with mock.patch.dict(os.environ, env):

            def run():
                check_single(
                    manager,
                    manager.create_cfg(
                        {
                            "context": {
                                "env": {
                                    "TEST_RAND_1": {"default": "Hello"},
                                    "TEST_RAND_2": {"default": "World"},
                                }
                            }
                        }
                    ),
                    "{{ TEST_RAND_1 }}, {{ TEST_RAND_2 }}!",
                    expected,
                    extra_args=extra_args,
                )

            if err is None:
                run()
            else:
                with pytest.raises(ValueError, match=err):
                    run()
