import pino from "pino";

const isDev = process.env.NODE_ENV === "DEV";

export const logger = pino({
    level: isDev ? "debug" : "info",
    redact: {
        paths: [
            "password",
            "*.password",
            "token",
            "*.token",
            "refreshToken",
            "*.refreshToken",
            "otp",
            "*.otp",
            "code",
            "*.code",
            "secret",
            "*.secret",
            "authorization",
            "req.headers.authorization",
            "req.headers.cookie",
        ],
        censor: "[redacted]",
    },
    transport: isDev
        ? {
              target: "pino-pretty",
              options: { colorize: true, translateTime: "HH:MM:ss", ignore: "pid,hostname" },
          }
        : undefined,
});

export default logger;
