type AnyObject = Record<string, unknown>;

const browserApi: AnyObject =
  (globalThis as unknown as { browser?: AnyObject }).browser ??
  (globalThis as unknown as { chrome?: AnyObject }).chrome ??
  {};

function runtimeLastErrorMessage(): string | undefined {
  const chromeApi = (globalThis as unknown as { chrome?: AnyObject }).chrome;
  const runtime = chromeApi?.runtime as AnyObject | undefined;
  const lastError = runtime?.lastError as { message?: string } | undefined;
  return lastError?.message;
}

function promisifyChromeCall<T>(fn: (...args: unknown[]) => void, ...args: unknown[]): Promise<T> {
  return new Promise((resolve, reject) => {
    fn(...args, (result: T) => {
      const errorMessage = runtimeLastErrorMessage();
      if (errorMessage) {
        reject(new Error(errorMessage));
        return;
      }
      resolve(result);
    });
  });
}

export async function storageGet<T extends AnyObject>(keys?: string | string[] | AnyObject): Promise<T> {
  const local = (browserApi.storage as AnyObject).local as AnyObject;

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    return (await (local.get as (keys?: unknown) => Promise<T>)(keys)) as T;
  }

  return promisifyChromeCall<T>(local.get as (...args: unknown[]) => void, keys);
}

export async function storageSet(data: AnyObject): Promise<void> {
  const local = (browserApi.storage as AnyObject).local as AnyObject;

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    await (local.set as (payload: unknown) => Promise<void>)(data);
    return;
  }

  await promisifyChromeCall<void>(local.set as (...args: unknown[]) => void, data);
}

export async function storageRemove(keys: string | string[]): Promise<void> {
  const local = (browserApi.storage as AnyObject).local as AnyObject;

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    await (local.remove as (input: string | string[]) => Promise<void>)(keys);
    return;
  }

  await promisifyChromeCall<void>(local.remove as (...args: unknown[]) => void, keys);
}

export async function runtimeSendMessage<T>(payload: unknown): Promise<T> {
  const runtime = browserApi.runtime as AnyObject;

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    return (await (runtime.sendMessage as (msg: unknown) => Promise<T>)(payload)) as T;
  }

  return promisifyChromeCall<T>(runtime.sendMessage as (...args: unknown[]) => void, payload);
}

export function addOnInstalledListener(callback: () => void): void {
  const runtime = browserApi.runtime as AnyObject;
  const onInstalled = runtime?.onInstalled as AnyObject | undefined;
  if (!onInstalled?.addListener) {
    return;
  }

  (onInstalled.addListener as (listener: () => void) => void)(callback);
}

export async function openOptionsPage(): Promise<void> {
  const runtime = browserApi.runtime as AnyObject;

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    await (runtime.openOptionsPage as () => Promise<void>)();
    return;
  }

  await promisifyChromeCall<void>(runtime.openOptionsPage as (...args: unknown[]) => void);
}

export async function proxyGet(): Promise<AnyObject> {
  const proxy = browserApi.proxy as AnyObject;
  const settings = proxy.settings as AnyObject;

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    return (await (settings.get as (details: AnyObject) => Promise<AnyObject>)({})) as AnyObject;
  }

  return promisifyChromeCall<AnyObject>(
    settings.get as (...args: unknown[]) => void,
    { incognito: false }
  );
}

export async function proxySet(value: AnyObject): Promise<void> {
  const proxy = browserApi.proxy as AnyObject;
  const settings = proxy.settings as AnyObject;
  const payload = {
    value,
    scope: "regular"
  };

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    await (settings.set as (details: AnyObject) => Promise<void>)({ value });
    return;
  }

  await promisifyChromeCall<void>(settings.set as (...args: unknown[]) => void, payload);
}

export async function proxyClear(): Promise<void> {
  const proxy = browserApi.proxy as AnyObject;
  const settings = proxy.settings as AnyObject;
  const payload = {
    scope: "regular"
  };

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    await (settings.clear as (details: AnyObject) => Promise<void>)({});
    return;
  }

  await promisifyChromeCall<void>(settings.clear as (...args: unknown[]) => void, payload);
}

export function setActionBadge(text: string, color: string): void {
  const action = browserApi.action as AnyObject;
  if (!action) {
    return;
  }

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    void (action.setBadgeText as (details: AnyObject) => Promise<void>)({ text });
    void (action.setBadgeBackgroundColor as (details: AnyObject) => Promise<void>)({ color });
    return;
  }

  (action.setBadgeText as (...args: unknown[]) => void)({ text });
  (action.setBadgeBackgroundColor as (...args: unknown[]) => void)({ color });
}

export function getRuntimeId(): string | undefined {
  const runtime = browserApi.runtime as AnyObject;
  return runtime?.id as string | undefined;
}

export function addMessageListener(
  handler: (message: unknown, sender: chrome.runtime.MessageSender) => Promise<unknown>
): void {
  const runtime = browserApi.runtime as AnyObject;
  const onMessage = runtime.onMessage as AnyObject;

  if ((globalThis as unknown as { browser?: AnyObject }).browser) {
    const listener = async (message: unknown, sender: chrome.runtime.MessageSender) => {
      try {
        const result = await handler(message, sender);
        return { ok: true, data: result };
      } catch (error: unknown) {
        const errorMessage =
          error instanceof Error ? error.message : "Unhandled background error";
        return { ok: false, error: errorMessage };
      }
    };

    (onMessage.addListener as (listener: (...args: unknown[]) => unknown) => void)(
      listener as unknown as (...args: unknown[]) => unknown
    );
    return;
  }

  const listener = ((
    message: unknown,
    sender: chrome.runtime.MessageSender,
    sendResponse: (payload: unknown) => void
  ) => {
    handler(message, sender)
      .then((result) => sendResponse({ ok: true, data: result }))
      .catch((error: unknown) => {
        const errorMessage =
          error instanceof Error ? error.message : "Unhandled background error";
        sendResponse({ ok: false, error: errorMessage });
      });
    return true;
  }) as unknown as (...args: unknown[]) => boolean;

  (onMessage.addListener as (listener: (...args: unknown[]) => boolean | void) => void)(listener);
}
