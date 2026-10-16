import { describe, expect, it } from 'vitest';

import { sqliteProfileDatabaseUrl } from './profile-paths';

describe('sqliteProfileDatabaseUrl', () => {
  it('normalizes a Windows profile path into a SQLx SQLite URL', () => {
    expect(sqliteProfileDatabaseUrl('D:\\Games\\Dungeon Master AI')).toBe(
      'sqlite:///D:/Games/Dungeon%20Master%20AI/campaign.db?mode=rwc',
    );
  });

  it('normalizes a POSIX profile path into a SQLx SQLite URL', () => {
    expect(sqliteProfileDatabaseUrl('/tmp/dmai profile')).toBe(
      'sqlite:///tmp/dmai%20profile/campaign.db?mode=rwc',
    );
  });
});
