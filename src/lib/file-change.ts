export type FileChangeKind =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "copied"
  | "type"
  | "conflict"
  | "untracked"
  | "unknown";

export interface FileChangeMeta {
  code: string;
  kind: FileChangeKind;
  label: string;
  textClass: string;
  bgClass: string;
  borderClass: string;
}

export function normalizeFileStatus(status: string | null | undefined): string {
  const normalized = (status ?? "").trim().toUpperCase();
  if (!normalized) return "M";
  if (normalized === "??") return "??";
  if (normalized.startsWith("R")) return "R";
  if (normalized.startsWith("C")) return "C";
  if (normalized.startsWith("A")) return "A";
  if (normalized.startsWith("M")) return "M";
  if (normalized.startsWith("D")) return "D";
  if (normalized.startsWith("T")) return "T";
  if (normalized.startsWith("U")) return "U";
  return "M";
}

export function getFileChangeMeta(status: string | null | undefined): FileChangeMeta {
  const code = normalizeFileStatus(status);
  switch (code) {
    case "A":
      return {
        code,
        kind: "added",
        label: "Added",
        textClass: "text-emerald-300",
        bgClass: "bg-emerald-500/10",
        borderClass: "border-emerald-500/35",
      };
    case "M":
      return {
        code,
        kind: "modified",
        label: "Modified",
        textClass: "text-amber-300",
        bgClass: "bg-amber-500/10",
        borderClass: "border-amber-500/35",
      };
    case "D":
      return {
        code,
        kind: "deleted",
        label: "Deleted",
        textClass: "text-rose-300",
        bgClass: "bg-rose-500/10",
        borderClass: "border-rose-500/35",
      };
    case "R":
      return {
        code,
        kind: "renamed",
        label: "Renamed",
        textClass: "text-sky-300",
        bgClass: "bg-sky-500/10",
        borderClass: "border-sky-500/35",
      };
    case "C":
      return {
        code,
        kind: "copied",
        label: "Copied",
        textClass: "text-cyan-300",
        bgClass: "bg-cyan-500/10",
        borderClass: "border-cyan-500/35",
      };
    case "T":
      return {
        code,
        kind: "type",
        label: "Type",
        textClass: "text-violet-300",
        bgClass: "bg-violet-500/10",
        borderClass: "border-violet-500/35",
      };
    case "U":
      return {
        code,
        kind: "conflict",
        label: "Conflict",
        textClass: "text-orange-300",
        bgClass: "bg-orange-500/10",
        borderClass: "border-orange-500/35",
      };
    case "??":
      return {
        code,
        kind: "untracked",
        label: "New",
        textClass: "text-slate-300",
        bgClass: "bg-slate-500/10",
        borderClass: "border-slate-500/35",
      };
    default:
      return {
        code: "M",
        kind: "unknown",
        label: "Changed",
        textClass: "text-gray-300",
        bgClass: "bg-gray-500/10",
        borderClass: "border-gray-500/35",
      };
  }
}
