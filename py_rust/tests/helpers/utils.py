import pathlib
import re
import typing as tp

from . import cli
from .tmp_file_manager import TmpFileManager

_LOCK_FILENAME = ".zetch.lock"
_MIDDLE_MATCHER = re.compile(r"\.zetch\.")
_END_MATCHER = re.compile(r"\.zetch$")


def get_out_path(path: pathlib.Path) -> tp.Optional[pathlib.Path]:
    middle_match = _MIDDLE_MATCHER.search(path.name)
    if middle_match is not None:
        return path.with_name(path.name.replace(middle_match.group(0), "."))

    end_match = _END_MATCHER.search(path.name)
    if end_match is not None:
        return path.with_name(path.name.replace(end_match.group(0), ""))

    return None


def get_lockfile_path(root: tp.Union[str, pathlib.Path]) -> pathlib.Path:
    return pathlib.Path(root).joinpath(f"./{_LOCK_FILENAME}")


def check_single(
    manager: TmpFileManager,
    config_file: tp.Union[str, pathlib.Path],
    contents: str,
    expected: tp.Union[str, tp.Callable[[str], bool]],
    file_type="txt",
    extra_args: tp.Optional[list[str]] = None,
    extra_templates_written: tp.Optional[list[pathlib.Path]] = None,
):
    template = manager.tmpfile(content=contents, suffix=".zetch.{}".format(file_type))

    rendered_info = cli.render(manager.root_dir, config_file, extra_args=extra_args)
    result = rendered_info["debug"]

    # Should return the correct compiled file:
    expected_written = [remove_template(template)] + (
        [remove_template(template) for template in extra_templates_written]
        if extra_templates_written
        else []
    )
    assert sorted(result["written"]) == sorted(expected_written), (
        sorted(result["written"]),
        sorted(expected_written),
    )
    assert result["identical"] == [], result["identical"]

    # Original shouldn't have changed:
    with open(template, "r") as file:
        assert contents == file.read()

    # Compiled should match expected:
    with open(result["written"][0], "r") as file:
        output = file.read()
        if isinstance(expected, str):
            assert output == expected, (output, expected)
        else:
            assert expected(output), (output, expected)


def remove_template(filepath: pathlib.Path) -> str:
    out_path = get_out_path(filepath)
    if out_path is None:
        raise ValueError(f"Could not find matcher in {filepath}")

    return str(out_path)
