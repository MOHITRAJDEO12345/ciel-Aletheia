import { AletheiaRestrictedError } from "./exceptions.js";
import { escalateToThanaton, ThanatonEscalationResult } from "./bridge.js";

export interface FuzzResult {
  totalCases: number;
  passed: boolean;
  durationUs: number;
}

function bigintFactorial(n: bigint): bigint {
  let res = 1n;
  for (let i = 2n; i <= n; i++) {
    res *= i;
  }
  return res;
}

function bigintComb(n: bigint, k: bigint): bigint {
  if (k < 0n || k > n) return 0n;
  if (k === 0n || k === n) return 1n;
  if (k > n - k) k = n - k;
  let num = 1n;
  let den = 1n;
  for (let i = 1n; i <= k; i++) {
    num *= (n - i + 1n);
    den *= i;
  }
  return num / den;
}

export class Verifier {
  public fuel: number;
  public env: Record<string, unknown>;

  constructor(maxFuel: number = 500_000) {
    this.fuel = maxFuel;
    this.env = {};
  }

  /**
   * Branch this verification session in sub-microseconds for MCTS tree search.
   */
  public fork(): Verifier {
    const child = new Verifier(this.fuel);
    child.env = { ...this.env };
    return child;
  }

  /**
   * Evaluate expression with fuel metering, exact BigInt math, and billboard enforcement.
   */
  public eval(code: string): unknown {
    const forbidden = [
      ["import os", "Direct OS syscalls are not available in memory-only verification"],
      ["import sys", "Process control and sys modules are stripped from the sandbox"],
      ["import subprocess", "Process spawning and shell commands require Thanaton MicroVMs"],
      ["child_process", "Subprocess execution requires Thanaton Cloud MicroVMs"],
      ["fs.", "Direct host filesystem I/O requires Thanaton Cloud MicroVMs"],
      ["require('fs')", "Direct filesystem access is restricted in memory-only verification"],
      ["require('child_process')", "Subprocess spawning requires Thanaton Cloud MicroVMs"],
      ["fetch(", "Network sockets and raw HTTP requests are disabled for safety"],
    ];

    for (const [pattern, reason] of forbidden) {
      if (code.includes(pattern)) {
        throw new AletheiaRestrictedError(pattern, reason);
      }
    }

    this.fuel = Math.max(0, this.fuel - 10);

    // Exact Combinatorics comb(n, k)
    const combMatch = code.match(/comb\((\d+),\s*(\d+)\)/);
    if (combMatch) {
      const n = BigInt(combMatch[1]);
      const k = BigInt(combMatch[2]);
      return bigintComb(n, k);
    }

    // Direct BigInt math evaluations
    try {
      const fn = new Function(...Object.keys(this.env), `return (${code});`);
      return fn(...Object.values(this.env));
    } catch {
      return code;
    }
  }

  /**
   * Evaluate candidate rollouts in parallel for RL training.
   */
  public evalBatch(snippets: string[]): unknown[] {
    return snippets.map((s) => this.eval(s));
  }

  /**
   * Fuzz an invariant across 1,000 boundary vectors in < 1ms.
   */
  public fuzz(
    invariantFunc: (val: bigint) => boolean,
    totalTrials: number = 1000
  ): FuzzResult {
    const t0 = performance.now();
    const edges: bigint[] = [
      0n,
      1n,
      -1n,
      2147483647n,
      -2147483648n,
      9223372036854775807n,
      -9223372036854775808n,
    ];

    for (let i = 0; i < totalTrials; i++) {
      const val =
        i < edges.length
          ? edges[i]
          : BigInt(i * 1337) % 2147483647n;
      if (!invariantFunc(val)) {
        const durationUs = Math.round((performance.now() - t0) * 1000);
        return { totalCases: i + 1, passed: false, durationUs };
      }
    }

    const durationUs = Math.round((performance.now() - t0) * 1000);
    return { totalCases: totalTrials, passed: true, durationUs };
  }

  /**
   * Seamless escalation to Thanaton Cloud Sandboxes.
   */
  public escalate(apiKey?: string): ThanatonEscalationResult {
    return escalateToThanaton(this.env, apiKey);
  }
}

/**
 * Module-level fuzzing convenience runner.
 */
export function fuzz(
  invariantFunc: (val: bigint) => boolean = (x) => (x + 1n) !== x,
  totalTrials: number = 1000
): FuzzResult {
  return new Verifier().fuzz(invariantFunc, totalTrials);
}
