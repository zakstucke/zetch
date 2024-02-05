import typing as tp

def register_function(func: tp.Callable) -> None:  # type: ignore
    """Register a custom function to be available in the template context.

    Example:
        ```python
        @zetch.register_function
        def adder(a: int, b: int = 3) -> int:
            return a + b

        ```
        `{{ foo(3, b=5) }}` -> `8`

    Args:
        func (tp.Callable): The function to register.
    """
    ...

def context() -> dict[str, tp.Any]:
    """Return the configured context globals for this run of zetch, can be run during custom extensions.

    Example:
        ```python
        @zetch.register_function
        def foo() -> str:
            return zetch.context()["foo"]
        ```
        `{{ foo() }}` -> `"bar"` (assuming the config has `foo = {"value": "bar"}`)

    Returns:
        dict[str, tp.Any]: The configured context globals.
    """
    ...

def _toml_create(data: tp.Any) -> str: ...
def _hash_contents(contents: str) -> str: ...

__version__: str

__all__ = ["__version__", "_hash_contents", "_toml_create"]
