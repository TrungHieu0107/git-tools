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

export function renderWhitespace(escapedHtml: string): string {
  return escapedHtml
    .replace(/ /g, '<span class="relative before:content-[\'·\'] before:absolute before:inset-0 before:flex before:items-center before:justify-center before:text-[#8b949e]/40 before:pointer-events-none before:select-none"> </span>')
    .replace(/\t/g, '<span class="line-through decoration-[#8b949e]/40 decoration-1">\t</span>');
}


export function isLargeFile(content: string): boolean {
  let count = 0;
  for (let i = 0; i < content.length; i++) {
    if (content[i] === "\n") count++;
    if (count > MAX_DIFF_LINES) return true;
  }
  return false;
}
