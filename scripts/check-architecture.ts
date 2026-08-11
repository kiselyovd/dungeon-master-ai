import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

interface ExceptionEntry {
  rule: string;
  source: string;
  target: string;
  reason: string;
}

interface ArchitectureConfig {
  version: number;
  rustWorkspaceDependencies: Record<string, string[]>;
  exceptions: ExceptionEntry[];
}

interface CargoDependency {
  kind: 'build' | 'dev' | null;
  name: string;
  path: string | null;
}

interface CargoPackage {
  dependencies: CargoDependency[];
  id: string;
  name: string;
}

interface CargoMetadata {
  packages: CargoPackage[];
  workspace_members: string[];
}

interface Violation {
  rule: string;
  source: string;
  target: string;
}

const scriptDir = dirname(fileURLToPath(import.meta.url));
const root = resolve(scriptDir, '..');
const configPath = resolve(scriptDir, 'architecture-allowlist.json');
const verbose = process.env.ARCH_CHECK_VERBOSE === '1';

function log(level: 'DEBUG' | 'INFO' | 'ERROR', message: string): void {
  if (level === 'DEBUG' && !verbose) return;
  const stream = level === 'ERROR' ? console.error : console.log;
  stream(`[architecture:${level}] ${message}`);
}

function violationKey(item: Pick<Violation, 'rule' | 'source' | 'target'>): string {
  return `${item.rule}|${item.source}|${item.target}`;
}

function normalizePath(path: string): string {
  return path.replaceAll('\\', '/');
}

function cargoMetadata(): CargoMetadata {
  log('DEBUG', 'running cargo metadata --format-version 1 --no-deps');
  const result = Bun.spawnSync(['cargo', 'metadata', '--format-version', '1', '--no-deps'], {
    cwd: root,
    stderr: 'pipe',
    stdout: 'pipe',
  });
  if (result.exitCode !== 0) {
    const stderr = result.stderr.toString().trim();
    throw new Error(`cargo metadata failed (${result.exitCode}): ${stderr}`);
  }
  return JSON.parse(result.stdout.toString()) as CargoMetadata;
}

function rustViolations(config: ArchitectureConfig): Violation[] {
  const metadata = cargoMetadata();
  const members = new Set(metadata.workspace_members);
  const packages = metadata.packages.filter((pkg) => members.has(pkg.id));
  const packageNames = new Set(packages.map((pkg) => pkg.name));
  const configuredNames = new Set(Object.keys(config.rustWorkspaceDependencies));
  const violations: Violation[] = [];

  for (const name of packageNames) {
    if (!configuredNames.has(name)) {
      violations.push({ rule: 'rust-unconfigured-workspace-crate', source: name, target: name });
    }
  }

  for (const pkg of packages) {
    const allowed = new Set(config.rustWorkspaceDependencies[pkg.name] ?? []);
    const productionDependencies = pkg.dependencies.filter((dep) => dep.kind !== 'dev');
    for (const dependency of productionDependencies) {
      if (dependency.path && packageNames.has(dependency.name) && !allowed.has(dependency.name)) {
        violations.push({
          rule: 'rust-workspace-dependency',
          source: pkg.name,
          target: dependency.name,
        });
      }
    }
  }

  const forbiddenByPackage: Record<string, Set<string>> = {
    'app-domain': new Set([
      'axum',
      'fastembed',
      'genai',
      'iota_stronghold',
      'reqwest',
      'serde_json',
      'sqlx',
      'tauri',
      'tokio',
      'tracing',
    ]),
    'app-application': new Set([
      'app-llm',
      'axum',
      'genai',
      'iota_stronghold',
      'reqwest',
      'sqlx',
      'tauri',
    ]),
  };

  for (const pkg of packages) {
    const forbidden = forbiddenByPackage[pkg.name];
    if (!forbidden) continue;
    for (const dependency of pkg.dependencies.filter((dep) => dep.kind !== 'dev')) {
      if (forbidden.has(dependency.name) || dependency.name.startsWith('adapter-')) {
        violations.push({
          rule: 'rust-forbidden-dependency',
          source: pkg.name,
          target: dependency.name,
        });
      }
    }
  }

  return violations;
}

function sourceFiles(directory: string): string[] {
  if (!existsSync(directory)) return [];
  const files: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === '__tests__') continue;
      files.push(...sourceFiles(path));
    } else if (/\.(ts|tsx)$/.test(entry.name) && !/\.(test|spec)\.(ts|tsx)$/.test(entry.name)) {
      files.push(path);
    }
  }
  return files;
}

function resolvedImport(source: string, specifier: string): string | null {
  if (!specifier.startsWith('.')) return null;
  return normalizePath(relative(root, resolve(dirname(source), specifier)));
}

function frontendViolations(): Violation[] {
  const files = ['api', 'state', 'ui', 'features'].flatMap((directory) =>
    sourceFiles(resolve(root, 'src', directory)),
  );
  const violations: Violation[] = [];

  for (const file of files) {
    const source = normalizePath(relative(root, file));
    const imports = readFileSync(file, 'utf8').matchAll(/\bfrom\s+['"]([^'"]+)['"]/g);
    for (const match of imports) {
      const target = resolvedImport(file, match[1]);
      if (!target) continue;

      if (source.startsWith('src/api/') && /^(src\/state|src\/components)(\/|$)/.test(target)) {
        violations.push({ rule: 'frontend-api-state', source, target });
      }
      if (source.startsWith('src/state/') && /^src\/components(\/|$)/.test(target)) {
        violations.push({ rule: 'frontend-state-component', source, target });
      }
      if (
        source.startsWith('src/ui/') &&
        /^src\/(app|features|state|api)(\/|$)/.test(target)
      ) {
        violations.push({ rule: 'frontend-ui-outward-import', source, target });
      }
      if (source.startsWith('src/features/') && target.startsWith('src/app/')) {
        violations.push({ rule: 'frontend-feature-app-import', source, target });
      }
      if (source.startsWith('src/features/') && target.startsWith('src/features/')) {
        const sourceFeature = source.split('/')[2];
        const targetFeature = target.split('/')[2];
        if (sourceFeature !== targetFeature) {
          violations.push({ rule: 'frontend-feature-sibling-import', source, target });
        }
      }
    }
  }

  return violations;
}

function main(): void {
  const startedAt = performance.now();
  const config = JSON.parse(readFileSync(configPath, 'utf8')) as ArchitectureConfig;
  if (config.version !== 1) throw new Error(`unsupported architecture config version ${config.version}`);

  const exceptionKeys = new Set<string>();
  for (const exception of config.exceptions) {
    if (!exception.reason.trim()) throw new Error(`exception has no reason: ${violationKey(exception)}`);
    const key = violationKey(exception);
    if (exceptionKeys.has(key)) throw new Error(`duplicate exception: ${key}`);
    exceptionKeys.add(key);
  }

  const violations = [...rustViolations(config), ...frontendViolations()];
  const violationKeys = new Set(violations.map(violationKey));
  const unapproved = violations.filter((violation) => !exceptionKeys.has(violationKey(violation)));
  const stale = config.exceptions.filter((exception) => !violationKeys.has(violationKey(exception)));

  for (const violation of violations) {
    log('DEBUG', `${exceptionKeys.has(violationKey(violation)) ? 'allowed legacy' : 'violation'} ${violationKey(violation)}`);
  }
  for (const violation of unapproved) log('ERROR', `unapproved ${violationKey(violation)}`);
  for (const exception of stale) log('ERROR', `stale exception ${violationKey(exception)}`);

  const durationMs = Math.round(performance.now() - startedAt);
  if (unapproved.length || stale.length) {
    log(
      'ERROR',
      `failed violations=${unapproved.length} stale_exceptions=${stale.length} duration_ms=${durationMs}`,
    );
    process.exitCode = 1;
    return;
  }
  log(
    'INFO',
    `passed workspace_crates=${Object.keys(config.rustWorkspaceDependencies).length} legacy_exceptions=${config.exceptions.length} duration_ms=${durationMs}`,
  );
}

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  log('ERROR', message);
  process.exitCode = 1;
}
