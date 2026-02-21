import { build, context } from "esbuild";
import { cp, mkdir, rm } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const distDir = resolve(root, "dist");
const publicDir = resolve(root, "public");
const watch = process.argv.includes("--watch");
const targetArg = process.argv.find((arg) => arg.startsWith("--target="));
const explicitTarget = targetArg?.split("=")[1];
const targets = explicitTarget ? [explicitTarget] : ["chrome", "firefox"];

const baseConfig = {
  entryPoints: [
    "src/background.ts",
    "src/options.ts",
    "src/popup.ts",
    "src/devtools.ts",
    "src/panel.ts"
  ],
  absWorkingDir: root,
  bundle: true,
  format: "iife",
  platform: "browser",
  target: ["chrome114", "firefox118"],
  sourcemap: true,
  logLevel: "info"
};

function manifestFileForTarget(target) {
  if (target === "chrome") {
    return "manifest.chrome.json";
  }

  if (target === "firefox") {
    return "manifest.firefox.json";
  }

  throw new Error(`Unsupported target: ${target}`);
}

async function copyPublicFiles(target, targetDistDir) {
  await cp(publicDir, targetDistDir, {
    recursive: true,
    force: true,
    filter: (source) => {
      return !source.endsWith("manifest.chrome.json") && !source.endsWith("manifest.firefox.json");
    }
  });

  await cp(
    resolve(publicDir, manifestFileForTarget(target)),
    resolve(targetDistDir, "manifest.json"),
    { force: true }
  );
}

async function run() {
  await rm(distDir, { recursive: true, force: true });
  await mkdir(distDir, { recursive: true });

  if (watch) {
    if (targets.length !== 1) {
      throw new Error("Watch mode requires a single target, use --target=chrome or --target=firefox");
    }

    const target = targets[0];
    const targetDistDir = resolve(distDir, target);
    await mkdir(targetDistDir, { recursive: true });

    const ctx = await context({
      ...baseConfig,
      outdir: targetDistDir
    });
    await ctx.watch();
    await copyPublicFiles(target, targetDistDir);
    console.log(`Watching extension sources for ${target}...`);
    return;
  }

  for (const target of targets) {
    const targetDistDir = resolve(distDir, target);
    await mkdir(targetDistDir, { recursive: true });

    await build({
      ...baseConfig,
      outdir: targetDistDir
    });

    await copyPublicFiles(target, targetDistDir);
  }
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
