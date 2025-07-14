import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";

export class EntitySearchViewProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = "bwq.entitySearch";
  private _view?: vscode.WebviewView;
  private readonly _extensionUri: vscode.Uri;
  private _languageClient: LanguageClient | undefined;

  constructor(extensionUri: vscode.Uri) {
    this._extensionUri = extensionUri;
    this._languageClient = undefined;
  }

  public setLanguageClient(client: LanguageClient) {
    this._languageClient = client;
  }

  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    _context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken
  ) {
    this._view = webviewView;

    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [vscode.Uri.joinPath(this._extensionUri, "webview")],
    };

    webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);

    // Handle messages from the webview
    webviewView.webview.onDidReceiveMessage(async (message) => {
      switch (message.command) {
        case "search":
          await this._handleSearch(message.query);
          break;
        case "copy":
          await this._handleCopy(message.entityId);
          break;
        case "openWikiData":
          await this._handleOpenWikiData(message.url);
          break;
      }
    });
  }

  private async _handleSearch(query: string) {
    if (!this._view) {
      return;
    }

    try {
      if (!this._languageClient) {
        throw new Error(
          "Language server not ready. Please wait for the server to start or try restarting the language server."
        );
      }

      // Send entity search request to language server
      const response = (await this._languageClient.sendRequest(
        "bwq/searchEntities",
        { query }
      )) as { results: any[] };

      this._view.webview.postMessage({
        command: "searchResults",
        results: response.results,
      });
    } catch (error) {
      this._view.webview.postMessage({
        command: "searchError",
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }

  private async _handleCopy(entityId: string) {
    const text = `entityId:${entityId}`;
    await vscode.env.clipboard.writeText(text);
    vscode.window.showInformationMessage(`Copied: ${text}`);
  }

  private async _handleOpenWikiData(url: string) {
    await vscode.env.openExternal(vscode.Uri.parse(url));
  }

  private _getHtmlForWebview(webview: vscode.Webview) {
    // Get the local path to main script run in the webview, then convert it to a uri we can use in the webview
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, "webview", "entitySearch.js")
    );

    // Do the same for the stylesheet
    const styleResetUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, "webview", "reset.css")
    );
    const styleVSCodeUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, "webview", "vscode.css")
    );
    const styleMainUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, "webview", "entitySearch.css")
    );

    // Use a nonce to only allow a specific script to be run
    const nonce = getNonce();

    return `<!DOCTYPE html>
      <html lang="en">
      <head>
        <meta charset="UTF-8">
        <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <link href="${styleResetUri}" rel="stylesheet">
        <link href="${styleVSCodeUri}" rel="stylesheet">
        <link href="${styleMainUri}" rel="stylesheet">
        <title>WikiData Entity Search</title>
      </head>
      <body>
        <div class="search-container">
          <div class="search-input-container">
            <input type="text" id="searchInput" placeholder="Search entities..." />
            <button id="searchButton">Search</button>
          </div>
          <div id="loading" class="loading hidden">Searching...</div>
          <div id="results" class="results"></div>
          <div id="error" class="error hidden"></div>
        </div>
        <script nonce="${nonce}" src="${scriptUri}"></script>
      </body>
      </html>`;
  }
}

function getNonce() {
  let text = "";
  const possible =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  for (let i = 0; i < 32; i++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}
