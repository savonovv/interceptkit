import { sendCommand } from "./lib/runtime";
import type { RewriteRule, RuleDraft } from "./lib/types";

interface CapturedRequest {
  id: string;
  method: string;
  url: string;
  status?: number;
  requestBody?: string;
  responseBody?: string;
}

const requestsRoot = document.getElementById("requests") as HTMLDivElement;
const captureStatus = document.getElementById("captureStatus") as HTMLSpanElement;

const capturedRequests: CapturedRequest[] = [];

function render(): void {
  if (capturedRequests.length === 0) {
    requestsRoot.innerHTML = '<p>No requests captured yet.</p>';
    return;
  }

  requestsRoot.innerHTML = capturedRequests
    .map(
      (entry) => `<article class="request-row" data-id="${entry.id}">
        <div>
          <div><strong>${entry.method}</strong> ${entry.url}</div>
          <div class="meta">status=${entry.status ?? "unknown"}</div>
        </div>
        <button data-action="draft">Create Rule</button>
      </article>`
    )
    .join("");
}

function addCapturedRequest(entry: CapturedRequest): void {
  capturedRequests.unshift(entry);
  if (capturedRequests.length > 40) {
    capturedRequests.pop();
  }
  render();
}

function requestContent(request: chrome.devtools.network.Request): Promise<string> {
  return new Promise((resolve) => {
    request.getContent((content) => resolve(content ?? ""));
  });
}

chrome.devtools.network.onRequestFinished.addListener(async (request) => {
  const content = await requestContent(request);
  const postData = request.request.postData?.text;

  addCapturedRequest({
    id: `${request.request.method}-${request.request.url}-${request.startedDateTime}`,
    method: request.request.method,
    url: request.request.url,
    status: request.response.status,
    requestBody: postData,
    responseBody: content
  });

  captureStatus.textContent = `${capturedRequests.length} captured`;
});

requestsRoot.addEventListener("click", (event) => {
  const target = event.target as HTMLElement;
  const button = target.closest<HTMLButtonElement>("button[data-action='draft']");
  if (!button) {
    return;
  }

  const row = target.closest<HTMLElement>("[data-id]");
  if (!row?.dataset.id) {
    return;
  }

  const selected = capturedRequests.find((entry) => entry.id === row.dataset.id);
  if (!selected) {
    return;
  }

  const draft: RuleDraft = {
    method: selected.method,
    url: selected.url,
    responseStatus: selected.status,
    responseBody: selected.responseBody,
    requestBody: selected.requestBody
  };

  void sendCommand<RewriteRule>({ type: "IMPORT_DRAFT", draft }).then((rule) => {
    captureStatus.textContent = `Draft imported as rule: ${rule.name}`;
  });
});

render();
