export async function api(path, options = {}) {
  const response = await fetch(path, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      ...options.headers,
    },
  });
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    try {
      message = (await response.json()).error || message;
    } catch {
      /* Non-JSON proxy errors. */
    }
    throw new Error(message);
  }
  return response.status === 204 ? null : response.json();
}
export function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}
