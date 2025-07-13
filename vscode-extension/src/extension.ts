import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  Executable,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let statusItem: vscode.LanguageStatusItem;
let restartInProgress = false;

export async function activate(
  context: vscode.ExtensionContext
): Promise<void> {
  // Create output channel for server logs
  outputChannel = vscode.window.createOutputChannel("BWQ Language Server");
  context.subscriptions.push(outputChannel);

  // Create language status item
  statusItem = vscode.languages.createLanguageStatusItem("bwq-status", {
    language: "bwq",
    scheme: "*",
  });
  statusItem.name = "BWQ";
  statusItem.text = "BWQ";
  statusItem.command = {
    title: "Show Logs",
    command: "bwq.showLogs",
  };
  context.subscriptions.push(statusItem);

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

      client.onDidChangeState((event) => {
        if (event.newState === 3) {
          // Running
          updateStatus(
            "Ready",
            vscode.LanguageStatusSeverity.Information,
            false
          );
        } else if (event.newState === 1) {
          // Starting
          updateStatus(
            "Starting...",
            vscode.LanguageStatusSeverity.Information,
            true
          );
        } else if (event.newState === 2) {
          // Stopped
          updateStatus("Stopped", vscode.LanguageStatusSeverity.Warning, false);
        }
      });

      await client.start();
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

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand("bwq.restart", async () => {
      await runServer();
    }),
    vscode.commands.registerCommand("bwq.showLogs", () => {
      outputChannel.show();
    })
  );

  // Watch for configuration changes
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration("bwq.serverPath") ||
        event.affectsConfiguration("bwq.trace.server")
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
