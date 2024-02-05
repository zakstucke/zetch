import pytest

from ..helpers import cli
from ..helpers.tmp_file_manager import TmpFileManager


@pytest.mark.parametrize(
    "desc, args, filename, file_contents, stdout_expected",
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
    ],
)
def test_file_cmd_inference(
    desc: str,
    args: "list[str]",
    filename: str,
    file_contents: str,
    stdout_expected: str,
):
    with TmpFileManager() as manager:
        filepath = manager.tmpfile(file_contents, full_name=filename)

        result = cli.run(["zetch", "file", str(filepath), *args])

        assert result == stdout_expected

        # Confirm contents hasn't changed:
        assert filepath.read_text() == file_contents
