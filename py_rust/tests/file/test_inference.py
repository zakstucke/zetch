import typing as tp

import pytest

from ..helpers import cli
from ..helpers.tmp_file_manager import TmpFileManager


@pytest.mark.parametrize(
    # Params are made in a way that allows test cases for both file and inline content.
    "desc, args, fn_or_contents, maybe_file_contents, stdout_expected",
    [
        # Auto inference based on filename:
        *[
            (f"1_{ft}", ["ree"], f"foo.{ft}", cont, "roo")
            for ft, cont in [
                ("json", '{"ree": "roo"}'),
                ("toml", 'ree = "roo"'),
                ("yaml", "ree: roo"),
                # Should also work with yml:
                ("yml", "ree: roo"),
            ]
        ],
        # Auto inference based on only one type successfully decoding content:
        *[
            (f"2_{ft}", ["ree.bar"], "arb_filename", cont, "roo")
            for ft, cont in [
                # Yaml is a superset of json, so json can't be inferred.
                ("toml", '[ree]\nbar = "roo"'),
                ("yaml", "ree:\n  bar: roo"),
            ]
        ],
        # Manual type specification:
        *[
            (f"3_{ft}", ["ree", f"--{ft}"], "arb_filename", cont, "roo")
            for ft, cont in [
                ("json", '{"ree": "roo"}'),
                ("toml", 'ree = "roo"'),
                ("yaml", "ree: roo"),
                # Should also work with yml:
                ("yml", "ree: roo"),
            ]
        ],
        # Inline content: Auto inference based on only one type successfully decoding content:
        *[
            (f"4_{ft}", ["ree.bar"], cont, None, "roo")
            for ft, cont in [
                # Yaml is a superset of json, so json can't be inferred.
                ("toml", '[ree]\nbar = "roo"'),
                ("yaml", "ree:\n  bar: roo"),
            ]
        ],
        # Inline content: Manual type specification:
        *[
            (f"3_{ft}", ["ree", f"--{ft}"], cont, None, "roo")
            for ft, cont in [
                ("json", '{"ree": "roo"}'),
                ("toml", 'ree = "roo"'),
                ("yaml", "ree: roo"),
                # Should also work with yml:
                ("yml", "ree: roo"),
            ]
        ],
    ],
)
def test_file_cmd_inference(
    desc: str,
    args: "list[str]",
    fn_or_contents: str,
    maybe_file_contents: tp.Optional[str],
    stdout_expected: str,
):
    with TmpFileManager() as manager:
        if maybe_file_contents is not None:
            filepath = manager.tmpfile(maybe_file_contents, full_name=fn_or_contents)
        else:
            filepath = fn_or_contents

        result = cli.run(["zetch", "read", str(filepath), *args])

        assert result == stdout_expected

        if maybe_file_contents is not None and not isinstance(filepath, str):
            # Confirm contents hasn't changed:
            assert filepath.read_text() == maybe_file_contents
