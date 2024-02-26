import typing_extensions as tp

Coerce_T = tp.Literal["str", "int", "float", "bool", "json"]


class CliCtx(tp.TypedDict):
    commands: "list[str]"
    coerce: tp.NotRequired[Coerce_T]
    light: tp.NotRequired["StaticCtx"]


class EnvCtx(tp.TypedDict):
    env_name: tp.NotRequired[str]
    default: tp.NotRequired["StaticCtx"]
    coerce: tp.NotRequired[Coerce_T]


class StaticCtx(tp.TypedDict):
    value: tp.Any
    coerce: tp.NotRequired[Coerce_T]


class Engine(tp.TypedDict):
    variable_start: tp.NotRequired[str]
    variable_end: tp.NotRequired[str]
    block_start: tp.NotRequired[str]
    block_end: tp.NotRequired[str]
    comment_start: tp.NotRequired[str]
    comment_end: tp.NotRequired[str]
    custom_extensions: tp.NotRequired["list[str]"]


class InputContext(tp.TypedDict):
    static: tp.NotRequired["dict[str, StaticCtx]"]
    cli: tp.NotRequired["dict[str, CliCtx]"]
    env: tp.NotRequired["dict[str, EnvCtx]"]


class Task(tp.TypedDict):
    commands: "list[str]"


class Tasks(tp.TypedDict):
    pre: tp.NotRequired["list[Task]"]
    post: tp.NotRequired["list[Task]"]


class InputConfig(tp.TypedDict):
    ignore_files: tp.NotRequired["list[str]"]
    matchers: tp.NotRequired["list[str]"]
    exclude: tp.NotRequired["list[str]"]
    engine: tp.NotRequired[Engine]
    context: tp.NotRequired[InputContext]
    tasks: tp.NotRequired["Tasks"]


class OutputConfig(InputConfig):
    context: "dict[str, tp.Any]"


class DebugOutput(tp.TypedDict):
    config: OutputConfig
