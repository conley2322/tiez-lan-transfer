import type { E2EConfig } from 'e2e';
import { web } from '@e2e-dev/web';

// The runner starts the preview harness on a free port, waits for it, and
// stops it afterwards. Locally an already-running harness is not reused on
// purpose: each run gets a fresh server with the current code.
export default {
  targets: [
    {
      name: 'clipboard-app',
      engine: web(),
      app: {
        url: 'http://127.0.0.1:0/',
        command: {
          executable: 'node',
          args: ['../node_modules/vite/bin/vite.js', '--config', 'harness/vite.config.mjs', '--port', '{port}'],
          log: '.e2e/logs/harness.log',
        },
      },
    },
  ],
} satisfies E2EConfig;
