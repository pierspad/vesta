# Hello Counter Example Plugin

This directory contains a minimal, working reference implementation of an external Vesta plugin written in Python using the bundled `vesta_plugin.py` SDK helper.

## Files

- `vesta-plugin.json`: Plugin manifest declaring ID, version, entry command, permissions (`network`, `vesta:cli`), tab title, and configuration settings.
- `vesta_plugin.py`: Lightweight zero-dependency Python SDK helper managing JSON-RPC 2.0 communication over stdio (including `plugin.run_cli(command, args, timeout_secs)` for invoking native `vesta-cli` subcommands via `vesta.cli`).
- `main.py`: Entry point implementing screen rendering, counter state, event dispatching, `vesta.cli` execution, and notification toasts.

## Testing Locally in Vesta

1. Start Vesta development server:
   ```bash
   cd apps/vesta-gui && npx tauri dev
   ```

2. Open the **Plugins** manager tab in the sidebar.

3. Switch the install source to **Local Folder** and select `docs/examples/plugin-hello` (or enter its path).

4. Click **Install Plugin**. You will see:
   - Name: "Hello Counter"
   - ID: `plugin-hello`
   - Version: `1.0.0`
   - Required Permissions: Network Access
   - Security Notice

5. Click **Confirm & Install**.

6. The plugin is installed into Vesta's plugin directory and **Hello Counter** appears under **Installed Plugins** in the **Plugins** view.

7. Click **Open** on the **Hello Counter** card (or click the card itself) to open the plugin inside the Plugins workspace.
