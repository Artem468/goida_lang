import * as fs from "fs";
import * as path from "path";
import { ExtensionContext, window, workspace } from "vscode";
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
    void client.start();
}
