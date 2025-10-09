# Querent SQL VS Code Extension

A VS Code extension that provides SQL language support and autocompletion through the Querent LSP server.

## Installation

1. Install dependencies:

   ```bash
   pnpm install
   ```

2. Build the extension:

   ```bash
   pnpm run compile
   ```

3. Run the extension in development mode:
   - Press `F5` in VS Code to open a new Extension Development Host window
   - Or run "Run Extension" from the Run and Debug panel

## Usage

1. Start the Querent LSP server:

   ```bash
   cd ../lsp
   cargo run
   ```

   The LSP server should be running on `ws://127.0.0.1:9001`

2. Open any `.sql` file in VS Code

3. Start typing SQL - completions will automatically appear as you type

4. Completions are triggered automatically when you type:
   - `.` (for table.column suggestions)
   - ` ` (space, for keyword suggestions)
   - `,` (comma, for continuing lists)

   Or manually trigger with `Ctrl+Space` / `Cmd+Space`
