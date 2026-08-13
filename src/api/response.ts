/** Release an intentionally unused response body immediately. */
export async function discardResponseBody(response: Response): Promise<void> {
  await response.body?.cancel();
}
