const fs = require("fs");
const path = require("path");

const extensionRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(extensionRoot, "..", "..");
const binaryName = process.platform === "win32" ? "lsp.exe" : "lsp";
const source = path.join(repoRoot, "target", "release", binaryName);
const destination = path.join(extensionRoot, binaryName);

if (!fs.existsSync(source)) {
    throw new Error(`LSP binary was not built: ${source}`);
}

fs.copyFileSync(source, destination);
