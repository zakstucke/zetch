import pytest

from ..helpers import cli
from ..helpers.tmp_file_manager import TmpFileManager
from .test_data.utils import tfile


@pytest.mark.parametrize(
    "desc, args, filename, file_contents, file_contents_out_expected",
    [
        # Delete from root object:
        *[
            (f"1_{ft}", ["ree"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '{"ree": "roo"}', "{}"),
                ("yaml", "ree: roo", "{}"),
                ("toml", 'ree = "roo"', ""),
            ]
        ],
        # Delete middle item in root array:
        *[
            (f"2_{ft}", ["1"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '[5, 6, "foo"]', '[\n  5,\n  "foo"\n]\n'),
                ("yaml", "- foo\n- bar\n- baz", "- foo\n- baz\n"),
                # Toml doesn't support arrays at root.
            ]
        ],
        # Delete last item in root array:
        *[
            (f"3_{ft}", ["2"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '[5, 6, "foo"]', "[\n  5,\n  6\n]\n"),
                ("yaml", "- foo\n- bar\n- baz", "- foo\n- bar\n"),
                # Toml doesn't support arrays at root.
            ]
        ],
        # Delete from nested object:
        *[
            (f"4_{ft}", ["ree.roo"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '{"ree": {"roo": ["bar", "baz"]}}', '{\n  "ree": {}\n}\n'),
                ("yaml", "ree:\n  roo:\n  - bar\n  - baz", "ree: {}\n"),
                ("yaml", "ree: {roo: [bar, baz]}", "ree: {}\n"),
                ("toml", 'ree = {roo = ["bar", "baz"]}', "ree = {}\n"),
                ("toml", '[ree]\nroo = ["bar", "baz"]', "[ree]\n"),
            ]
        ],
        # Delete middle item from nested array:
        *[
            (f"5_{ft}", ["ree.roo.0"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                (
                    "json",
                    '{"ree": {"roo": ["bar", "baz"]}}',
                    '{\n  "ree": {\n    "roo": ["baz"]\n  }\n}\n',
                ),
                ("yaml", "ree:\n  roo:\n  - bar\n  - baz", "ree:\n  roo:\n  - baz\n"),
                ("yaml", "ree: {roo: [bar, baz]}", "ree: {roo: [baz]}\n"),
                ("toml", 'ree = {roo = ["bar", "baz"]}', 'ree = {roo = [ "baz"]}\n'),
                ("toml", '[ree]\nroo = ["bar", "baz"]', '[ree]\nroo = [ "baz"]\n'),
                # Toml array of tables:
                (
                    "toml",
                    '[[ree.roo]]\nfoo = "bar"\n\n[[ree.roo]]\nfoo = "baz"',
                    '[[ree.roo]]\nfoo = "baz"\n',
                ),
            ]
        ],
        # Deleting end item from nested array:
        *[
            (f"6_{ft}", ["ree.roo.1"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                (
                    "json",
                    '{"ree": {"roo": ["bar", "baz"]}}',
                    '{\n  "ree": {\n    "roo": ["bar"]\n  }\n}\n',
                ),
                ("yaml", "ree:\n  roo:\n  - bar\n  - baz", "ree:\n  roo:\n  - bar\n"),
                ("yaml", "ree: {roo: [bar, baz]}", "ree: {roo: [bar]}\n"),
                ("toml", 'ree = {roo = ["bar", "baz"]}', 'ree = {roo = ["bar"]}\n'),
                ("toml", '[ree]\nroo = ["bar", "baz"]', '[ree]\nroo = ["bar"]\n'),
                # Toml array of tables:
                (
                    "toml",
                    '[[ree.roo]]\nfoo = "bar"\n\n[[ree.roo]]\nfoo = "baz"',
                    '[[ree.roo]]\nfoo = "bar"\n',
                ),
            ]
        ],
        # Complex with comments etc should all be maintained on rewrite:
        *[
            (f"7_{ft}", ["phoneNumbers.1.type"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", tfile("complex.json"), tfile("complex_delete_out.json")),
                ("toml", tfile("complex.toml"), tfile("complex_delete_out.toml")),
                ("yaml", tfile("complex.yaml"), tfile("complex_delete_out.yaml")),
            ]
        ],
        # No error or file change when target key doesn't exist:
        *[
            (f"8_{ft}", ["ree.roo"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '{"ree": {"bar": "baz"}}', '{"ree": {"bar": "baz"}}'),
                ("yaml", "ree:\n  bar: baz", "ree:\n  bar: baz"),
                ("toml", 'ree = {bar = "baz"}', 'ree = {bar = "baz"}'),
            ]
        ],
        # No error or file change when intermediary key doesn't exist:
        *[
            (f"9_{ft}", ["ree.roo.bar"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '{"ree": {"bar": "baz"}}', '{"ree": {"bar": "baz"}}'),
                ("yaml", "ree:\n  bar: baz", "ree:\n  bar: baz"),
                ("toml", 'ree = {bar = "baz"}', 'ree = {bar = "baz"}'),
            ]
        ],
    ],
)
def test_file_cmd_delete(
    desc: str,
    args: "list[str]",
    filename: str,
    file_contents: str,
    file_contents_out_expected: str,
):
    with TmpFileManager() as manager:
        filepath = manager.tmpfile(file_contents, full_name=filename)

        result = cli.run(["zetch", "file", str(filepath), *args, "--delete"])

        # Confirm contents has been updated correctly:
        if filepath.read_text().strip() != file_contents_out_expected.strip():
            raise AssertionError(
                "Delete mismatch! (Desc: {}) \nExpected:\n'{}'\n\nGot:\n'{}'\n\nNon escaped expected:\n{}\n\nNon escaped got:\n{}\n\nStd all:\n{}".format(
                    desc,
                    file_contents_out_expected.encode("unicode_escape").decode("utf-8"),
                    filepath.read_text().encode("unicode_escape").decode("utf-8"),
                    file_contents_out_expected,
                    filepath.read_text(),
                    result,
                )
            )
