import {
  DEFAULT_CONTROL_BASE_URL,
  DEFAULT_PROXY_HOST,
  DEFAULT_PROXY_PORT,
  PROTOCOL_VERSION
} from "./lib/constants";
import type {
  ProxyStatus,
  ProxyVersion,
  RewriteRule,
  RuleDraft,
  RuntimeCommand,
  SetupChecklist,
  SetupStatus
} from "./lib/types";
import {
  addMessageListener,
  addOnInstalledListener,
  getRuntimeId,
  proxyClear,
  proxyGet,
  proxySet,
  setActionBadge,
  storageGet,
  storageRemove,
  storageSet
} from "./lib/webext";

interface LocalSettings {
  controlBaseUrl: string;
  proxyPort: number;
  savedProxyConfig?: Record<string, unknown>;
}

type RuleDiagnostics = {
  ok: boolean;
  matchedEnabledRule: boolean;
  enabledRuleCount: number;
};

function assertCommand(value: unknown): asserts value is RuntimeCommand {
  if (!value || typeof value !== "object" || !("type" in value)) {
    throw new Error("Invalid command payload");
  }
}

async function ensureDefaults(): Promise<void> {
  const settings = await storageGet<Partial<LocalSettings>>(["controlBaseUrl", "proxyPort"]);

  const nextSettings: Partial<LocalSettings> = {};
  if (!settings.controlBaseUrl) {
    nextSettings.controlBaseUrl = DEFAULT_CONTROL_BASE_URL;
  }

  if (!settings.proxyPort) {
    nextSettings.proxyPort = DEFAULT_PROXY_PORT;
  }

  if (Object.keys(nextSettings).length > 0) {
    await storageSet(nextSettings);
  }
}

async function readSettings(): Promise<LocalSettings> {
  const settings = await storageGet<Partial<LocalSettings>>([
    "controlBaseUrl",
    "proxyPort",
    "savedProxyConfig"
  ]);

  return {
    controlBaseUrl: settings.controlBaseUrl ?? DEFAULT_CONTROL_BASE_URL,
    proxyPort: settings.proxyPort ?? DEFAULT_PROXY_PORT,
    savedProxyConfig: settings.savedProxyConfig
  };
}

async function proxyRequest<T>(path: string, init?: RequestInit): Promise<T> {
  const settings = await readSettings();
  const response = await fetch(`${settings.controlBaseUrl}${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init?.headers ?? {})
    }
  });

  if (!response.ok) {
    const raw = await response.text();
    throw new Error(`Proxy API ${response.status}: ${raw}`);
  }

  if (response.status === 204) {
    return {} as T;
  }

  return (await response.json()) as T;
}

async function getProxyStatus(): Promise<ProxyStatus> {
  return proxyRequest<ProxyStatus>("/status");
}

function isProxyActive(proxyValue: Record<string, unknown> | undefined, expectedPort: number): boolean {
  if (!proxyValue) {
    return false;
  }

  const proxyType = proxyValue.proxyType as string | undefined;
  if (proxyType === "manual") {
    const http = parseHostPort(proxyValue.http as string | undefined, Number(proxyValue.httpPort ?? 0));
    const ssl = parseHostPort(proxyValue.ssl as string | undefined, Number(proxyValue.sslPort ?? 0));
    const httpHost = http.host;
    const httpPort = http.port;
    const sslHost = ssl.host;
    const sslPort = ssl.port;
    const hostMatch =
      (httpHost === DEFAULT_PROXY_HOST || httpHost === "localhost") &&
      (sslHost === DEFAULT_PROXY_HOST || sslHost === "localhost");
    return (
      hostMatch &&
      httpPort === expectedPort &&
      sslPort === expectedPort
    );
  }

  const mode = proxyValue.mode as string | undefined;
  if (mode !== "fixed_servers") {
    return false;
  }

  const rules = proxyValue.rules as Record<string, unknown> | undefined;
  const singleProxy = rules?.singleProxy as Record<string, unknown> | undefined;
  if (!singleProxy) {
    return false;
  }

  const host = singleProxy.host as string | undefined;
  const port = singleProxy.port as number | undefined;

  return host === DEFAULT_PROXY_HOST && port === expectedPort;
}

function parseHostPort(raw: string | undefined, explicitPort: number): { host: string; port: number } {
  if (!raw) {
    return { host: "", port: explicitPort || 0 };
  }

  const trimmed = raw.trim();
  const colonIndex = trimmed.lastIndexOf(":");
  if (colonIndex > -1 && colonIndex < trimmed.length - 1) {
    const host = trimmed.slice(0, colonIndex);
    const maybePort = Number(trimmed.slice(colonIndex + 1));
    if (Number.isFinite(maybePort) && maybePort > 0) {
      return { host, port: explicitPort || maybePort };
    }
  }

  return { host: trimmed, port: explicitPort || 0 };
}

async function runSetupCheck(): Promise<SetupStatus> {
  const settings = await readSettings();

  let checklist: SetupChecklist = {
    reachable: false,
    protocolCompatible: false,
    proxyConfigured: false,
    certReady: false,
    mitmReady: false,
    diagnosticsOk: false,
    message: "Proxy not reachable"
  };

  let proxyStatus: ProxyStatus | undefined;
  let proxyVersion: ProxyVersion | undefined;
  let rawProxyValue: Record<string, unknown> | undefined;

  try {
    await proxyRequest<{ ok: boolean }>("/health");
    checklist.reachable = true;
    proxyVersion = await proxyRequest<ProxyVersion>("/version");
    checklist.protocolCompatible = proxyVersion.protocolVersion === PROTOCOL_VERSION;
    proxyStatus = await getProxyStatus();
    checklist.certReady = proxyStatus.certReady;
    checklist.mitmReady = proxyStatus.mitmReady;

    const diagnostics = await proxyRequest<RuleDiagnostics>("/diagnostics/rewrite-check", {
      method: "POST",
      body: JSON.stringify({})
    });

    checklist.diagnosticsOk = diagnostics.ok;

    const currentProxy = (await proxyGet()) as {
      value?: Record<string, unknown>;
      [key: string]: unknown;
    };
    rawProxyValue =
      (currentProxy.value as Record<string, unknown> | undefined) ??
      (currentProxy as Record<string, unknown>);
    checklist.proxyConfigured = isProxyActive(rawProxyValue, settings.proxyPort);
    checklist.proxyDebug = formatProxyDebug(rawProxyValue);

    checklist.message = checklist.proxyConfigured
      ? "Ready for interception"
      : "Proxy binary is up. Enable browser proxy to start intercepting.";
  } catch (error) {
    checklist.message =
      error instanceof Error ? error.message : "Failed to run setup checks";
  }

  setActionBadge(checklist.reachable && checklist.proxyConfigured ? "ON" : "OFF", checklist.reachable && checklist.proxyConfigured ? "#1c8c45" : "#8b2f2f");

  return {
    checklist,
    proxyStatus,
    proxyVersion
  };
}

function formatProxyDebug(proxyValue: Record<string, unknown> | undefined): string {
  if (!proxyValue) {
    return "proxy settings unavailable";
  }

  const proxyType = (proxyValue.proxyType as string | undefined) ??
    (proxyValue.mode as string | undefined) ??
    "unknown";
  const host =
    (proxyValue.http as string | undefined) ??
    (proxyValue.host as string | undefined) ??
    ((proxyValue.rules as Record<string, unknown> | undefined)?.singleProxy as
      | Record<string, unknown>
      | undefined)?.host as string | undefined;
  const parsedHostPort = parseHostPort(host, Number(proxyValue.httpPort ?? 0));
  const port =
    parsedHostPort.port ||
    Number(proxyValue.port ?? 0) ||
    Number(
      ((proxyValue.rules as Record<string, unknown> | undefined)?.singleProxy as
        | Record<string, unknown>
        | undefined)?.port ?? 0
    );

  return `type=${proxyType} host=${parsedHostPort.host || host || "n/a"} port=${port || "n/a"}`;
}

async function enableProxy(): Promise<SetupStatus> {
  const settings = await readSettings();
  const current = (await proxyGet()) as { value?: Record<string, unknown> };

  if (!settings.savedProxyConfig) {
    await storageSet({ savedProxyConfig: current.value ?? null });
  }

  const isFirefox = typeof (globalThis as { browser?: unknown }).browser !== "undefined";

  if (isFirefox) {
    await proxySet({
      proxyType: "manual",
      http: `${DEFAULT_PROXY_HOST}:${settings.proxyPort}`,
      httpPort: settings.proxyPort,
      ssl: `${DEFAULT_PROXY_HOST}:${settings.proxyPort}`,
      sslPort: settings.proxyPort,
      passthrough: "localhost,127.0.0.1",
      proxyDNS: false
    });
  } else {
    await proxySet({
      mode: "fixed_servers",
      rules: {
        singleProxy: {
          scheme: "http",
          host: DEFAULT_PROXY_HOST,
          port: settings.proxyPort
        },
        bypassList: ["<local>", "localhost", "127.0.0.1"]
      }
    });
  }

  return runSetupCheck();
}

async function restoreProxy(): Promise<SetupStatus> {
  const settings = await readSettings();

  if (settings.savedProxyConfig) {
    await proxySet(settings.savedProxyConfig);
  } else {
    await proxyClear();
  }

  await storageRemove("savedProxyConfig");
  return runSetupCheck();
}

async function setInterception(enabled: boolean): Promise<ProxyStatus> {
  return proxyRequest<ProxyStatus>("/status/interception", {
    method: "POST",
    body: JSON.stringify({ enabled })
  });
}

async function listRules(): Promise<RewriteRule[]> {
  return proxyRequest<RewriteRule[]>("/rules");
}

async function createRule(rule: RewriteRule): Promise<RewriteRule> {
  return proxyRequest<RewriteRule>("/rules", {
    method: "POST",
    body: JSON.stringify(rule)
  });
}

async function deleteRule(id: string): Promise<void> {
  await proxyRequest<void>(`/rules/${encodeURIComponent(id)}`, {
    method: "DELETE"
  });
}

function buildRuleFromDraft(draft: RuleDraft): RewriteRule {
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID(),
    name: `Captured ${draft.method} ${new URL(draft.url).pathname}`,
    enabled: true,
    priority: 0,
    matcher: {
      methods: [draft.method],
      urlPattern: draft.url
    },
    action: {
      type: "mockResponse",
      status: draft.responseStatus ?? 200,
      contentType: "application/json",
      body: draft.responseBody ?? "{}"
    },
    tags: ["captured", "devtools"],
    createdAt: now,
    updatedAt: now
  };
}

async function handleCommand(command: RuntimeCommand): Promise<unknown> {
  switch (command.type) {
    case "GET_STATUS":
      return getProxyStatus();
    case "RUN_SETUP_CHECK":
      return runSetupCheck();
    case "ENABLE_PROXY":
      return enableProxy();
    case "RESTORE_PROXY":
      return restoreProxy();
    case "SET_INTERCEPTION":
      return setInterception(command.enabled);
    case "LIST_RULES":
      return listRules();
    case "CREATE_RULE":
      return createRule(command.rule);
    case "DELETE_RULE":
      await deleteRule(command.id);
      return { ok: true };
    case "IMPORT_DRAFT": {
      const rule = buildRuleFromDraft(command.draft);
      return createRule(rule);
    }
    default:
      throw new Error("Unsupported command");
  }
}

addMessageListener(async (message, sender) => {
  if (sender.id && sender.id !== getRuntimeId()) {
    throw new Error("Untrusted sender");
  }

  assertCommand(message);
  return handleCommand(message);
});

addOnInstalledListener(() => {
  void ensureDefaults();
});

void ensureDefaults();
