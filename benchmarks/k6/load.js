/**
 * miransas-db — k6 load test
 *
 * Ramps up to 50 VUs, holds for 2 minutes, then ramps down.
 * Run:  k6 run benchmarks/k6/load.js
 *
 * Env vars:
 *   BASE_URL        default http://localhost:3001
 *   ADMIN_PASSWORD  default miransas-admin-2024
 */
import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";

const BASE_URL = __ENV.BASE_URL || "http://localhost:3001";
const ADMIN_PASSWORD = __ENV.ADMIN_PASSWORD || "miransas-admin-2024";

const errorRate = new Rate("errors");
const loginDuration = new Trend("login_duration", true);

export const options = {
  stages: [
    { duration: "30s", target: 10 },   // warm-up
    { duration: "1m",  target: 50 },   // ramp to peak
    { duration: "2m",  target: 50 },   // hold
    { duration: "30s", target: 0  },   // ramp down
  ],
  thresholds: {
    http_req_failed:   ["rate<0.01"],          // <1 % errors
    http_req_duration: ["p(95)<800", "p(99)<2000"],
    errors:            ["rate<0.01"],
  },
};

export function setup() {
  const start = Date.now();
  const res = http.post(
    `${BASE_URL}/auth/login`,
    JSON.stringify({ password: ADMIN_PASSWORD }),
    { headers: { "Content-Type": "application/json" } }
  );
  loginDuration.add(Date.now() - start);
  check(res, { "setup login 200": (r) => r.status === 200 });
  return { token: res.json("token") };
}

export default function (data) {
  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${data.token}`,
  };

  const scenarios = [
    () => http.get(`${BASE_URL}/health`),
    () => http.get(`${BASE_URL}/api/projects`,        { headers }),
    () => http.get(`${BASE_URL}/api/databases`,       { headers }),
    () => http.get(`${BASE_URL}/api/secrets`,         { headers }),
    () => http.get(`${BASE_URL}/api/admin/summary`,   { headers }),
  ];

  const res = scenarios[Math.floor(Math.random() * scenarios.length)]();

  const ok = check(res, {
    "status 2xx": (r) => r.status >= 200 && r.status < 300,
  });
  errorRate.add(!ok);

  sleep(Math.random() * 0.5 + 0.1); // 100–600 ms think time
}
