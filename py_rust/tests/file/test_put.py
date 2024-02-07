import json

import pytest

from ..helpers import cli, utils
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
                (
                    "yaml",
                    # The replace is to fix a weird windows issue in CI, no idea why single newline is output on windows...
                    tfile("complex.yaml").replace("\n\nphoneNumbers", "\nphoneNumbers"),
                    tfile("complex_put_out.yaml").replace("\n\nphoneNumbers", "\nphoneNumbers"),
                ),
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
        # Untouched on identical object key put:
        *[
            (f"9_{ft}", '{"foo": [1, 2, 3]}', ["ree", "--coerce=json"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                ("json", '{"ree": {"foo": [1, 2, 3]}}', '{"ree": {"foo": [1, 2, 3]}}'),
                ("yaml", "ree:\n  foo:\n  - 1\n  - 2\n  - 3", "ree:\n  foo:\n  - 1\n  - 2\n  - 3"),
                ("toml", "ree = {foo = [1, 2, 3]}", "ree = {foo = [1, 2, 3]}"),
            ]
        ],
        # Untouched on identical array index put:
        *[
            (f"10_{ft}", '{"foo": [1, 2, 3]}', ["ree.0", "--coerce=json"], f"foo.{ft}", cont, out)
            for ft, cont, out in [
                (
                    "json",
                    '{"ree": [{"foo": [1, 2, 3]}, "other"]}',
                    '{"ree": [{"foo": [1, 2, 3]}, "other"]}',
                ),
                (
                    "yaml",
                    "ree:\n- foo:\n  - 1\n  - 2\n  - 3\n- other",
                    "ree:\n- foo:\n  - 1\n  - 2\n  - 3\n- other",
                ),
                (
                    "toml",
                    'ree = [{foo = [1, 2, 3]}, "other"]',
                    'ree = [{foo = [1, 2, 3]}, "other"]',
                ),
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
        last_change_time = utils.file_mod_time(str(filepath))

        result = cli.run(["zetch", "put", str(filepath), *args, put_val])

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

        # If file shouldn't change, make sure the file wasn't touched at an OS level:
        if file_contents == file_contents_out_expected:
            utils.assert_file_not_modified(str(filepath), last_change_time)


@pytest.mark.parametrize(
    "args, expected_stdout",
    [
        # Json:
        (['{"foo": "bar"}', "foo", "baz", "--json"], '{\n  "foo": "baz"\n}'),
        # Yaml:
        (["foo: bar", "foo", "baz"], "foo: baz"),
        # Toml:
        (['foo = "bar"', "foo", "baz", "--toml"], 'foo = "baz"'),
    ],
)
def test_file_cmd_put_inline_content(args: "list[str]", expected_stdout: str):
    """Confirm works and outputs updated contents to stdout when inline content used."""
    result = cli.run(["zetch", "put", *args])
    assert result == expected_stdout
