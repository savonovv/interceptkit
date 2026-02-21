import type { RuntimeCommand, RuntimeResponse } from "./types";
import { runtimeSendMessage } from "./webext";

export async function sendCommand<T>(command: RuntimeCommand): Promise<T> {
  const response = await runtimeSendMessage<RuntimeResponse<T>>(command);
  if (!response.ok) {
    throw new Error(response.error ?? "Command failed");
  }

  return response.data as T;
}
