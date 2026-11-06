// SPDX-License-Identifier: AGPL-3.0-only
import { createRequire } from 'node:module';

/** Platforms with a bundled engine, as `${process.platform}-${process.arch}`. */
export const BUNDLED_PLATFORMS = ['win32-x64', 'linux-x64', 'darwin-arm64', 'darwin-x64'] as const;

/**
 * The `umwelt` executable to run: `UMWELT_BINARY` when set, else the engine
 * bundled for this platform (the optional package
 * `@kallamo/umwelt-<platform>-<arch>`), else `umwelt` from the PATH.
 */
export function resolveEngineBinary(
  environment: NodeJS.ProcessEnv = process.env,
  platform: string = process.platform,
  arch: string = process.arch,
): string {
  const override = environment.UMWELT_BINARY;
  if (override) return override;
  const file = platform === 'win32' ? 'umwelt.exe' : 'umwelt';
  try {
    return createRequire(import.meta.url).resolve(`@kallamo/umwelt-${platform}-${arch}/bin/${file}`);
  } catch {
    return 'umwelt';
  }
}
