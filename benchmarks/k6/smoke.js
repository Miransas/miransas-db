/**
 * miransas-db — k6 smoke test
 *
 * Verifies that the key API paths respond correctly under minimal load.
 * Run:  k6 run benchmarks/k6/smoke.js
 *
 * Env vars (override via -e):
 *   BASE_URL        default http://localhost:3001
 *   ADMIN_PASSWORD  default miransas-admin-2024
 */
import http from "k6/http";
import { check, sleep } from "k6";

const BASE_URL = __ENV.BASE_URL || "http://localhost:3001";
const ADMIN_PASSWORD = __ENV.ADMIN_PASSWORD || "miransas-admin-2024";

export const options = {
  vus: 1,
  iterations: 1,
  thresholds: {
    http_req_failed: ["rate<0.01"],
    http_req_duration: ["p(95)<500"],
  },
};

export function setup() {
  const res = http.post(
    `${BASE_URL}/auth/login`,
    JSON.stringify({ password: ADMIN_PASSWORD }),
    { headers: { "Content-Type": "application/json" } }
  );
  check(res, { "login 200": (r) => r.status === 200 });
  return { token: res.json("token") };
}

export default function (data) {
  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${data.token}`,
  };

  // Health (no auth)
  check(http.get(`${BASE_URL}/health`), {
    "health 200": (r) => r.status === 200,
    "health ok":  (r) => r.json("status") === "ok",
  });

  // Protected routes
  check(http.get(`${BASE_URL}/api/projects`, { headers }), {
    "projects 200": (r) => r.status === 200,
  });

  check(http.get(`${BASE_URL}/api/databases`, { headers }), {
    "databases 200": (r) => r.status === 200,
  });

  check(http.get(`${BASE_URL}/api/admin/summary`, { headers }), {
    "summary 200": (r) => r.status === 200,
  });

  sleep(0.1);
}
