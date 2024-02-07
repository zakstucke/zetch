import json
import pathlib
import typing as tp

import pytest

from ..helpers import cli
from ..helpers.tmp_file_manager import TmpFileManager


def j(manager: TmpFileManager, contents: tp.Any):
    manager.tmpfile(json.dumps(contents), full_name="foo.json")


""" Run usage error checks.

The nice thing about the internal implementation is these errors should all come from the generic traverser.
Meaning we can run all these checks on one lang variant. (apart from any special cases specific to certain langs)
"""


@pytest.mark.parametrize(
    "desc, args, setup, err_expected",
    [
        # Generic:
        ("missing_file", ["read", "non_existant.json", "foo.bar"], None, "FileNotFound"),
        ("invalid_path_1", ["read", "foo.json", ".."], lambda man: j(man, {}), "FilePathError"),
        ("invalid_path_2", ["read", "foo.json", "[]//"], lambda man: j(man, {}), "FilePathError"),
        ("empty_path_1", ["read", "foo.json", ""], lambda man: j(man, {}), "FilePathError"),
        ("empty_path_1", ["read", "foo.json", "."], lambda man: j(man, {}), "FilePathError"),
        # Read:
        ("read_path_missing", ["read", "foo.json", "ree"], lambda man: j(man, {}), "FilePathError"),
        # Put:
        (
            # Don't check missing, is fine for put, just out of bounds: (more than 1, as one is push)
            "put_path_arr_oob",
            ["put", "foo.json", "ree.9", "foo"],
            lambda man: j(man, {"ree": [1, 2, 3]}),
            "FilePathError",
        ),
        # Delete:
        # No missing or oob checks for delete, both will be treated as no-op.
        # Special case for toml, trying to put a none object to an array of tables:
        (
            "put_toml_table_array_invalid_value",
            ["put", "foo.toml", "ree.0", "foo"],
            lambda man: man.tmpfile("[[ree]]\nfoo = 1\n\n[[ree]]\nfoo = 2", full_name="foo.toml"),
            "InvalidPutValue",
        ),
    ],
)
def test_file_cmd_invalid(
    desc: str,
    args: "list[str]",
    setup: "tp.Callable[[TmpFileManager], tp.Any]",
    err_expected: str,
):
    with TmpFileManager() as manager:
        if setup:
            setup(manager)

        with pytest.raises(ValueError, match=err_expected):
            cli.run(["zetch", *args], custom_root=pathlib.Path(manager.root_dir))
