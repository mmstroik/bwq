import { workspace, ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  Executable,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  const config = workspace.getConfiguration("bwq");
  const serverPath = config.get<string>("serverPath", "bwq");

  const serverOptions: Executable = {
    command: serverPath,
    args: ["server"],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "bwq" },
      { scheme: "untitled", language: "bwq" },
    ],
  };

  client = new LanguageClient(
    "bwq",
    "BWQ Language Server",
    serverOptions,
    clientOptions
  );

  if (config.get<string>("trace.server") !== "off") {
    client.setTrace(config.get<string>("trace.server") as any);
  }

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
