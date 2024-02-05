import typing_extensions as tp


class Update(tp.TypedDict):
    # The path in the yaml file to update.
    path: "list[tp.Union[str, int]]"
    # If key doesn't exist will be treated as delete.
    # The string will be a json str that needs to be decoded.
    put: "tp.NotRequired[str]"


def modify_yaml(src: str, updates: "list[Update]") -> memoryview:
    """Add and delete yaml whilst preserving comments.

    Multiple updates can be provided, which will be applied in order.
    Will error on missing intermediary paths etc, should all be handled by caller.
    Will also error on missing delete as caller shouldn't have asked!
    """
    import io
    import json

    from ruamel.yaml import YAML

    yaml = YAML(typ="rt", pure=True)
    code = yaml.load(src)

    for update in updates:
        # Get to the parent of the item to delete or put
        parent = code
        path = update["path"]
        for p in path[:-1]:
            parent = parent[p]

        if "put" in update:
            key = path[-1]
            value = json.loads(update["put"])
            # If working on array and index one out of range, push:
            if isinstance(key, int) and key == len(parent):
                parent.append(value)
            else:
                parent[key] = value
        else:
            # Delete
            del parent[path[-1]]

    buf = io.BytesIO()
    yaml.dump(code, buf)
    # Save a python string alloc and just return the memoryview to do on rust side:
    return buf.getbuffer()
