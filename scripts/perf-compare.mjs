#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const schema = "runx.perf_compare.v1";
const throughputSchema = "runx.oss_runtime_throughput.v1";
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

try {
  const options = parseArgs(process.argv.slice(2));
  const report = compare(options);
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.status !== "passed") {
    process.exitCode = 1;
  }
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}

function compare(options) {
  const threshold = options.threshold ?? 0.05;
  const baseline = options.baseline ?? "phase0";
  const current = options.current ?? "current";
  const mode = fileExists(baseline) && fileExists(current) ? "runtime_report" : "criterion";
  const comparisons = mode === "runtime_report"
    ? compareRuntimeReports(baseline, current, threshold, options.workloads)
    : compareCriterionBaselines(baseline, current, threshold, options);
  const failed = comparisons.filter((comparison) => comparison.status === "failed");
  return {
    schema,
    status: failed.length === 0 ? "passed" : "failed",
    mode,
    baseline,
    current,
    threshold,
    comparisons,
    failures: failed.map((comparison) => comparison.workload),
  };
}

function compareRuntimeReports(baselinePath, currentPath, threshold, requestedWorkloads) {
  const baseline = readJson(path.resolve(repoRoot, baselinePath));
  const current = readJson(path.resolve(repoRoot, currentPath));
  assertRuntimeReport(baseline, "baseline");
  assertRuntimeReport(current, "current");
  const workloads = requestedWorkloads ?? Object.keys(baseline.workloads).sort();
  return workloads.map((workload) => compareMetric(
    workload,
    baseline.workloads[workload]?.mean_ns,
    current.workloads[workload]?.mean_ns,
    threshold,
  ));
}

function compareCriterionBaselines(baselineName, currentName, threshold, options) {
  const criterionRoot = resolveCriterionRoot(options.criterionRoot);
  const baseline = criterionEstimates(criterionRoot, baselineName);
  const current = criterionEstimates(criterionRoot, currentName);
  const workloads = options.workloads
    ?? [...new Set([...Object.keys(baseline), ...Object.keys(current)])].sort();
  if (workloads.length === 0) {
    throw new Error(
      `no criterion estimates found for '${baselineName}' and '${currentName}' under ${criterionRoot}`,
    );
  }
  return workloads.map((workload) => compareMetric(
    workload,
    baseline[workload]?.mean_ns,
    current[workload]?.mean_ns,
    threshold,
  ));
}

function compareMetric(workload, baselineMeanNs, currentMeanNs, threshold) {
  if (!isPositiveFinite(baselineMeanNs) || !isPositiveFinite(currentMeanNs)) {
    return {
      workload,
      status: "failed",
      reason: "missing baseline or current mean_ns",
      baseline_mean_ns: baselineMeanNs,
      current_mean_ns: currentMeanNs,
    };
  }
  const ratio = currentMeanNs / baselineMeanNs;
  return {
    workload,
    status: ratio <= 1 + threshold ? "passed" : "failed",
    baseline_mean_ns: baselineMeanNs,
    current_mean_ns: currentMeanNs,
    mean_regression_ratio: ratio,
    max_mean_regression_ratio: 1 + threshold,
  };
}

function resolveCriterionRoot(explicitRoot) {
  const candidates = explicitRoot
    ? [path.resolve(repoRoot, explicitRoot)]
    : [
        path.join(repoRoot, "crates", "target", "runx-perf", "criterion"),
        path.join(repoRoot, "crates", "target", "criterion"),
        path.join(repoRoot, "target", "criterion"),
      ];
  const root = candidates.find((candidate) => existsSync(candidate));
  if (!root) {
    throw new Error(`criterion root not found; checked ${candidates.join(", ")}`);
  }
  return root;
}

function criterionEstimates(criterionRoot, baselineName) {
  const estimates = {};
  for (const estimatesPath of findEstimateFiles(criterionRoot, baselineName)) {
    const workload = workloadFromCriterionEstimatePath(criterionRoot, estimatesPath, baselineName);
    const payload = readJson(estimatesPath);
    const meanNs = payload?.mean?.point_estimate;
    if (isPositiveFinite(meanNs)) {
      estimates[workload] = { mean_ns: meanNs };
    }
  }
  return estimates;
}

function findEstimateFiles(directory, baselineName) {
  const entries = safeReadDir(directory);
  const files = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...findEstimateFiles(entryPath, baselineName));
    } else if (
      entry.name === "estimates.json"
      && entryPath.endsWith(`${path.sep}${baselineName}${path.sep}estimates.json`)
    ) {
      files.push(entryPath);
    }
  }
  return files;
}

function workloadFromCriterionEstimatePath(criterionRoot, estimatesPath, baselineName) {
  const segments = path.relative(criterionRoot, estimatesPath).split(path.sep);
  const baselineIndex = segments.lastIndexOf(baselineName);
  const workloadSegments = baselineIndex > 0 ? segments.slice(0, baselineIndex) : segments.slice(0, -2);
  return workloadSegments.join("/");
}

function parseArgs(argv) {
  const options = {};
  const positional = [];
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--threshold") {
      options.threshold = Number(requiredValue(argv, ++index, arg));
    } else if (arg === "--criterion-root") {
      options.criterionRoot = requiredValue(argv, ++index, arg);
    } else if (arg === "--workloads") {
      options.workloads = requiredValue(argv, ++index, arg).split(",").filter(Boolean);
    } else if (arg === "--baseline") {
      options.baseline = requiredValue(argv, ++index, arg);
    } else if (arg === "--current" || arg === "--candidate") {
      options.current = requiredValue(argv, ++index, arg);
    } else if (arg === "--help" || arg === "-h") {
      throw new Error("usage: node scripts/perf-compare.mjs [baseline current] [--threshold 0.05] [--criterion-root path] [--workloads a,b]");
    } else if (arg.startsWith("--")) {
      throw new Error(`unknown argument '${arg}'`);
    } else {
      positional.push(arg);
    }
  }
  if (positional.length > 2) {
    throw new Error("perf-compare accepts at most two positional arguments: baseline current");
  }
  if (positional[0]) {
    options.baseline = positional[0];
  }
  if (positional[1]) {
    options.current = positional[1];
  }
  if (!Number.isFinite(options.threshold ?? 0.05) || (options.threshold ?? 0.05) < 0) {
    throw new Error("--threshold must be a non-negative number");
  }
  return options;
}

function assertRuntimeReport(report, label) {
  if (!report || report.schema !== throughputSchema || typeof report.workloads !== "object") {
    throw new Error(`${label} must use ${throughputSchema}`);
  }
}

function fileExists(candidate) {
  return existsSync(path.resolve(repoRoot, candidate));
}

function readJson(filePath) {
  return JSON.parse(readFileSync(filePath, "utf8"));
}

function safeReadDir(directory) {
  try {
    return readdirSync(directory, { withFileTypes: true });
  } catch {
    return [];
  }
}

function requiredValue(argv, index, flag) {
  const value = argv[index];
  if (!value || value.startsWith("--")) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function isPositiveFinite(value) {
  return typeof value === "number" && Number.isFinite(value) && value > 0;
}
