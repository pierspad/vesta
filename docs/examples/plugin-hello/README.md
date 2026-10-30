# Hello Counter Example Plugin

This directory contains a minimal, working reference implementation of an external Vesta plugin written in Python using the bundled `vesta_plugin.py` SDK helper.

## Files

- `vesta-plugin.json`: Plugin manifest declaring ID, version, entry command, permissions, tab title, and configuration settings.
- `vesta_plugin.py`: Lightweight zero-dependency Python SDK helper managing JSON-RPC 2.0 communication over stdio.
- `main.py`: Entry point implementing screen rendering, counter state, event dispatching, and notification toasts.

## Testing Locally in Vesta

1. Start Vesta development server:
   ```bash
   cd apps/srt-gui && npm run dev
   ```

2. Open the **Plugins** manager tab in the sidebar.

3. In the repository URL input field, enter the path to this folder:
   ```
   docs/examples/plugin-hello
   ```
   *(or an absolute path `/path/to/vesta/docs/examples/plugin-hello` / `~/...` / `file://...`)*

4. Click **Inspect Plugin**. You will see:
   - Name: "Hello Counter"
   - ID: `plugin-hello`
   - Version: `1.0.0`
   - Required Permissions: Network Access
   - Security Notice

5. Click **Confirm & Install**.

6. The plugin is installed into Vesta's plugin directory and a new tab titled **Hello Counter** appears in the sidebar.

7. Click the tab in the sidebar to interact with the plugin!
