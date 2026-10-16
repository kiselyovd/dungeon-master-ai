import { existsSync } from 'node:fs';

const [script, ...args] = Bun.argv.slice(2);
if (!script) {
  console.error('usage: bun scripts/run-bash.ts <script> [args...]');
  process.exit(2);
}

const windowsCandidates = [
  process.env.GIT_BASH_PATH,
  'C:\\Program Files\\Git\\bin\\bash.exe',
  'C:\\Program Files\\Git\\usr\\bin\\bash.exe',
].filter((candidate): candidate is string => Boolean(candidate));

const bash =
  process.platform === 'win32'
    ? windowsCandidates.find((candidate) => existsSync(candidate))
    : 'bash';

if (!bash) {
  console.error('Git Bash was not found; install Git for Windows or set GIT_BASH_PATH');
  process.exit(127);
}

const result = Bun.spawnSync([bash, script, ...args], {
  cwd: process.cwd(),
  stderr: 'inherit',
  stdin: 'inherit',
  stdout: 'inherit',
});
process.exit(result.exitCode);
