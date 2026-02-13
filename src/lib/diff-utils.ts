import type { DiffLineType } from "./diff-types";

const MAX_DIFF_LINES = 10_000;

export function mapLineType(backendType: string): DiffLineType {
  if (backendType === "context") return "equal";
  if (backendType === "add") return "added";
  if (backendType === "remove") return "removed";
  return "equal";
}

export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

export function isLargeFile(content: string): boolean {
  let count = 0;
  for (let i = 0; i < content.length; i++) {
    if (content[i] === "\n") count++;
    if (count > MAX_DIFF_LINES) return true;
  }
  return false;
}
