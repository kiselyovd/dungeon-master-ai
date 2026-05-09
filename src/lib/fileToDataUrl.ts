/**
 * Read a File / Blob into a base64 data URL. Used by the composer to stage
 * images before sending them as `MessagePart::Image` to the backend.
 */
export function fileToDataUrl(file: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

/**
 * Strip the `data:image/foo;base64,` prefix from a data URL and return the
 * raw base64 payload. Used when posting `MessagePart::Image` bodies to the
 * backend, which expects the payload only (no scheme).
 */
export function stripDataUrlPrefix(dataUrl: string): string {
  const idx = dataUrl.indexOf(',');
  return idx === -1 ? dataUrl : dataUrl.slice(idx + 1);
}
