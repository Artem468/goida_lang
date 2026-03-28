import * as path from 'path';
import {ExtensionContext, workspace} from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

export function activate(context: ExtensionContext) {
    const serverPath = context.asAbsolutePath(path.join('..', '..', 'target', 'debug', 'lsp.exe'));

    const serverOptions: ServerOptions = {
        run: { command: serverPath },
        debug: { command: serverPath }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'goida' }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/.goida')
        }
    };

    const client = new LanguageClient('goidaLsp', 'Goida Language Server', serverOptions, clientOptions);
    client.start().then(() => {
        client.outputChannel.show();
    });
}
