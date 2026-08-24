import { invoke } from "@tauri-apps/api/core";

export type TestEntry = {
  path: string;
  name: string;
};

export type Catalog = {
  actions: Array<{
    definition: {
      manifest_path: string;
      name: string;
    };
    supported: boolean;
    warning: string | null;
  }>;
  tests: TestEntry[];
};

export type SessionPlan = {
  session_id: string;
  workflow_path: string;
  cleanup_identity: {
    remote: string;
    ref_name: string;
    session_id: string;
  };
  git_plan: {
    summary: string;
    preconditions: string[];
  };
};

export type TestResult = {
  session_id: string;
  run_id: number;
  run_url: string;
  conclusion: string;
  passed: boolean;
  manifest_path: string | null;
  logs_path: string;
};

export type StartResponse = {
  plan: SessionPlan;
  result: TestResult | null;
};

export type WatchResponse = {
  pending: boolean;
  result: TestResult | null;
};

export const listActionTests = (repoRoot: string) =>
  invoke<Catalog>("list_action_tests", { repoRoot });

export const startActionTest = (
  repoRoot: string,
  testName: string,
  confirmed: boolean,
) =>
  invoke<StartResponse>("start_action_test", {
    repoRoot,
    testName,
    confirmed,
  });

export const watchActionTest = (repoRoot: string, sessionId: string) =>
  invoke<WatchResponse>("watch_action_test", { repoRoot, sessionId });

export const getActionTestResult = (
  repoRoot: string,
  sessionId: string,
) =>
  invoke<TestResult | null>("get_action_test_result", {
    repoRoot,
    sessionId,
  });
