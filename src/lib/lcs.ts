export type EditOp =
  | { kind: "equal"; baseLine: string; baseIdx: number; modIdx: number }
  | { kind: "removed"; baseLine: string; baseIdx: number }
  | { kind: "added"; modLine: string; modIdx: number };

export function computeLCS(baseLines: string[], modLines: string[]): number[][] {
  const m = baseLines.length;
  const n = modLines.length;

  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (baseLines[i - 1] === modLines[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  return dp;
}

export function backtrackEditOps(dp: number[][], baseLines: string[], modLines: string[]): EditOp[] {
  const ops: EditOp[] = [];
  let i = baseLines.length;
  let j = modLines.length;

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && baseLines[i - 1] === modLines[j - 1]) {
      ops.push({
        kind: "equal",
        baseLine: baseLines[i - 1],
        baseIdx: i,
        modIdx: j,
      });
      i--;
      j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      ops.push({ kind: "added", modLine: modLines[j - 1], modIdx: j });
      j--;
    } else {
      ops.push({ kind: "removed", baseLine: baseLines[i - 1], baseIdx: i });
      i--;
    }
  }

  ops.reverse();
  return ops;
}
