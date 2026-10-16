/** Drain an intentionally unused response body so WebView2 can reuse the connection. */
export async function discardResponseBody(response: Response): Promise<void> {
  await response.arrayBuffer();
}
