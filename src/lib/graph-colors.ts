export const BRANCH_COLORS = [
  "#4a90d9",
  "#37a349",
  "#d4b44e",
  "#e06860",
  "#a57ad9",
  "#6aabdb",
  "#db904a",
  "#b893db",
  "#49b856",
] as const;

export function getColorForColumn(index: number): string {
  return BRANCH_COLORS[index % BRANCH_COLORS.length];
}
