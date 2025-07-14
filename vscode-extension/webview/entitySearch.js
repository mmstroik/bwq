// Entity Search webview JavaScript
(function () {
  const vscode = acquireVsCodeApi();

  // DOM elements
  const searchInput = document.getElementById("searchInput");
  const searchButton = document.getElementById("searchButton");
  const loadingDiv = document.getElementById("loading");
  const resultsDiv = document.getElementById("results");
  const errorDiv = document.getElementById("error");

  searchButton.addEventListener("click", performSearch);
  searchInput.addEventListener("keypress", (e) => {
    if (e.key === "Enter") {
      performSearch();
    }
  });

  window.addEventListener("message", (event) => {
    const message = event.data;

    switch (message.command) {
      case "searchResults":
        handleSearchResults(message.results);
        break;
      case "searchError":
        handleSearchError(message.error);
        break;
    }
  });

  function performSearch() {
    const query = searchInput.value.trim();

    if (!query) {
      showError("Please enter a search term");
      return;
    }

    showLoading();
    clearError();

    vscode.postMessage({
      command: "search",
      query: query,
    });
  }

  function handleSearchResults(results) {
    hideLoading();
    displayResults(results);
  }

  function handleSearchError(error) {
    hideLoading();
    showError(`Search failed: ${error}`);
  }

  function displayResults(results) {
    resultsDiv.innerHTML = "";

    if (results.length === 0) {
      resultsDiv.innerHTML = `
        <div class="no-results">
          No entities found for "${searchInput.value}". Try a different search term.
        </div>
      `;
      return;
    }

    results.forEach((result) => {
      const resultElement = createResultElement(result);
      resultsDiv.appendChild(resultElement);
    });
  }

  function createResultElement(result) {
    const div = document.createElement("div");
    div.className = "result-item";
    div.tabIndex = 0;

    div.innerHTML = `
      <div class="result-header">
        <h3 class="result-title">${escapeHtml(result.label)}</h3>
        <span class="result-id">${escapeHtml(result.id)}</span>
      </div>
      ${
        result.description
          ? `<p class="result-description">${escapeHtml(
              result.description
            )}</p>`
          : ""
      }
      <div class="result-actions">
        <button class="copy-button" data-entity-id="${escapeHtml(
          result.id
        )}">Copy entityId</button>
        <button class="open-button" data-url="${escapeHtml(
          result.concepturi
        )}">Open in WikiData</button>
      </div>
    `;

    // Add event listeners to buttons
    const copyButton = div.querySelector(".copy-button");
    const openButton = div.querySelector(".open-button");

    copyButton.addEventListener("click", (e) => {
      e.stopPropagation();
      const entityId = result.id.startsWith("Q")
        ? result.id.substring(1)
        : result.id;
      vscode.postMessage({
        command: "copy",
        entityId: entityId,
      });
    });

    openButton.addEventListener("click", (e) => {
      e.stopPropagation();
      vscode.postMessage({
        command: "openWikiData",
        url: result.concepturi,
      });
    });

    // Make the whole item clickable to copy
    div.addEventListener("click", () => {
      const entityId = result.id.startsWith("Q")
        ? result.id.substring(1)
        : result.id;
      vscode.postMessage({
        command: "copy",
        entityId: entityId,
      });
    });

    // Keyboard navigation
    div.addEventListener("keypress", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        const entityId = result.id.startsWith("Q")
          ? result.id.substring(1)
          : result.id;
        vscode.postMessage({
          command: "copy",
          entityId: entityId,
        });
      }
    });

    return div;
  }

  function showLoading() {
    loadingDiv.classList.remove("hidden");
    resultsDiv.innerHTML = "";
  }

  function hideLoading() {
    loadingDiv.classList.add("hidden");
  }

  function showError(message) {
    errorDiv.textContent = message;
    errorDiv.classList.remove("hidden");
  }

  function clearError() {
    errorDiv.classList.add("hidden");
    errorDiv.textContent = "";
  }

  function escapeHtml(text) {
    const map = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#039;",
    };
    return text.replace(/[&<>"']/g, (m) => map[m]);
  }

  // Focus the search input on load
  searchInput.focus();
})();
