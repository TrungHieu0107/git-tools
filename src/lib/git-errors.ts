export function getAuthRequiredMessage(raw: string): string | null {
  const text = raw.toLowerCase();
  const isAuthError =
    text.includes("authentication failed") ||
    text.includes("terminal prompts disabled") ||
    text.includes("could not read username") ||
    text.includes("could not read password") ||
    text.includes("permission denied (publickey)") ||
    text.includes("http basic: access denied") ||
    text.includes("returned error: 401") ||
    text.includes("returned error: 403");

  if (!isAuthError) return null;
  return "Authentication required. Git credentials are needed for this operation.";
}
