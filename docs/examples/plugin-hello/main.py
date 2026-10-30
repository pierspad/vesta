#!/usr/bin/env python3
"""Minimal Example Plugin for Vesta (Hello Counter)

Demonstrates declarative widget rendering, user interaction events,
patch operations, and notifications over JSON-RPC 2.0 stdio.
"""

from vesta_plugin import (
    Button,
    Checkbox,
    Column,
    Divider,
    Heading,
    Link,
    NumberInput,
    Row,
    Text,
    TextInput,
    VestaPlugin,
)

plugin = VestaPlugin()
counter = 0

def build_screen(name: str = "World", step: int = 1, show_tip: bool = True):
    prefix = plugin.settings.get("welcomePrefix", "Welcome")
    children = [
        Heading(f"{prefix}, {name}!", level=1),
        Text("This tab is rendered entirely by an external child process via JSON-RPC 2.0.", variant="muted"),
        Divider(),
        Row([
            TextInput("user_name", label="Your Name", defaultValue=name, placeholder="Enter your name…"),
            NumberInput("step_val", label="Step Size", defaultValue=step, min=1, max=10),
        ], gap=3),
        Row([
            Button("btn_increment", label=f"Increment Counter (Current: {counter})", variant="primary"),
            Button("btn_reset", label="Reset to 0", variant="secondary"),
        ], gap=2),
        Checkbox("show_tip_box", label="Show Developer Tip", defaultValue=show_tip),
    ]

    if show_tip:
        children.append(
            Column([
                Text("Tip: Widgets are defined declaratively in Python and styled natively by Vesta.", variant="normal", size="sm"),
                Link("Read Plugin Specification", "https://github.com/pierspad/vesta/blob/main/docs/PLUGINS.md"),
            ], gap=1, id="tip_container")
        )

    return Column(children, gap=4)

@plugin.on_render("main")
def render_main(params):
    return build_screen()

@plugin.on_event("btn_increment")
def handle_increment(screen, widget_id, event_type, values):
    global counter
    name = values.get("user_name", "World") or "World"
    try:
        step = int(values.get("step_val", 1))
    except (ValueError, TypeError):
        step = 1

    show_tip = bool(values.get("show_tip_box", True))
    counter += step
    plugin.notify(f"Counter incremented by {step}! New count: {counter}", level="success")
    return build_screen(name=name, step=step, show_tip=show_tip)

@plugin.on_event("btn_reset")
def handle_reset(screen, widget_id, event_type, values):
    global counter
    counter = 0
    name = values.get("user_name", "World") or "World"
    show_tip = bool(values.get("show_tip_box", True))
    plugin.notify("Counter reset to 0.", level="info")
    return build_screen(name=name, step=1, show_tip=show_tip)

@plugin.on_event("show_tip_box")
def handle_toggle_tip(screen, widget_id, event_type, values):
    show_tip = bool(values.get("show_tip_box", False))
    return {
        "patch": {
            "visibility": {
                "tip_container": show_tip
            }
        }
    }

if __name__ == "__main__":
    plugin.run()
