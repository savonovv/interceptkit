export interface RuleMatcher {
  methods?: string[];
  urlPattern: string;
  headerEquals?: Record<string, string>;
  queryEquals?: Record<string, string>;
  bodyContains?: string;
}

export interface TransformOps {
  setHeaders?: Record<string, string>;
  removeHeaders?: string[];
  replaceBody?: string;
  jsonSet?: Record<string, unknown>;
}

export interface MockResponseAction {
  type: "mockResponse";
  status: number;
  headers?: Record<string, string>;
  body?: string;
  contentType?: string;
  delayMs?: number;
}

export interface RewritePassThroughAction {
  type: "rewritePassThrough";
  request?: TransformOps;
  response?: TransformOps;
  delayMs?: number;
}

export interface SequenceStep {
  action: MockResponseAction | RewritePassThroughAction;
}

export interface SequenceAction {
  type: "sequence";
  steps: SequenceStep[];
}

export type RuleAction = MockResponseAction | RewritePassThroughAction | SequenceAction;

export interface RewriteRule {
  id: string;
  name: string;
  enabled: boolean;
  priority: number;
  matcher: RuleMatcher;
  action: RuleAction;
  tags?: string[];
  createdAt: string;
  updatedAt: string;
}

export interface ProxyStatus {
  interceptionEnabled: boolean;
  proxyPort: number;
  controlPort: number;
  certReady: boolean;
  mitmReady: boolean;
  ruleCount: number;
  recentEventCount: number;
  lastError?: string;
}

export interface SetupChecklist {
  reachable: boolean;
  protocolCompatible: boolean;
  proxyConfigured: boolean;
  proxyDebug?: string;
  certReady: boolean;
  mitmReady: boolean;
  diagnosticsOk: boolean;
  message: string;
}

export interface ProxyVersion {
  name: string;
  version: string;
  protocolVersion: number;
}

export interface SetupStatus {
  checklist: SetupChecklist;
  proxyStatus?: ProxyStatus;
  proxyVersion?: ProxyVersion;
}

export interface RuleDraft {
  method: string;
  url: string;
  responseStatus?: number;
  responseBody?: string;
  requestBody?: string;
}

export interface RuntimeResponse<T> {
  ok: boolean;
  data?: T;
  error?: string;
}

export type RuntimeCommand =
  | { type: "GET_STATUS" }
  | { type: "RUN_SETUP_CHECK" }
  | { type: "ENABLE_PROXY" }
  | { type: "RESTORE_PROXY" }
  | { type: "SET_INTERCEPTION"; enabled: boolean }
  | { type: "LIST_RULES" }
  | { type: "CREATE_RULE"; rule: RewriteRule }
  | { type: "DELETE_RULE"; id: string }
  | { type: "IMPORT_DRAFT"; draft: RuleDraft };
