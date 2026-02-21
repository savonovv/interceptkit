import { PROXY_RELEASE_URL } from "./lib/constants";
import { sendCommand } from "./lib/runtime";
import type { ProxyStatus, RewriteRule, SetupStatus } from "./lib/types";

function byId<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`Missing element #${id}`);
  }
  return element as T;
}

const statusGrid = byId<HTMLDivElement>("statusGrid");
const rulesList = byId<HTMLDivElement>("rulesList");
const activity = byId<HTMLParagraphElement>("activity");
const runChecksButton = byId<HTMLButtonElement>("runChecks");
const enableProxyButton = byId<HTMLButtonElement>("enableProxy");
const restoreProxyButton = byId<HTMLButtonElement>("restoreProxy");
const toggleInterceptionButton = byId<HTMLButtonElement>("toggleInterception");
const downloadProxyButton = byId<HTMLButtonElement>("downloadProxy");
const ruleForm = byId<HTMLFormElement>("ruleForm");

let latestStatus: SetupStatus | undefined;

function renderInlineError(message: string): void {
  activity.textContent = `Error: ${message}`;
  statusGrid.innerHTML = `<article class="status-item"><b>Setup Error</b><span class="warn">Needs setup</span><div class="small">${message}</div></article>`;
}

function statusItem(label: string, ok: boolean, details: string): string {
  return `<article class="status-item"><b>${label}</b><span class="${ok ? "ok" : "warn"}">${
    ok ? "OK" : "Needs setup"
  }</span><div class="small">${details}</div></article>`;
}

function renderStatus(status: SetupStatus): void {
  activity.textContent = `Setup checked at ${new Date().toLocaleTimeString()}.`;
  latestStatus = status;
  const checklist = status.checklist;
  statusGrid.innerHTML = [
    statusItem("Proxy reachable", checklist.reachable, checklist.message),
    statusItem(
      "Protocol compatible",
      checklist.protocolCompatible,
      status.proxyVersion
        ? `v${status.proxyVersion.version} / protocol ${status.proxyVersion.protocolVersion}`
        : "Unknown"
    ),
    statusItem(
      "Proxy configured",
      checklist.proxyConfigured,
      checklist.proxyDebug
        ? `Browser proxy must point to local daemon. ${checklist.proxyDebug}`
        : "Browser proxy must point to local daemon"
    ),
    statusItem("Cert ready", checklist.certReady, "Required for HTTPS MITM rewriting"),
    statusItem("MITM ready", checklist.mitmReady, "Proxy cert trust + interception enabled"),
    statusItem("Diagnostics", checklist.diagnosticsOk, "Rule engine and control API response")
  ].join("");

  if (status.proxyStatus) {
    toggleInterceptionButton.textContent = status.proxyStatus.interceptionEnabled
      ? "Disable Interception"
      : "Enable Interception";
  }
}

function renderRules(rules: RewriteRule[]): void {
  if (rules.length === 0) {
    rulesList.innerHTML = '<p class="small">No rules yet.</p>';
    return;
  }

  rulesList.innerHTML = rules
    .map(
      (rule) => `<article class="rule-row" data-rule-id="${rule.id}">
        <div>
          <b>${rule.name}</b>
          <div class="small">${rule.matcher.methods?.join(",") ?? "ANY"} ${rule.matcher.urlPattern}</div>
          <div class="small">priority=${rule.priority} enabled=${rule.enabled}</div>
        </div>
        <button class="danger" data-action="delete">Delete</button>
      </article>`
    )
    .join("");
}

async function refreshStatus(): Promise<void> {
  activity.textContent = "Running setup check...";
  try {
    const status = await sendCommand<SetupStatus>({ type: "RUN_SETUP_CHECK" });
    renderStatus(status);
  } catch (error) {
    renderInlineError(error instanceof Error ? error.message : "Failed to run setup check");
  }
}

async function refreshRules(): Promise<void> {
  try {
    const rules = await sendCommand<RewriteRule[]>({ type: "LIST_RULES" });
    renderRules(rules);
  } catch (error) {
    rulesList.innerHTML = `<p class="small">${
      error instanceof Error ? error.message : "Failed to load rules"
    }</p>`;
  }
}

function createRuleFromForm(): RewriteRule {
  const ruleName = byId<HTMLInputElement>("ruleName").value.trim();
  const ruleMethod = byId<HTMLSelectElement>("ruleMethod").value.trim();
  const rulePattern = byId<HTMLInputElement>("rulePattern").value.trim();
  const ruleMode = byId<HTMLSelectElement>("ruleMode").value;
  const rulePriority = Number.parseInt(byId<HTMLInputElement>("rulePriority").value, 10) || 0;
  const ruleStatus = Number.parseInt(byId<HTMLInputElement>("ruleStatus").value, 10) || 200;
  const ruleBody = byId<HTMLTextAreaElement>("ruleBody").value;

  const now = new Date().toISOString();

  const action =
    ruleMode === "mockResponse"
      ? {
          type: "mockResponse" as const,
          status: ruleStatus,
          contentType: "application/json",
          body: ruleBody || "{}"
        }
      : {
          type: "rewritePassThrough" as const,
          response: {
            replaceBody: ruleBody || "{}",
            setHeaders: {
              "content-type": "application/json"
            }
          }
        };

  return {
    id: crypto.randomUUID(),
    name: ruleName,
    enabled: true,
    priority: rulePriority,
    matcher: {
      methods: ruleMethod ? [ruleMethod] : undefined,
      urlPattern: rulePattern
    },
    action,
    tags: ["manual"],
    createdAt: now,
    updatedAt: now
  };
}

async function toggleInterception(): Promise<void> {
  if (!latestStatus?.proxyStatus) {
    return;
  }

  const nextValue = !latestStatus.proxyStatus.interceptionEnabled;
  await sendCommand<ProxyStatus>({
    type: "SET_INTERCEPTION",
    enabled: nextValue
  });
  await refreshStatus();
}

runChecksButton.addEventListener("click", () => {
  void refreshStatus();
});

enableProxyButton.addEventListener("click", () => {
  activity.textContent = "Enabling browser proxy...";
  void sendCommand<SetupStatus>({ type: "ENABLE_PROXY" })
    .then(renderStatus)
    .catch((error) => {
      renderInlineError(error instanceof Error ? error.message : "Enable proxy failed");
    });
});

restoreProxyButton.addEventListener("click", () => {
  activity.textContent = "Restoring browser proxy...";
  void sendCommand<SetupStatus>({ type: "RESTORE_PROXY" })
    .then(renderStatus)
    .catch((error) => {
      renderInlineError(error instanceof Error ? error.message : "Restore proxy failed");
    });
});

toggleInterceptionButton.addEventListener("click", () => {
  void toggleInterception().catch((error) => {
    renderInlineError(error instanceof Error ? error.message : "Toggle interception failed");
  });
});

downloadProxyButton.addEventListener("click", () => {
  window.open(PROXY_RELEASE_URL, "_blank", "noopener,noreferrer");
});

ruleForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const rule = createRuleFromForm();
  void sendCommand<RewriteRule>({ type: "CREATE_RULE", rule }).then(async () => {
    ruleForm.reset();
    await refreshRules();
    await refreshStatus();
  }).catch((error) => {
    renderInlineError(error instanceof Error ? error.message : "Create rule failed");
  });
});

rulesList.addEventListener("click", (event) => {
  const target = event.target as HTMLElement;
  const deleteButton = target.closest("button[data-action='delete']");
  if (!deleteButton) {
    return;
  }

  const row = target.closest<HTMLElement>("[data-rule-id]");
  const id = row?.dataset.ruleId;
  if (!id) {
    return;
  }

  void sendCommand<{ ok: true }>({ type: "DELETE_RULE", id }).then(async () => {
    await refreshRules();
    await refreshStatus();
  }).catch((error) => {
    renderInlineError(error instanceof Error ? error.message : "Delete rule failed");
  });
});

void Promise.all([refreshStatus(), refreshRules()]);
