import os
import typing as tp

import pytest
import zetch

from .helpers import cli
from .helpers.tmp_file_manager import TmpFileManager
from .helpers.types import InputConfig, Task


def check_file(filepath: str, content: str):
    with open(filepath, "r") as file:
        assert file.read().strip() == content.strip()


@pytest.mark.parametrize(
    "desc, typ, tasks, other_config, cb",
    [
        # Creating a random file should work in both pre and post: (also making sure it only runs once in the command itself)
        *[
            (
                f"basic_{typ}",
                typ,
                [{"commands": ["bash -c 'echo hello > file.txt'"]}],
                {},
                lambda man: lambda: check_file(os.path.join(man.root_dir, "file.txt"), "hello"),
            )
            for typ in ["pre", "post"]
        ],
        # Zetch read, put, del commands should work in both:
        *[
            (
                f"read_put_del_cmd_{typ}",
                typ,
                [
                    {
                        "commands": [
                            'bash -c \'echo {\\"ree\\": \\"bar\\"} > file.json\'',
                            "bash -c 'zetch put file.json value $(zetch read file.json ree)'",
                            "zetch del file.json ree",
                        ]
                    }
                ],
                {},
                lambda man: lambda: check_file(
                    os.path.join(man.root_dir, "file.json"), '{\n  "value": "bar"\n}'
                ),
            )
            for typ in ["pre", "post"]
        ],
        # Zetch read var should work in post: (but not pre which is checked in invalid test)
        (
            "read_var_post",
            "post",
            [
                {
                    "commands": [
                        "bash -c 'echo \"{}\" > file.json'",
                        "bash -c 'zetch put file.json value $(zetch var FOO)'",
                    ]
                }
            ],
            {"context": {"static": {"FOO": {"value": "bar"}}}},
            lambda man: lambda: check_file(
                os.path.join(man.root_dir, "file.json"), '{\n  "value": "bar"\n}'
            ),
        ),
    ],
)
def test_tasks_valid(
    desc: str,
    typ: tp.Literal["pre", "post"],
    tasks: "list[Task]",
    other_config: InputConfig,
    # Setup callable returns either None or a callable that will be called after the test to run any other checks.
    cb: "tp.Optional[tp.Callable[[TmpFileManager], tp.Optional[tp.Callable[[], tp.Any]]]]",
):
    with TmpFileManager() as manager:
        config = other_config
        config["tasks"] = {  # type: ignore
            typ: tasks,
        }
        conf_file = manager.tmpfile(
            zetch._toml_create(config),
            full_name="zetch.config.toml",
        )

        post_check = cb(manager) if cb else None

        cli.render(manager.root_dir, conf_file)

        if post_check:
            post_check()


@pytest.mark.parametrize(
    "desc, config, err_expected",
    [
        # Calling e.g. render should fail in both pre and post as would cause infinite task recursion:
        *[
            (
                "render_in_" + task,
                {"tasks": {task: [{"commands": ["zetch render"]}]}},
                "TaskRecursionError",
            )
            for task in ["pre", "post"]
        ],
        # Calling var subcommand in pre should fail as context is obviously only cached in post:
        (
            "var_in_pre",
            {
                "context": {"static": {"FOO": {"value": "bar"}}},
                "tasks": {"pre": [{"commands": ["zetch var FOO"]}]},
            },
            "TaskRecursionError",
        ),
        # Error response from middle command, end command, pre & post should cause task to fail:
        *[
            (
                "error_in_{}_{}_cmd".format(task, "middle" if in_middle else "end"),
                {
                    "tasks": {
                        "pre": [
                            {
                                "commands": ["bash -c 'echo foo && false'", "echo bar"]
                                if in_middle
                                else ["echo foo", "bash -c 'echo bar && false'"],
                            }
                        ]
                    }
                },
                "UserCommandError",
            )
            for task in ["pre", "post"]
            for in_middle in [True, False]
        ],
    ],
)
def test_tasks_invalid(
    desc: str,
    config: InputConfig,
    err_expected: str,
):
    with TmpFileManager() as manager:
        conf_file = manager.tmpfile(
            zetch._toml_create(config),
            full_name="zetch.config.toml",
        )
        with pytest.raises(ValueError, match=err_expected):
            cli.render(manager.root_dir, conf_file)


@pytest.mark.parametrize(
    "desc, config, cb",
    [
        # NOTE: written to catch bug:
        # Make sure var works in post, and still everything works when pre exists too. (pre should work with read/put/del but not var):
        (
            "var_in_post_when_pre_exist",
            {
                "tasks": {
                    "pre": [
                        {
                            "commands": [
                                "echo foo",
                            ]
                        }
                    ],
                    "post": [
                        {
                            "commands": [
                                "zetch var FOO",
                                'bash -c \'echo {\\"ree\\": \\"randomo\\"} > file.json\'',
                                "zetch read file.json ree",
                                "zetch put file.json value bar",
                                "zetch del file.json ree",
                                "zetch var FOO",
                            ]
                        }
                    ],
                },
                "context": {"static": {"FOO": {"value": "bar"}}},
            },
            lambda man: lambda: check_file(
                os.path.join(man.root_dir, "file.json"), '{\n  "value": "bar"\n}'
            ),
        ),
        # NOTE: written to catch: parent state decoding bug when env vars used in var subcommands in tasks, so check doesn't error anymore
        # (and doing the same for cli just in case)
        (
            "var_in_post_when_env_ctx_used",
            {
                "tasks": {"post": [{"commands": ["zetch var BAR", "zetch var BAZ"]}]},
                "context": {
                    "env": {"BAR": {"default": {"value": "bar"}}},
                    "cli": {"BAZ": {"commands": ["echo baz"]}},
                },
            },
            None,
        ),
    ],
)
def test_tasks_various(
    desc: str,
    config: InputConfig,
    # Setup callable returns either None or a callable that will be called after the test to run any other checks.
    cb: "tp.Optional[tp.Callable[[TmpFileManager], tp.Optional[tp.Callable[[], tp.Any]]]]",
):
    with TmpFileManager() as manager:
        conf_file = manager.tmpfile(
            zetch._toml_create(config),
            full_name="zetch.config.toml",
        )

        post_check = cb(manager) if cb else None

        cli.render(manager.root_dir, conf_file)

        if post_check:
            post_check()
