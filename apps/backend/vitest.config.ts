import path from "path";
import { defineConfig } from "vitest/config";

export default defineConfig({
    resolve: {
        alias: {
            "@@activation": path.resolve(__dirname, "./src/domains/activation"),
            "@@admin": path.resolve(__dirname, "./src/domains/admin"),
            "@@adminActivity": path.resolve(__dirname, "./src/domains/adminActivity"),
            "@@credit": path.resolve(__dirname, "./src/domains/credit"),
            "@@device": path.resolve(__dirname, "./src/domains/device"),
            "@@diagnostics": path.resolve(__dirname, "./src/domains/diagnostics"),
            "@@feedback": path.resolve(__dirname, "./src/domains/feedback"),
            "@@license": path.resolve(__dirname, "./src/domains/license"),
            "@@organization": path.resolve(__dirname, "./src/domains/organization"),
            "@@otp": path.resolve(__dirname, "./src/domains/otp"),
            "@@payment": path.resolve(__dirname, "./src/domains/payment"),
            "@@plan": path.resolve(__dirname, "./src/domains/plan"),
            "@@release": path.resolve(__dirname, "./src/domains/release"),
            "@@subscription": path.resolve(__dirname, "./src/domains/subscription"),
            "@@telemetry": path.resolve(__dirname, "./src/domains/telemetry"),
            "@@user": path.resolve(__dirname, "./src/domains/user"),
            "@@shared/logger": path.resolve(__dirname, "./src/shared/logger"),
            "@@shared": path.resolve(__dirname, "./src/shared"),
            "@@config": path.resolve(__dirname, "./src/config"),
            "@": path.resolve(__dirname, "./src/apps"),
            "~": path.resolve(__dirname, "./src"),
            "@@tests": path.resolve(__dirname, "./tests"),
        },
    },
    test: {
        globals: true,
        environment: "node",
        setupFiles: ["tests/setup/setup.ts"],
        include: ["src/**/__tests__/**/*.test.ts", "tests/**/*.test.ts"],
        fileParallelism: false,
        pool: "forks",
        isolate: false,
        reporters: ["default", "json", "html"],
        outputFile: {
            json: "./test-result/test-results.json",
            html: "./test-result/test-report.html",
        },
        hookTimeout: 20000,
        testTimeout: 20000,

        env: {
            NODE_ENV: "DEV",
            TEST_DISABLE_RATE_LIMIT: "true",
        },
    },
});
