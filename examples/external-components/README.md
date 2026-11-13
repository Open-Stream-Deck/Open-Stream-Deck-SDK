# External Components Example

This example installs two text files, without running native code. Use CLI 1.3.0 or newer and Open Stream Deck 1.1.0 or newer on Windows, Linux or macOS (x86_64 or aarch64). The example includes all six platform variants using the same two text files.

1. Create an empty directory outside the SDK, for example `F:/dev/OSD Component Test`.
2. Create a plain file named `example-app.txt` inside it. Its contents do not matter.
3. Run `opensd pack` in this example directory using the updated CLI.
4. Install `dist/dev.opensd.components-1.0.0.opensd` in Open Stream Deck.
5. Open the plugin's External components screen, select Example Extension and Example App.
6. Choose the test directory and review changes. Both destinations should be marked Create.
7. Confirm installation. Check `extensions/opensd/extension.txt` and `settings.txt`.
8. Review the same installation again. Both files should be Unchanged.
9. Remove the component and confirm the plan. Its two files and empty directories created by the installer should disappear; `example-app.txt` should remain.

To test updates, install version 1, change the component version and a payload file, repack, reinstall the plugin, then choose the same target directory. Updating the plugin alone must not change the external files.

To test conflicts, change an installed file directly. Review an update or removal. It must show a conflict and require explicit confirmation. To preserve the edited file, cancel. Confirmed conflict replacements are backed up in the app's component state directory.

To test stale plans, prepare a plan and edit a destination before confirming. Confirmation must fail without overwriting that edit. Review a fresh plan to continue.

To test original restoration, create `extensions/opensd/extension.txt` with your own text before installation, confirm its replacement, update the component, then remove it. Your initial text must be restored.

Do not use an actual application directory for these experiments.
