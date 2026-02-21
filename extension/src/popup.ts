import { sendCommand } from "./lib/runtime";
import type { SetupStatus } from "./lib/types";
import { openOptionsPage } from "./lib/webext";

const quickStatus = document.getElementById("quickStatus") as HTMLDivElement;
const openOptions = document.getElementById("openOptions") as HTMLButtonElement;

openOptions.addEventListener("click", () => {
  void openOptionsPage();
});

async function refreshQuickStatus(): Promise<void> {
  try {
    const status = await sendCommand<SetupStatus>({ type: "RUN_SETUP_CHECK" });
    const checklist = status.checklist;
    quickStatus.textContent = checklist.proxyConfigured
      ? "Proxy enabled and ready"
      : checklist.message;
  } catch (error) {
    quickStatus.textContent =
      error instanceof Error ? error.message : "Could not check setup";
  }
}

void refreshQuickStatus();
