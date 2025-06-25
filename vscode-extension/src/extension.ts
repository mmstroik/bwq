import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import {
	LanguageClient,
	LanguageClientOptions,
	Executable,
	TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
	const config = workspace.getConfiguration('bwqLint');
	const serverPath = config.get<string>('serverPath', 'bwq-lint');

	const serverOptions: Executable = {
		command: serverPath,
		args: ['lsp'],
	};

	const clientOptions: LanguageClientOptions = {
		documentSelector: [
			{ scheme: 'file', language: 'bwq' },
			{ scheme: 'file', pattern: '**/*.bwq' }
		],
		synchronize: {
			configurationSection: 'bwqLint',
			fileEvents: workspace.createFileSystemWatcher('**/*.bwq')
		}
	};

	client = new LanguageClient(
		'bwqLint',
		'BWQ Lint Language Server',
		serverOptions,
		clientOptions
	);

	if (config.get<string>('trace.server') !== 'off') {
		client.setTrace(config.get<string>('trace.server') as any);
	}

	client.start();
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}