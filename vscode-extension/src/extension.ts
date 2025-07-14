import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  Executable,
} from "vscode-languageclient/node";
import { EntitySearchViewProvider } from "./entitySearchView";

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let statusItem: vscode.LanguageStatusItem;
let restartInProgress = false;

let entitySearchProvider: EntitySearchViewProvider | undefined;

export async function activate(
  context: vscode.ExtensionContext
): Promise<void> {
  outputChannel = vscode.window.createOutputChannel(
    "Brandwatch Query Language Server"
  );
  context.subscriptions.push(outputChannel);

  statusItem = vscode.languages.createLanguageStatusItem("bwq-status", {
    language: "bwq",
    scheme: "*",
  });
  statusItem.name = "Brandwatch Query";
  statusItem.text = "Brandwatch Query";
  statusItem.command = {
    title: "Show Logs",
    command: "bwq.showLogs",
  };
  context.subscriptions.push(statusItem);

  entitySearchProvider = new EntitySearchViewProvider(context.extensionUri);
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      EntitySearchViewProvider.viewType,
      entitySearchProvider
    )
  );

  const runServer = async () => {
    if (restartInProgress) {
      return;
    }

    restartInProgress = true;
    updateStatus(
      "Starting...",
      vscode.LanguageStatusSeverity.Information,
      true
    );

    try {
      if (client) {
        await client.stop();
        client = undefined;
      }

      const config = vscode.workspace.getConfiguration("bwq");
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
        outputChannel: outputChannel,
        initializationOptions: {
          wikidataHoverEnabled: config.get<boolean>(
            "wikidata.enableHover",
            true
          ),
        },
      };

      client = new LanguageClient(
        "bwq",
        "Brandwatch Query Language Server",
        serverOptions,
        clientOptions
      );

      if (config.get<string>("trace.server") !== "off") {
        client.setTrace(config.get<string>("trace.server") as any);
      }

      client.onDidChangeState((event) => {
        if (event.newState === 3) {
          updateStatus(
            "Ready",
            vscode.LanguageStatusSeverity.Information,
            false
          );
        } else if (event.newState === 1) {
          updateStatus(
            "Starting...",
            vscode.LanguageStatusSeverity.Information,
            true
          );
        } else if (event.newState === 2) {
          updateStatus("Stopped", vscode.LanguageStatusSeverity.Warning, false);
        }
      });

      await client.start();

      // Update the view provider with the language client
      if (entitySearchProvider) {
        entitySearchProvider.setLanguageClient(client);
      }

      updateStatus("Ready", vscode.LanguageStatusSeverity.Information, false);
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      updateStatus(
        "Failed to start",
        vscode.LanguageStatusSeverity.Error,
        false,
        errorMessage
      );
      vscode.window.showErrorMessage(
        `Failed to start BWQ Language Server: ${errorMessage}`
      );
    } finally {
      restartInProgress = false;
    }
  };

  context.subscriptions.push(
    vscode.commands.registerCommand("bwq.restart", async () => {
      await runServer();
    }),
    vscode.commands.registerCommand("bwq.showLogs", () => {
      outputChannel.show();
    }),
    vscode.commands.registerCommand("bwq.searchEntities", () => {
      // Focus the entity search view if it's already open
      vscode.commands.executeCommand("bwq.entitySearch.focus");
    })
  );

  // Watch for configuration changes
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration("bwq.serverPath") ||
        event.affectsConfiguration("bwq.trace.server") ||
        event.affectsConfiguration("bwq.wikidata.enableHover")
      ) {
        vscode.window
          .showInformationMessage(
            "BWQ configuration changed. Restart the language server to apply changes.",
            "Restart"
          )
          .then((selection) => {
            if (selection === "Restart") {
              vscode.commands.executeCommand("bwq.restart");
            }
          });
      }
    })
  );

  await runServer();
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
  }
}

function updateStatus(
  status: string,
  severity: vscode.LanguageStatusSeverity,
  busy: boolean,
  detail?: string
): void {
  if (statusItem) {
    statusItem.text = `BWQ: ${status}`;
    statusItem.severity = severity;
    statusItem.busy = busy;
    statusItem.detail = detail;
  }
}
