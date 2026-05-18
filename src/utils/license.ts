/**
 * License classification for media providers. Backend catalog at
 * crates/app-server/src/providers/catalog.rs is the source of truth for the
 * string values; this helper just decides whether the string represents an
 * OSS license that's allowed under the user's licenseRestrictedMode toggle.
 *
 * Keep as a strict allowlist - new licenses default to non-OSS until added
 * here.
 */
export function isOssLicense(license: string): boolean {
  return license.startsWith('Apache 2.0') || license === 'MIT';
}
