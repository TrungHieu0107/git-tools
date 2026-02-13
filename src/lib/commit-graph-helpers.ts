export function chooseBaseContent(parentContents: string[], modified: string): string {
  if (parentContents.length === 0) return "";
  const firstDifferent = parentContents.find((content) => content !== modified);
  return firstDifferent ?? parentContents[0];
}

export function buildCurvedConnectionPath(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  turnAtStart: boolean,
): string {
  if (x1 === x2) {
    return `M ${x1} ${y1} V ${y2}`;
  }

  if (y1 === y2) {
    return `M ${x1} ${y1} H ${x2}`;
  }

  const dx = x2 > x1 ? 1 : -1;
  const dy = y2 > y1 ? 1 : -1;
  const horizontalGap = Math.abs(x2 - x1);
  const verticalGap = Math.abs(y2 - y1);
  const radius = Math.min(8, horizontalGap / 2, verticalGap);

  if (radius < 0.5) {
    return turnAtStart ? `M ${x1} ${y1} H ${x2} V ${y2}` : `M ${x1} ${y1} V ${y2} H ${x2}`;
  }

  if (turnAtStart) {
    const xBeforeCorner = x2 - dx * radius;
    const yAfterCorner = y1 + dy * radius;
    return [`M ${x1} ${y1}`, `H ${xBeforeCorner}`, `Q ${x2} ${y1} ${x2} ${yAfterCorner}`, `V ${y2}`].join(
      " ",
    );
  }

  const yBeforeCorner = y2 - dy * radius;
  const xAfterCorner = x1 + dx * radius;
  return [`M ${x1} ${y1}`, `V ${yBeforeCorner}`, `Q ${x1} ${y2} ${xAfterCorner} ${y2}`, `H ${x2}`].join(
    " ",
  );
}

export function getTreePath(filePath: string): string {
  const normalized = filePath.replaceAll("\\", "/");
  const renameParts = normalized.split(" -> ");
  return (renameParts[renameParts.length - 1] ?? normalized).trim();
}

export function getBaseName(filePath: string): string {
  const path = getTreePath(filePath);
  const segments = path.split("/").filter(Boolean);
  return segments.length > 0 ? segments[segments.length - 1] : path;
}

export function collapseSinglePath(path: string, maxLength: number, collapseToken: string): string {
  const normalized = path.replaceAll("\\", "/").trim();
  if (normalized.length <= maxLength) return normalized;

  const segments = normalized.split("/").filter(Boolean);
  if (segments.length <= 1) {
    return `...${normalized.slice(-Math.max(1, maxLength - 3))}`;
  }

  let first = segments[0];
  let last = segments[segments.length - 1];
  let candidate = `${first}/${collapseToken}/${last}`;

  if (candidate.length > maxLength) {
    const lastBudget = Math.max(10, maxLength - (first.length + collapseToken.length + 5));
    if (last.length > lastBudget) {
      last = `...${last.slice(-Math.max(1, lastBudget - 3))}`;
    }
    candidate = `${first}/${collapseToken}/${last}`;
  }

  if (candidate.length > maxLength) {
    const firstBudget = Math.max(3, maxLength - (collapseToken.length + last.length + 5));
    if (first.length > firstBudget) {
      first = `${first.slice(0, Math.max(1, firstBudget - 3))}...`;
    }
    candidate = `${first}/${collapseToken}/${last}`;
  }

  if (candidate.length <= maxLength) return candidate;
  return `...${normalized.slice(-Math.max(1, maxLength - 3))}`;
}

export function formatPathLabel(path: string, maxLength: number, collapseToken: string): string {
  const normalized = path.replaceAll("\\", "/").trim();
  const renameParts = normalized.split(" -> ").map((part) => part.trim()).filter(Boolean);

  if (renameParts.length === 2) {
    const leftBudget = Math.max(16, Math.floor((maxLength - 4) / 2));
    const rightBudget = Math.max(16, maxLength - 4 - leftBudget);
    const left = collapseSinglePath(renameParts[0], leftBudget, collapseToken);
    const right = collapseSinglePath(renameParts[1], rightBudget, collapseToken);
    return `${left} -> ${right}`;
  }

  return collapseSinglePath(normalized, maxLength, collapseToken);
}
