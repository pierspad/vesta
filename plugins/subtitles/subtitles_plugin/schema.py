# SPDX-License-Identifier: GPL-3.0-or-later
"""Declarative Widget UI Schema and Dynamic Patching for Vesta Plugins.

Produces standard JSON widget trees
and single-object patch operations compliant with Vesta Plugin System Specification (v1).

Supported Widgets:
column, row, text, heading, textInput, textArea, passwordInput, numberInput,
pathInput, languageSelect, segmented, select, checkbox, button,
link / externalLink, list, table, image, progress, fileOutput,
divider, settings.

Dynamic UI Patching (PluginPatch):
{
    "patch": {
        "values": { ... },
        "setProps": { id: { ... } },
        "visibility": { id: bool },
        "insert": { "parentId": str, "index"?: int, "node": dict },
        "remove": { "id": str },
        "move": { "id": str, "newParentId": str, "newIndex"?: int }
    }
}
"""

from __future__ import annotations

import copy
from typing import Any


# ---------------------------------------------------------------------------
# Declarative Widgets
# ---------------------------------------------------------------------------


def column(
    children: list[dict[str, Any]],
    gap: int | str = 4,
    id: str | None = None,
    hidden: bool = False,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "column", "gap": gap, "children": children}
    if id:
        node["id"] = id
    if hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def row(
    children: list[dict[str, Any]],
    gap: int | str = 3,
    align: str = "start",
    id: str | None = None,
    hidden: bool = False,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "row", "gap": gap, "align": align, "children": children}
    if id:
        node["id"] = id
    if hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def heading(
    text: str,
    level: int = 1,
    id: str | None = None,
    hidden: bool = False,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "heading", "level": level, "text": text}
    if id:
        node["id"] = id
    if hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def text(
    text: str,
    variant: str = "normal",  # normal, muted, danger, warning, success
    size: str = "base",  # sm, base, lg
    id: str | None = None,
    hidden: bool = False,
    color: str | None = None,
    visible: bool | None = None,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    if color and variant == "normal":
        if color == "error":
            variant = "danger"
        elif color in ("warning", "success", "muted"):
            variant = color

    node: dict[str, Any] = {"type": "text", "text": text, "variant": variant, "size": size}
    if id:
        node["id"] = id
    if visible is not None:
        node["hidden"] = not visible
    elif hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def text_input(
    id: str,
    label: str | None = None,
    placeholder: str = "",
    defaultValue: str = "",
    value: str | None = None,
    disabled: bool = False,
    hint: str | dict[str, Any] | None = None,
    link: dict[str, str] | None = None,
    hidden: bool = False,
    icon: str | None = None,
    flex: int | None = None,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    initial_val = value if value is not None else defaultValue
    node: dict[str, Any] = {
        "type": "textInput",
        "id": id,
        "defaultValue": initial_val,
        "placeholder": placeholder,
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if hint is not None:
        node["hint"] = hint
    if link is not None:
        node["link"] = link
    if hidden:
        node["hidden"] = True
    if icon is not None:
        node["icon"] = icon
    if flex is not None:
        node["flex"] = flex
    if expertOnly:
        node["expertOnly"] = True
    node["value"] = initial_val
    node.update(kwargs)
    return node


def text_area(
    id: str,
    label: str,
    placeholder: str = "",
    defaultValue: str = "",
    rows: int = 4,
    disabled: bool = False,
    hidden: bool = False,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {
        "type": "textArea",
        "id": id,
        "label": label,
        "placeholder": placeholder,
        "defaultValue": defaultValue,
        "rows": rows,
        "disabled": disabled,
    }
    if hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def password_input(
    id: str,
    label: str | None = None,
    placeholder: str = "",
    defaultValue: str = "",
    value: str | None = None,
    disabled: bool = False,
    hint: str | dict[str, Any] | None = None,
    link: dict[str, str] | None = None,
    hidden: bool = False,
    icon: str | None = None,
    flex: int | None = None,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    initial_val = value if value is not None else defaultValue
    node: dict[str, Any] = {
        "type": "passwordInput",
        "id": id,
        "defaultValue": initial_val,
        "placeholder": placeholder,
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if hint is not None:
        node["hint"] = hint
    if link is not None:
        node["link"] = link
    if hidden:
        node["hidden"] = True
    if icon is not None:
        node["icon"] = icon
    if flex is not None:
        node["flex"] = flex
    if expertOnly:
        node["expertOnly"] = True
    node["value"] = initial_val
    node.update(kwargs)
    return node


def path_input(
    id: str,
    label: str | None = None,
    placeholder: str = "",
    defaultValue: str = "",
    value: str | None = None,
    picker: str = "directory",
    mode: str = "directory",
    disabled: bool = False,
    hint: str | dict[str, Any] | None = None,
    link: dict[str, str] | None = None,
    hidden: bool = False,
    icon: str | None = None,
    flex: int | None = None,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    initial_val = value if value is not None else defaultValue
    node: dict[str, Any] = {
        "type": "pathInput",
        "id": id,
        "defaultValue": initial_val,
        "value": initial_val,
        "placeholder": placeholder,
        "pickerType": "file" if "file" in (picker, mode) else "folder",
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if hint is not None:
        node["hint"] = hint
    if link is not None:
        node["link"] = link
    if hidden:
        node["hidden"] = True
    if icon is not None:
        node["icon"] = icon
    if flex is not None:
        node["flex"] = flex
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def language_select(
    id: str,
    label: str | None = None,
    placeholder: str = "",
    defaultValue: str = "",
    value: str | None = None,
    disabled: bool = False,
    hint: str | dict[str, Any] | None = None,
    hidden: bool = False,
    icon: str | None = None,
    flex: int | None = None,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    initial_val = value if value is not None else defaultValue
    node: dict[str, Any] = {
        "type": "languageSelect",
        "id": id,
        "defaultValue": initial_val,
        "value": initial_val,
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if placeholder:
        node["placeholder"] = placeholder
    if hint is not None:
        node["hint"] = hint
    if hidden:
        node["hidden"] = True
    if icon is not None:
        node["icon"] = icon
    if flex is not None:
        node["flex"] = flex
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def segmented(
    id: str,
    options: list[dict[str, Any]],
    defaultValue: str = "",
    value: str | None = None,
    disabled: bool = False,
    hidden: bool = False,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    initial_val = value if value is not None else defaultValue
    node: dict[str, Any] = {
        "type": "segmented",
        "id": id,
        "options": options,
        "defaultValue": initial_val,
        "value": initial_val,
        "disabled": disabled,
    }
    if hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def number_input(
    id: str,
    label: str | None = None,
    defaultValue: int | float = 0,
    value: int | float | None = None,
    min_val: int | float | None = None,
    max_val: int | float | None = None,
    step: int | float = 1,
    disabled: bool = False,
    hint: str | dict[str, Any] | None = None,
    hidden: bool = False,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    initial_val = value if value is not None else defaultValue
    node: dict[str, Any] = {
        "type": "numberInput",
        "id": id,
        "defaultValue": initial_val,
        "value": initial_val,
        "step": step,
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if min_val is not None:
        node["min"] = min_val
    if max_val is not None:
        node["max"] = max_val
    if hint is not None:
        node["hint"] = hint
    if hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def checkbox(
    id: str,
    label: str,
    defaultValue: bool = False,
    checked: bool | None = None,
    disabled: bool = False,
    hint: str | dict[str, Any] | None = None,
    hidden: bool = False,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    initial_val = checked if checked is not None else defaultValue
    node: dict[str, Any] = {
        "type": "checkbox",
        "id": id,
        "label": label,
        "defaultValue": initial_val,
        "checked": initial_val,
        "disabled": disabled,
    }
    if hint is not None:
        node["hint"] = hint
    if hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def select(
    id: str,
    options: list[dict[str, str]],
    label: str | None = None,
    defaultValue: str = "",
    value: str | None = None,
    disabled: bool = False,
    hint: str | dict[str, Any] | None = None,
    hidden: bool = False,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    initial_val = value if value is not None else defaultValue
    node: dict[str, Any] = {
        "type": "select",
        "id": id,
        "options": options,
        "defaultValue": initial_val,
        "value": initial_val,
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if hint is not None:
        node["hint"] = hint
    if hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def button(
    id: str,
    label: str,
    variant: str = "primary",  # primary, secondary, danger
    disabled: bool = False,
    icon: str | None = None,
    hidden: bool = False,
    expertOnly: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {
        "type": "button",
        "id": id,
        "label": label,
        "variant": variant,
        "disabled": disabled,
    }
    if icon:
        node["icon"] = icon
    if hidden:
        node["hidden"] = True
    if expertOnly:
        node["expertOnly"] = True
    node.update(kwargs)
    return node


def link(*args: Any, **kwargs: Any) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "link"}
    if len(args) == 1:
        if "label" in kwargs:
            node["id"] = args[0]
        else:
            node["label"] = args[0]
    elif len(args) == 2:
        node["label"] = args[0]
        node["url"] = args[1]
    elif len(args) >= 3:
        node["id"] = args[0]
        node["label"] = args[1]
        node["url"] = args[2]

    for k, v in kwargs.items():
        node[k] = v
    return node


def external_link(*args: Any, **kwargs: Any) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "externalLink"}
    if len(args) == 1:
        if "label" in kwargs:
            node["id"] = args[0]
        else:
            node["label"] = args[0]
    elif len(args) == 2:
        node["label"] = args[0]
        node["url"] = args[1]
    elif len(args) >= 3:
        node["id"] = args[0]
        node["label"] = args[1]
        node["url"] = args[2]

    for k, v in kwargs.items():
        node[k] = v
    return node


def list_item(
    id: str,
    title: str,
    subtitle: str | None = None,
    badge: str | None = None,
    image: str | None = None,
    actions: list[dict[str, Any]] | None = None,
    **kwargs: Any,
) -> dict[str, Any]:
    item: dict[str, Any] = {"id": id, "title": title}
    if subtitle is not None:
        item["subtitle"] = subtitle
    if badge is not None:
        item["badge"] = badge
    if image is not None:
        item["image"] = image
    if actions is not None:
        item["actions"] = actions
    item.update(kwargs)
    return item


def list_widget(
    id: str | None = None,
    items: list[dict[str, Any]] | None = None,
    empty_text: str = "No items to display",
    hidden: bool = False,
    visible: bool | None = None,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {
        "type": "list",
        "items": items or [],
        "emptyText": empty_text,
    }
    if id:
        node["id"] = id
    if visible is not None:
        node["hidden"] = not visible
    elif hidden:
        node["hidden"] = True
    node.update(kwargs)
    return node


def table(
    headers: list[str],
    rows: list[list[Any]],
    id: str | None = None,
    hidden: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "table", "headers": headers, "rows": rows}
    if id:
        node["id"] = id
    if hidden:
        node["hidden"] = True
    node.update(kwargs)
    return node


def image(
    src: str,
    alt: str = "",
    width: int | None = None,
    height: int | None = None,
    id: str | None = None,
    hidden: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "image", "src": src, "alt": alt}
    if width is not None:
        node["width"] = width
    if height is not None:
        node["height"] = height
    if id:
        node["id"] = id
    if hidden:
        node["hidden"] = True
    node.update(kwargs)
    return node


def progress(
    value: float,
    max_val: float = 100.0,
    label: str = "",
    id: str | None = None,
    hidden: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "progress", "value": value, "max": max_val}
    if label:
        node["label"] = label
    if id:
        node["id"] = id
    if hidden:
        node["hidden"] = True
    node.update(kwargs)
    return node


def file_output(
    path: str = "",
    label: str | None = None,
    content: str | None = None,
    id: str | None = None,
    hidden: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    resolved_path = path or (content or "")
    node: dict[str, Any] = {"type": "fileOutput", "path": resolved_path, "content": resolved_path}
    if label:
        node["label"] = label
    if id:
        node["id"] = id
    if hidden:
        node["hidden"] = True
    node.update(kwargs)
    return node


def divider(**kwargs: Any) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "divider"}
    node.update(kwargs)
    return node


def settings_widget(
    fields: list[dict[str, Any]],
    id: str | None = None,
    hidden: bool = False,
    **kwargs: Any,
) -> dict[str, Any]:
    node: dict[str, Any] = {"type": "settings", "fields": fields}
    if id:
        node["id"] = id
    if hidden:
        node["hidden"] = True
    node.update(kwargs)
    return node


# ---------------------------------------------------------------------------
# Dynamic UI Operations (Single-Object PluginPatch)
# ---------------------------------------------------------------------------


def make_patch(
    values: dict[str, Any] | None = None,
    set_props: dict[str, dict[str, Any]] | None = None,
    visibility: dict[str, bool] | None = None,
    insert: dict[str, Any] | None = None,
    remove: dict[str, Any] | None = None,
    move: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Constructs a single-object PluginPatch dictionary per Vesta specification.

    Structure:
    {
        "values": { ... },
        "setProps": { id: { prop: val } },
        "visibility": { id: bool },
        "insert": { "parentId": str, "index"?: int, "node": dict },
        "remove": { "id": str },
        "move": { "id": str, "newParentId": str, "newIndex"?: int }
    }
    """
    patch: dict[str, Any] = {}
    if values is not None:
        patch["values"] = values
    if set_props is not None:
        patch["setProps"] = set_props
    if visibility is not None:
        patch["visibility"] = visibility
    if insert is not None:
        patch["insert"] = insert
    if remove is not None:
        patch["remove"] = remove
    if move is not None:
        patch["move"] = move
    return patch


def find_node(
    root: dict[str, Any],
    node_id: str,
    parent: dict[str, Any] | None = None,
    index: int | None = None,
) -> tuple[dict[str, Any] | None, dict[str, Any] | None, int | None]:
    """Recursively search for a node by id in a widget tree.

    Returns (node, parent_node, index_in_parent).
    """
    if root.get("id") == node_id:
        return root, parent, index

    children = root.get("children")
    if isinstance(children, list):
        for idx, child in enumerate(children):
            if isinstance(child, dict):
                found, p, i = find_node(child, node_id, root, idx)
                if found is not None:
                    return found, p, i

    items = root.get("items")
    if isinstance(items, list):
        for idx, item in enumerate(items):
            if isinstance(item, dict):
                found, p, i = find_node(item, node_id, root, idx)
                if found is not None:
                    return found, p, i

    return None, None, None


def apply_patch_to_tree(root: dict[str, Any], patch: dict[str, Any]) -> dict[str, Any]:
    """Applies a single-object PluginPatch to a widget tree in-memory, matching
    the exact behavior of Vesta's Svelte PluginRenderer.
    """
    p = patch.get("patch", patch) if isinstance(patch, dict) else {}
    current = copy.deepcopy(root)

    # 1. Handle move: extract then re-insert
    move_cfg = p.get("move")
    if move_cfg and isinstance(move_cfg, dict) and "id" in move_cfg:
        move_id = move_cfg["id"]
        new_parent_id = move_cfg.get("newParentId")
        new_index = move_cfg.get("newIndex")
        moved_node: dict[str, Any] | None = None

        def extract(node: dict[str, Any]) -> None:
            nonlocal moved_node
            children = node.get("children")
            if isinstance(children, list):
                for idx, c in enumerate(children):
                    if isinstance(c, dict) and c.get("id") == move_id:
                        moved_node = children.pop(idx)
                        return
                for c in children:
                    if isinstance(c, dict):
                        extract(c)

        extract(current)

        if moved_node and new_parent_id:
            def insert_into(node: dict[str, Any]) -> None:
                if node.get("id") == new_parent_id and isinstance(node.get("children"), list):
                    idx = new_index if new_index is not None else len(node["children"])
                    node["children"].insert(idx, moved_node)
                    return
                children = node.get("children")
                if isinstance(children, list):
                    for c in children:
                        if isinstance(c, dict):
                            insert_into(c)

            insert_into(current)

    # 2. Visit nodes for visibility, setProps, insert, remove
    visibility_map = p.get("visibility") or {}
    set_props_map = p.get("setProps") or {}
    insert_cfg = p.get("insert")
    remove_cfg = p.get("remove")

    def visit(node: dict[str, Any]) -> None:
        node_id = node.get("id")
        if node_id:
            if node_id in visibility_map:
                node["hidden"] = not bool(visibility_map[node_id])
                node["visible"] = bool(visibility_map[node_id])
            if node_id in set_props_map and isinstance(set_props_map[node_id], dict):
                node.update(set_props_map[node_id])
            if insert_cfg and isinstance(insert_cfg, dict) and insert_cfg.get("parentId") == node_id:
                if "children" in node and isinstance(node["children"], list):
                    idx = insert_cfg.get("index")
                    insert_node = copy.deepcopy(insert_cfg.get("node", {}))
                    if idx is not None and 0 <= idx <= len(node["children"]):
                        node["children"].insert(idx, insert_node)
                    else:
                        node["children"].append(insert_node)

        children = node.get("children")
        if isinstance(children, list):
            if remove_cfg and isinstance(remove_cfg, dict) and "id" in remove_cfg:
                rem_id = remove_cfg["id"]
                node["children"] = [
                    c for c in children if not (isinstance(c, dict) and c.get("id") == rem_id)
                ]
            for c in node["children"]:
                if isinstance(c, dict):
                    visit(c)

    visit(current)
    return current


def apply_patches(
    tree: dict[str, Any],
    patch: dict[str, Any] | list[dict[str, Any]],
) -> dict[str, Any]:
    """Applies a patch (or list of patches) and returns the updated tree."""
    if isinstance(patch, list):
        res = tree
        for item in patch:
            res = apply_patch_to_tree(res, item)
        return res
    return apply_patch_to_tree(tree, patch)
