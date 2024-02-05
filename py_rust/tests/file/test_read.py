import pytest

from ..helpers import cli
from ..helpers.tmp_file_manager import TmpFileManager
from .test_data.utils import tfile


@pytest.mark.parametrize(
    "desc, args, filename, file_contents, stdout_expected",
    [
        # Root object key:
        *[
            (f"1_{ft}", ["ree"], f"foo.{ft}", cont, "roo")
            for ft, cont in [
                ("json", '{"ree": "roo"}'),
                ("toml", 'ree = "roo"'),
                ("yaml", "ree: roo"),
            ]
        ],
        # Middle index in root array:
        *[
            (f"2_{ft}", ["1"], f"foo.{ft}", cont, "baz")
            for ft, cont in [
                ("json", '["bar", "baz", "foo"]'),
                ("yaml", "- bar\n- baz\n- foo"),
                # Toml doesn't support arrays at root.
            ]
        ],
        # End index in root array:
        *[
            (f"3_{ft}", ["2"], f"foo.{ft}", cont, "foo")
            for ft, cont in [
                ("json", '["bar", "baz", "foo"]'),
                ("yaml", "- bar\n- baz\n- foo"),
                # Toml doesn't support arrays at root.
            ]
        ],
        # Nested object key:
        *[
            (f"4_{ft}", ["ree.roo"], f"foo.{ft}", cont, "bar")
            for ft, cont in [
                ("json", '{"ree": {"roo": "bar"}}'),
                ("toml", 'ree = {roo = "bar"}'),
                ("toml", '[ree]\nroo = "bar"'),
                ("yaml", "ree:\n  roo: bar"),
                ("yaml", "ree: {roo: bar}"),
            ]
        ],
        # Nested middle array index:
        *[
            (f"5_{ft}", ["ree.roo.1"], f"foo.{ft}", cont, "baz")
            for ft, cont in [
                ("json", '{"ree": {"roo": ["bar", "baz"]}}'),
                ("toml", 'ree = {roo = ["bar", "baz"]}'),
                ("yaml", "ree:\n  roo:\n  - bar\n  - baz"),
                ("yaml", "ree: {roo: [bar, baz]}"),
            ]
        ],
        # Nested end array index:
        *[
            (f"6_{ft}", ["ree.roo.1"], f"foo.{ft}", cont, "baz")
            for ft, cont in [
                ("json", '{"ree": {"roo": ["bar", "baz"]}}'),
                ("toml", 'ree = {roo = ["bar", "baz"]}'),
                ("yaml", "ree:\n  roo:\n  - bar\n  - baz"),
                ("yaml", "ree: {roo: [bar, baz]}"),
            ]
        ],
        # Root object key with specified raw output:
        *[
            (f"7_{ft}", ["ree", "--output=raw"], f"foo.{ft}", cont, "roo")
            for ft, cont in [
                ("json", '{"ree": "roo"}'),
                ("toml", 'ree = "roo"'),
                ("yaml", "ree: roo"),
            ]
        ],
        # Root object key with specified json output:
        *[
            (f"8_{ft}", ["ree", "--output=json"], f"foo.{ft}", cont, '"roo"')
            for ft, cont in [
                ("json", '{"ree": "roo"}'),
                ("toml", 'ree = "roo"'),
                ("yaml", "ree: roo"),
            ]
        ],
        # Read non-basic object:
        *[
            (f"5_{ft}", ["ree"], f"foo.{ft}", cont, '{"roo":"bar"}')
            for ft, cont in [
                ("json", '{"ree": {"roo": "bar"}}'),
                ("toml", 'ree = {roo = "bar"}'),
                ("toml", '[ree]\nroo = "bar"'),
                ("yaml", "ree:\n  roo: bar"),
            ]
        ],
        # Read non-basic array:
        *[
            (f"6_{ft}", ["ree"], f"foo.{ft}", cont, '["roo","bar"]')
            for ft, cont in [
                ("json", '{"ree": ["roo", "bar"]}'),
                ("toml", 'ree = ["roo", "bar"]'),
                ("yaml", "ree:\n- roo\n- bar"),
                ("yaml", "ree: [roo, bar]"),
            ]
        ],
        # Complex file with comments etc.
        *[
            (f"7_{ft}", ["phoneNumbers.1.type"], f"foo.{ft}", cont, "work")
            for ft, cont in [
                ("json", tfile("complex.json")),
                ("toml", tfile("complex.toml")),
                ("yaml", tfile("complex.yaml")),
            ]
        ],
    ],
)
def test_file_cmd_read(
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
