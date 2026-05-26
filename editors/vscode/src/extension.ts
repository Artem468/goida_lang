import * as fs from "fs";
import * as path from "path";
import {
    commands,
    ExtensionContext,
    ViewColumn,
    window,
    workspace,
} from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";

function findServerBinary(context: ExtensionContext): string | undefined {
    const binaryName = process.platform === "win32" ? "lsp.exe" : "lsp";
    const configuredPath = workspace.getConfiguration("goida").get<string>("languageServer.path");

    if (configuredPath && fs.existsSync(configuredPath)) {
        return configuredPath;
    }

    const workspaceFolder = workspace.workspaceFolders?.[0]?.uri.fsPath;
    const workspaceCandidates = workspaceFolder
        ? [
              path.join(workspaceFolder, "target", "debug", binaryName),
              path.join(workspaceFolder, "target", "release", binaryName),
              path.join(workspaceFolder, "editors", "vscode", binaryName),
          ]
        : [];
    const extensionBundled = path.join(context.extensionPath, binaryName);

    return [...workspaceCandidates, extensionBundled].find((candidate) => fs.existsSync(candidate));
}

export function activate(context: ExtensionContext) {
    const serverPath = findServerBinary(context);
    if (!serverPath) {
        void window.showErrorMessage(
            "Goida LSP binary not found. Set goida.languageServer.path in settings.",
        );
        return;
    }

    const serverOptions: ServerOptions = {
        run: { command: serverPath },
        debug: { command: serverPath },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "goida" }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher("**/*.goida"),
        },
    };

    const client = new LanguageClient(
        "goidaLsp",
        "Goida Language Server",
        serverOptions,
        clientOptions,
    );

    context.subscriptions.push(client);
    const clientReady = client.start();

    context.subscriptions.push(
        commands.registerCommand("goida.showMacroExpansion", async () => {
            const editor = window.activeTextEditor;
            if (!editor || editor.document.languageId !== "goida") {
                await window.showInformationMessage("Open a Goida file to preview macro expansion.");
                return;
            }

            await clientReady;
            const result = await client.sendRequest<string | null>("workspace/executeCommand", {
                command: "goida.expandMacros",
                arguments: [editor.document.uri.toString()],
            });

            if (result === null) {
                await window.showWarningMessage("Goida macro expansion preview returned no result.");
                return;
            }

            if (result.startsWith("GOIDA_MACRO_PREVIEW_ERROR\n")) {
                await window.showErrorMessage(
                    result.replace("GOIDA_MACRO_PREVIEW_ERROR\n", ""),
                );
                return;
            }

            const document = await workspace.openTextDocument({
                content: result,
                language: "plaintext",
            });
            await window.showTextDocument(document, {
                viewColumn: ViewColumn.Beside,
                preview: true,
            });
            await commands
                .executeCommand("workbench.action.files.setActiveEditorReadonlyInSession")
                .then(undefined, () => undefined);
        }),
    );
    void clientReady;
}
