import * as path from 'path';
import * as fs from 'fs';
import { ExtensionContext, workspace, window } from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

export function activate(context: ExtensionContext) {
    const binaryName = process.platform === 'win32' ? 'lsp.exe' : 'lsp';
    
    const serverPath = path.join(context.extensionPath, binaryName);
    
    window.showInformationMessage(`Goida LSP: путь = ${serverPath}`);
    
    if (!fs.existsSync(serverPath)) {
        window.showErrorMessage(`Goida LSP: файл не найден по пути ${serverPath}`);
        console.error(`[Goida] Файл не найден: ${serverPath}`);
        return;
    }
    
    const serverOptions: ServerOptions = {
        run: { command: serverPath },
        debug: { command: serverPath }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'goida' }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/*.goida')
        }
    };

    const client = new LanguageClient('goidaLsp', 'Goida Language Server', serverOptions, clientOptions);
    
    client.start().catch((err) => {
        window.showErrorMessage(`Goida LSP: ошибка - ${err.message}`);
        console.error(err);
    });
}