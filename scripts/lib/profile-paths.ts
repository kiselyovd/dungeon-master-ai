export function sqliteProfileDatabaseUrl(profileDirectory: string): string {
  const normalizedDirectory = profileDirectory.replaceAll('\\', '/').replace(/\/$/, '');
  const absolutePath = normalizedDirectory.startsWith('/')
    ? normalizedDirectory
    : `/${normalizedDirectory}`;
  return `sqlite://${encodeURI(`${absolutePath}/campaign.db`)}?mode=rwc`;
}
