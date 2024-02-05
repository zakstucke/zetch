import json

import pytest

from ..helpers import cli
from ..helpers.tmp_file_manager import TmpFileManager
from .test_data.utils import tfile


@pytest.mark.parametrize(
    "desc, put_val, args, filename, file_contents, file_contents_out_expected",
    [
        # Modify root object:
        *[
            (f"1_{ft}", "hello!", ["ree"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '{"ree": "roo"}', '{\n  "ree": "hello!"\n}\n'),
                ("yaml", "ree: roo", "ree: hello!\n"),
                ("toml", 'ree = "roo"', 'ree = "hello!"\n'),
            ]
        ],
        # Modify index in root array:
        *[
            (f"2_{ft}", "hello!", ["2"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '[5, 6, "foo"]', '[\n  5,\n  6,\n  "hello!"\n]\n'),
                ("yaml", "- foo\n- bar\n- baz", "- foo\n- bar\n- hello!\n"),
                # Toml doesn't support arrays at root.
            ]
        ],
        # Push into root array: (index 1 out of bounds treated as push)
        *[
            (f"3_{ft}", "hello!", ["1"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '["foo"]', '[\n  "foo",\n  "hello!"\n]\n'),
                ("yaml", "- foo", "- foo\n- hello!\n"),
                # Toml doesn't support arrays at root.
            ]
        ],
        # Modify nested object:
        *[
            (f"4_{ft}", "hello!", ["ree.roo"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                (
                    "json",
                    '{"ree": {"roo": ["bar", "baz"]}}',
                    '{\n  "ree": { "roo": "hello!" }\n}\n',
                ),
                ("toml", 'ree = {roo = ["bar", "baz"]}', 'ree = {roo = "hello!"}\n'),
                ("toml", '[ree]\nroo = ["bar", "baz"]', '[ree]\nroo = "hello!"\n'),
                ("yaml", "ree:\n  roo:\n  - bar\n  - baz", "ree:\n  roo: hello!\n"),
                ("yaml", "ree: {roo: [bar, baz]}", "ree: {roo: hello!}\n"),
            ]
        ],
        # Modify index in nested array:
        *[
            (
                f"5_{ft}",
                json.dumps({"foo": "bar"}),
                ["ree.roo.0", "--coerce=json"],
                f"foo.{ft}",
                cont,
                out,
            )
            for ft, cont, out in [
                (
                    "json",
                    '{"ree": {"roo": ["bar", "baz"]}}',
                    '{\n  "ree": {\n    "roo": [\n      { "foo": "bar" },\n      "baz"\n    ]\n  }\n}\n',
                ),
                ("yaml", "ree:\n  roo:\n  - bar\n  - baz", "ree:\n  roo:\n  - foo: bar\n  - baz\n"),
                (
                    "toml",
                    'ree = {roo = ["bar", "baz"]}',
                    'ree = {roo = [{ foo = "bar" }, "baz"]}\n',
                ),
                # Toml array of tables:
                (
                    "toml",
                    '[[ree.roo]]\nfoo = "baz"\n\n[[ree.roo]]\nfoo = "boo"',
                    '[[ree.roo]]\nfoo = "bar"\n\n[[ree.roo]]\nfoo = "boo"\n',
                ),
            ]
        ],
        # Push into nested array: (index 1 out of bounds treated as push)
        *[
            (
                f"6_{ft}",
                json.dumps({"foo": "bar"}),
                ["ree.1", "--coerce=json"],
                f"foo.{ft}",
                cont,
                out,
            )
            for ft, cont, out in [
                (
                    "json",
                    '{"ree": ["bar"]}',
                    '{\n  "ree": [\n    "bar",\n    { "foo": "bar" }\n  ]\n}\n',
                ),
                ("yaml", "ree:\n- bar", "ree:\n- bar\n- foo: bar\n"),
                ("toml", 'ree = ["bar"]', 'ree = ["bar", { foo = "bar" }]\n'),
                # Toml array of tables:
                (
                    "toml",
                    '[[ree]]\nroo = "baz"',
                    '[[ree]]\nroo = "baz"\n\n[[ree]]\nfoo = "bar"\n',
                ),
            ]
        ],
        # Complex file should maintain full structure on edit (including comments):
        *[
            (f"7_{ft}", "NEW_TYPE", ["phoneNumbers.1.type"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", tfile("complex.json"), tfile("complex_put_out.json")),
                ("toml", tfile("complex.toml"), tfile("complex_put_out.toml")),
                ("yaml", tfile("complex.yaml"), tfile("complex_put_out.yaml")),
            ]
        ],
        # Missing intermediary object keys should be auto created:
        *[
            (f"8_{ft}", "NEW", ["ree.roo.baz.bar"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                (
                    "json",
                    '{"ree": {}}',
                    '{\n  "ree": {\n    "roo": {\n      "baz": { "bar": "NEW" }\n    }\n  }\n}\n',
                ),
                ("yaml", "ree: {}", "ree:\n  roo:\n    baz:\n      bar: NEW\n"),
                ("toml", "ree = {}", 'ree = { roo = { baz = { bar = "NEW" } } }\n'),
            ]
        ],
    ],
)
def test_file_cmd_put(
    desc: str,
    put_val: str,
    args: "list[str]",
    filename: str,
    file_contents: str,
    file_contents_out_expected: str,
):
    with TmpFileManager() as manager:
        filepath = manager.tmpfile(file_contents, full_name=filename)

        result = cli.run(["zetch", "file", str(filepath), *args, "--put", put_val])

        # Confirm contents has been updated correctly:
        if filepath.read_text().strip() != file_contents_out_expected.strip():
            raise AssertionError(
                "Put mismatch! (Desc: {}) \nExpected:\n'{}'\n\nGot:\n'{}'\n\nNon escaped expected:\n{}\n\nNon escaped got:\n{}\n\nStd all:\n{}".format(
                    desc,
                    file_contents_out_expected.encode("unicode_escape").decode("utf-8"),
                    filepath.read_text().encode("unicode_escape").decode("utf-8"),
                    file_contents_out_expected,
                    filepath.read_text(),
                    result,
                )
            )
